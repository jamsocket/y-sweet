use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    headers::{self, authorization::Bearer},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router, TypedHeader,
};
use dashmap::{mapref::one::MappedRef, DashMap};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::mpsc::channel;
use tracing::{span, Instrument, Level};
use url::Url;
use y_sweet_core::{
    api_types::{AuthDocRequest, ClientToken, NewDocResponse},
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
        header: Option<TypedHeader<headers::Authorization<Bearer>>>,
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

    pub async fn serve(self, addr: &SocketAddr) -> Result<()> {
        let server_state = Arc::new(self);

        let app = Router::new()
            .route("/doc/ws/:doc_id", get(handler))
            .route("/doc/new", post(new_doc))
            .route("/doc/:doc_id/auth", post(auth_doc))
            .with_state(server_state);

        axum::Server::try_bind(addr)?
            .serve(app.into_make_service())
            .await
            .map_err(|_| anyhow!("Failed to serve"))?;

        Ok(())
    }
}

#[derive(Deserialize)]
struct HandlerParams {
    token: Option<String>,
}

async fn handler(
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

async fn new_doc(
    authorization: Option<TypedHeader<headers::Authorization<Bearer>>>,
    State(server_state): State<Arc<Server>>,
) -> Result<Json<NewDocResponse>, StatusCode> {
    server_state.check_auth(authorization)?;

    let doc_id = server_state.create_doc().await.map_err(|d| {
        tracing::error!(?d, "Failed to create doc");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(NewDocResponse { doc: doc_id }))
}

async fn auth_doc(
    authorization: Option<TypedHeader<headers::Authorization<Bearer>>>,
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
        url.join("/doc/ws").unwrap().to_string();
        url.to_string()
    } else {
        format!("ws://{host}/doc/ws")
    };

    Ok(Json(ClientToken {
        url,
        doc: doc_id,
        token,
    }))
}
