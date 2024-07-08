use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use axum_extra::typed_header::TypedHeader;
use dashmap::{mapref::one::MappedRef, DashMap};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{net::TcpListener, sync::mpsc::channel};
use tracing::{span, Instrument, Level};
use url::Url;
use y_sweet_core::{
    api_types::{
        validate_doc_name, AuthDocRequest, ClientToken, DocCreationRequest, NewDocResponse,
    },
    auth::Authenticator,
    doc_connection::DocConnection,
    doc_sync::DocWithSyncKv,
    store::Store,
    sync::awareness::Awareness,
};

fn current_time_epoch_millis() -> u64 {
    let now = std::time::SystemTime::now();
    let duration_since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    duration_since_epoch.as_millis() as u64
}

pub struct Server {
    docs: DashMap<String, DocWithSyncKv>,
    store: Option<Arc<Box<dyn Store>>>,
    checkpoint_freq: Duration,
    authenticator: Option<Authenticator>,
    url_prefix: Option<Url>,
}

impl Server {
    pub async fn new(
        store: Option<Box<dyn Store>>,
        checkpoint_freq: Duration,
        authenticator: Option<Authenticator>,
        url_prefix: Option<Url>,
    ) -> Result<Self> {
        Ok(Self {
            docs: DashMap::new(),
            store: store.map(Arc::new),
            checkpoint_freq,
            authenticator,
            url_prefix,
        })
    }

    pub async fn doc_exists(&self, doc_id: &str) -> bool {
        if self.docs.contains_key(doc_id) {
            return true;
        }
        if let Some(store) = &self.store {
            store
                .exists(&format!("{}/data.ysweet", doc_id))
                .await
                .unwrap_or_default()
        } else {
            false
        }
    }

    pub async fn create_doc(&self) -> Result<String> {
        let doc_id = nanoid::nanoid!();
        self.load_doc(&doc_id).await?;
        tracing::info!(doc_id=?doc_id, "Created doc");
        Ok(doc_id)
    }

    pub async fn load_doc(&self, doc_id: &str) -> Result<()> {
        let (send, mut recv) = channel(1024);

        let dwskv = DocWithSyncKv::new(doc_id, self.store.clone(), move || {
            send.try_send(()).unwrap();
        })
        .await?;

        dwskv
            .sync_kv()
            .persist()
            .await
            .map_err(|e| anyhow!("Error persisting: {:?}", e))?;

        {
            let sync_kv = dwskv.sync_kv();
            let checkpoint_freq = self.checkpoint_freq;
            let doc_id = doc_id.to_string();
            tokio::spawn(
                async move {
                    // TODO: expedite save on shutdown.
                    let mut last_save = std::time::Instant::now();

                    while let Some(()) = recv.recv().await {
                        tracing::info!("Received dirty signal.");
                        let now = std::time::Instant::now();
                        if now - last_save < checkpoint_freq {
                            let timeout = checkpoint_freq - (now - last_save);
                            tracing::info!(?timeout, "Throttling.");
                            tokio::time::sleep(timeout).await;
                            tracing::info!("Done throttling.");
                        }

                        tracing::info!("Persisting.");
                        sync_kv.persist().await.unwrap();
                        last_save = std::time::Instant::now();
                        tracing::info!("Done persisting.");
                    }

                    tracing::info!("Terminating loop.");
                }
                .instrument(span!(Level::INFO, "save_loop", doc_id=?doc_id)),
            );
        }

        self.docs.insert(doc_id.to_string(), dwskv);
        Ok(())
    }

    pub async fn get_or_create_doc(
        &self,
        doc_id: &str,
    ) -> Result<MappedRef<String, DocWithSyncKv, DocWithSyncKv>> {
        if !self.docs.contains_key(doc_id) {
            tracing::info!(doc_id=?doc_id, "Loading doc");
            self.load_doc(doc_id).await?;
        }

        Ok(self
            .docs
            .get(doc_id)
            .expect("Doc should exist, we just created it.")
            .map(|d| d))
    }

    pub fn check_auth(
        &self,
        header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    ) -> Result<(), StatusCode> {
        if let Some(auth) = &self.authenticator {
            if let Some(TypedHeader(headers::Authorization(bearer))) = header {
                if let Ok(()) =
                    auth.verify_server_token(bearer.token(), current_time_epoch_millis())
                {
                    return Ok(());
                }
            }
            Err(StatusCode::UNAUTHORIZED)
        } else {
            Ok(())
        }
    }

    pub fn routes(self) -> Router {
        Router::new()
            .route("/check_store", get(check_store))
            .route("/doc/ws/:doc_id", get(handle_socket_upgrade))
            .route("/doc/new", post(new_doc))
            .route("/doc/:doc_id/auth", post(auth_doc))
            .with_state(Arc::new(self))
    }

    pub async fn serve(self, addr: &SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;

        let app = self.routes();

        axum::serve(listener, app.into_make_service())
            .await
            .map_err(|_| anyhow!("Failed to serve"))?;

        Ok(())
    }
}

#[derive(Deserialize)]
struct HandlerParams {
    token: Option<String>,
}

