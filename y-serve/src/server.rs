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
use base64::{engine::general_purpose, Engine};
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
use y_serve_core::{
    api_types::{AuthDocRequest, AuthDocResponse, NewDocResponse},
    auth::Authenticator,
    doc_connection::DocConnection,
    doc_sync::DocWithSyncKv,
    store::Store,
};
use y_sync::awareness::Awareness;

pub struct Server {
    docs: DashMap<String, DocWithSyncKv>,
    store: Arc<Box<dyn Store>>,
    checkpoint_freq: Duration,
    bearer_token: Option<String>,
    authenticator: Option<Authenticator>,
}

impl Server {
    pub async fn new(
        store: Box<dyn Store>,
        checkpoint_freq: Duration,
        bearer_token: Option<String>,
        authenticator: Option<Authenticator>,
    ) -> Result<Self> {
        Ok(Self {
            docs: DashMap::new(),
            store: Arc::new(store),
            checkpoint_freq,
            bearer_token,
            authenticator,
        })
    }

    pub async fn doc_exists(&self, doc_id: &str) -> bool {
        self.docs.contains_key(doc_id)
            || self
                .store
                .exists(&format!("{}/data.bin", doc_id))
                .await
                .unwrap_or_default()
    }

    pub async fn create_doc(&self) -> String {
        let doc_id = nanoid::nanoid!();
        self.load_doc(&doc_id).await;
        tracing::info!(doc_id=?doc_id, "Created doc");
        doc_id
    }

    pub async fn load_doc(&self, doc_id: &str) {
        let (send, mut recv) = channel(1024);

        let dwskv = DocWithSyncKv::new(&doc_id, self.store.clone(), move || {
            send.try_send(()).unwrap();
        })
        .await
        .unwrap();

        dwskv.sync_kv().persist().await.unwrap();

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
    }

    pub async fn get_or_create_doc(
        &self,
        doc_id: &str,
    ) -> Result<MappedRef<String, DocWithSyncKv, DocWithSyncKv>> {
        if !self.docs.contains_key(doc_id) {
            tracing::info!(doc_id=?doc_id, "Loading doc");
            self.load_doc(doc_id).await;
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
        if let Some(token) = &self.bearer_token {
            if let Some(TypedHeader(headers::Authorization(bearer))) = header {
                let bytes = general_purpose::STANDARD
                    .decode(bearer.token())
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                if bytes == token.as_bytes() {
                    return Ok(());
                }
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(())
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
    if let Some(authenticator) = &server_state.authenticator {
        if let Some(token) = params.token {
            if !authenticator
                .verify_token(&token, &doc_id)
                .unwrap_or_default()
            {
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    let dwskv = server_state.get_or_create_doc(&doc_id).await.unwrap();
    let awareness = dwskv.awareness().clone();

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

    let doc_id = server_state.create_doc().await;
    Ok(Json(NewDocResponse { doc_id }))
}

async fn auth_doc(
    authorization: Option<TypedHeader<headers::Authorization<Bearer>>>,
    TypedHeader(host): TypedHeader<headers::Host>,
    State(server_state): State<Arc<Server>>,
    Path(doc_id): Path<String>,
    Json(_body): Json<AuthDocRequest>,
) -> Result<Json<AuthDocResponse>, StatusCode> {
    server_state.check_auth(authorization)?;

    if !server_state.doc_exists(&doc_id).await {
        return Err(StatusCode::NOT_FOUND);
    }

    let token = if let Some(paseto) = &server_state.authenticator {
        let token = paseto
            .gen_token(&doc_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Some(token)
    } else {
        None
    };

    let base_url = format!("ws://{}/doc/ws", host);
    Ok(Json(AuthDocResponse {
        base_url,
        doc_id,
        token,
    }))
}
