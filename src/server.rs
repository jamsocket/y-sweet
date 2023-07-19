use crate::{doc_service::DocService, stores::Store};
use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    headers::{self, authorization::Bearer},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router, TypedHeader,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap, convert::Infallible, future::ready, net::SocketAddr, sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use y_sync::net::BroadcastGroup;

pub struct Server {
    docs: DashMap<String, DocService>,
    pub store: Arc<Box<dyn Store>>,
    pub checkpoint_freq: Duration,
    pub bearer_token: Option<String>,
}

impl Server {
    pub async fn new(
        store: Box<dyn Store>,
        checkpoint_freq: Duration,
        bearer_token: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            docs: DashMap::new(),
            store: Arc::new(store),
            checkpoint_freq,
            bearer_token,
        })
    }

    pub fn check_auth(
        &self,
        header: Option<TypedHeader<headers::Authorization<Bearer>>>,
    ) -> Result<(), StatusCode> {
        if let Some(token) = &self.bearer_token {
            if let Some(TypedHeader(headers::Authorization(bearer))) = header {
                if bearer.token() == token {
                    return Ok(());
                }
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(())
    }

    pub async fn create_doc(&self) -> String {
        let doc_id = nanoid::nanoid!();
        let doc_service = DocService::new(self.store.clone(), doc_id.clone(), self.checkpoint_freq)
            .await
            .unwrap(); // todo: handle error
        self.docs.insert(doc_id.clone(), doc_service);

        tracing::info!(doc_id=?doc_id, "Created doc");

        doc_id
    }

    pub async fn serve(self, addr: &SocketAddr) -> Result<()> {
        let server_state = Arc::new(self);

        let trace_layer = TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_request(DefaultOnRequest::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO));

        let app = Router::new()
            .route("/doc/ws/:doc_id", get(handler))
            .route("/doc/new", post(new_doc))
            .route("/doc/:doc_id/auth", post(auth_doc))
            .with_state(server_state)
            .layer(trace_layer);

        axum::Server::try_bind(addr)?
            .serve(app.into_make_service())
            .await
            .map_err(|_| anyhow!("Failed to serve"))?;

        Ok(())
    }
}

async fn handler(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, StatusCode> {
    let Some(doc_service) = server_state.docs.get(&doc_id) else {
        return Err(StatusCode::NOT_FOUND);
    };

    let broadcast_group = doc_service.broadcast_group.clone();
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, broadcast_group)))
}

async fn handle_socket(socket: WebSocket, broadcast_group: Arc<BroadcastGroup>) {
    let (sink, stream) = socket.split();

    let stream = tokio_stream::StreamExt::filter_map(stream, |d| match d {
        Ok(Message::Binary(s)) => Some(Ok::<_, Infallible>(s)),
        Ok(Message::Close(_)) => None,
        msg => {
            tracing::warn!(?msg, "Received non-binary message");
            None
        }
    });

    let sink = sink.with(|d| ready(Ok::<_, axum::Error>(Message::Binary(d))));
    let sink = Arc::new(Mutex::new(sink));
    let sub = broadcast_group.subscribe(sink, stream);

    match sub.completed().await {
        Ok(_) => tracing::info!("Socket closed"),
        Err(e) => tracing::warn!(?e, "Socket closed with error"),
    }
}

#[derive(Serialize)]
struct DocResponse {
    doc_id: String,
}

async fn new_doc(
    authorization: Option<TypedHeader<headers::Authorization<Bearer>>>,
    State(server_state): State<Arc<Server>>,
) -> Result<Json<DocResponse>, StatusCode> {
    server_state.check_auth(authorization)?;

    let doc_id = server_state.create_doc().await;
    Ok(Json(DocResponse { doc_id }))
}

#[derive(Deserialize)]
pub enum Authorization {
    #[serde(rename = "none")]
    Nothing,
    #[serde(rename = "readonly")]
    ReadOnly,
    #[serde(rename = "full")]
    Full,
}

impl Authorization {
    fn full() -> Self {
        Self::Full
    }
}

#[derive(Deserialize)]
struct AuthDocRequest {
    #[serde(default = "Authorization::full")]
    authorization: Authorization,
    user_id: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, Value>,
}

#[derive(Serialize)]
struct AuthDocResponse {
    base_url: String,
    doc_id: String,
}

async fn auth_doc(
    authorization: Option<TypedHeader<headers::Authorization<Bearer>>>,
    TypedHeader(host): TypedHeader<headers::Host>,
    State(server_state): State<Arc<Server>>,
    Path(doc_id): Path<String>,
    Json(body): Json<AuthDocRequest>,
) -> Result<Json<AuthDocResponse>, StatusCode> {
    server_state.check_auth(authorization)?;

    if !server_state.docs.contains_key(&doc_id) {
        if server_state
            .store
            .exists(&format!("{}/data.bin", doc_id)) // TODO: this should live elsewhere?
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            let doc_service = DocService::new(
                server_state.store.clone(),
                doc_id.clone(),
                server_state.checkpoint_freq,
            )
            .await
            .unwrap(); // todo: handle error
            server_state.docs.insert(doc_id.clone(), doc_service);

            tracing::info!(doc_id=?doc_id, "Loaded doc from snapshot");
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let base_url = format!("ws://{}/doc/ws", host);
    Ok(Json(AuthDocResponse { base_url, doc_id }))
}