async fn handle_socket_upgrade(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    Query(params): Query<HandlerParams>,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, StatusCode> {
    // TODO: clean this up.
    if let Some(authenticator) = &server_state.authenticator {
        if let Some(token) = params.token {
            authenticator
                .verify_doc_token(&token, &doc_id, current_time_epoch_millis())
                .map_err(|_| StatusCode::FORBIDDEN)?;
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    let dwskv = server_state.get_or_create_doc(&doc_id).await.unwrap();
    let awareness = dwskv.awareness();

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, awareness)))
}

async fn handle_socket(socket: WebSocket, awareness: Arc<RwLock<Awareness>>) {
    let (mut sink, mut stream) = socket.split();
    let (send, mut recv) = channel(1024);

    tokio::spawn(async move {
        while let Some(msg) = recv.recv().await {
            let _ = sink.send(Message::Binary(msg)).await;
        }
    });

    let connection = DocConnection::new(awareness.clone(), move |bytes| {
        if let Err(e) = send.try_send(bytes.to_vec()) {
            tracing::warn!(?e, "Error sending message");
        }
    });

    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Ok(Message::Binary(bytes)) => bytes,
            Ok(Message::Close(_)) => break,
            Err(_e) => {
                // The stream will complain about things like
                // connections being lost without handshake.
                continue;
            }
            msg => {
                tracing::warn!(?msg, "Received non-binary message");
                continue;
            }
        };

        if let Err(e) = connection.send(&msg).await {
            tracing::warn!(?e, "Error handling message");
        }
    }
}

async fn check_store(
    authorization: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    State(server_state): State<Arc<Server>>,
) -> Result<Json<Value>, StatusCode> {
    server_state.check_auth(authorization)?;

    if server_state.store.is_none() {
        return Ok(Json(json!({"ok": false, "error": "No store set."})));
    };

    // The check_store endpoint for the native server is kind of moot, since
    // the server will not start if store is not ok.
    Ok(Json(json!({"ok": true})))
}

async fn new_doc(
    authorization: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    State(server_state): State<Arc<Server>>,
    Json(body): Json<DocCreationRequest>,
) -> Result<Json<NewDocResponse>, StatusCode> {
    server_state.check_auth(authorization)?;

    let doc_id = if let Some(doc_id) = body.doc_id {
        if !validate_doc_name(doc_id.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }

        server_state
            .get_or_create_doc(doc_id.as_str())
            .await
            .map_err(|e| {
                tracing::error!(?e, "Failed to create doc");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        doc_id
    } else {
        server_state.create_doc().await.map_err(|d| {
            tracing::error!(?d, "Failed to create doc");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    Ok(Json(NewDocResponse { doc_id }))
}

async fn auth_doc(
    authorization: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    TypedHeader(host): TypedHeader<headers::Host>,
    State(server_state): State<Arc<Server>>,
    Path(doc_id): Path<String>,
    Json(_body): Json<AuthDocRequest>,
) -> Result<Json<ClientToken>, StatusCode> {
    server_state.check_auth(authorization)?;

    if !server_state.doc_exists(&doc_id).await {
        return Err(StatusCode::NOT_FOUND);
    }

    let token = if let Some(auth) = &server_state.authenticator {
        let token = auth.gen_doc_token(&doc_id, current_time_epoch_millis());
        Some(token)
    } else {
        None
    };

    let url = if let Some(url_prefix) = &server_state.url_prefix {
        let mut url = url_prefix.clone();
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme).unwrap();
        url = url.join("/doc/ws").unwrap();
        url.to_string()
    } else {
        format!("ws://{host}/doc/ws")
    };

    Ok(Json(ClientToken { url, doc_id, token }))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use y_sweet_core::api_types::Authorization;

    #[tokio::test]
    async fn test_auth_doc() {
        let server_state = Server::new(None, Duration::from_secs(60), None, None)
            .await
            .unwrap();

        let doc_id = server_state.create_doc().await.unwrap();

        let token = auth_doc(
            None,
            TypedHeader(headers::Host::from(http::uri::Authority::from_static(
                "localhost",
            ))),
            State(Arc::new(server_state)),
            Path(doc_id.clone()),
            Json(AuthDocRequest {
                authorization: Authorization::Full,
                user_id: None,
                metadata: HashMap::new(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(token.url, "ws://localhost/doc/ws");
        assert_eq!(token.doc_id, doc_id);
        assert!(token.token.is_none());
    }

    #[tokio::test]
    async fn test_auth_doc_with_prefix() {
        let prefix: Url = "https://foo.bar".parse().unwrap();
        let server_state = Server::new(None, Duration::from_secs(60), None, Some(prefix))
            .await
            .unwrap();

        let doc_id = server_state.create_doc().await.unwrap();

        let token = auth_doc(
            None,
            TypedHeader(headers::Host::from(http::uri::Authority::from_static(
                "localhost",
            ))),
            State(Arc::new(server_state)),
            Path(doc_id.clone()),
            Json(AuthDocRequest {
                authorization: Authorization::Full,
                user_id: None,
                metadata: HashMap::new(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(token.url, "wss://foo.bar/doc/ws");
        assert_eq!(token.doc_id, doc_id);
        assert!(token.token.is_none());
    }
}
