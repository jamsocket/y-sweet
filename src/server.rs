use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade, Path,
    },
    response::Response,
    routing::{get, post},
    Router, Json, TypedHeader, headers::{self, authorization::Bearer}
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::{convert::Infallible, future::ready, net::SocketAddr, sync::Arc, collections::HashMap, time::Duration};
use tokio::sync::Mutex;
use y_sync::net::BroadcastGroup;
use crate::{doc_service::DocService, stores::Store};
use serde::Deserialize;

pub struct Server {
    docs: DashMap<String, DocService>,
    pub store: Box<dyn Store>,
    pub checkpoint_freq: Duration,
    pub bearer_token: Option<String>,
}

impl Server {
    pub async fn new(store: Box<dyn Store>, checkpoint_freq: Duration, bearer_token: Option<String>) -> Result<Self> {
        Ok(Self {
            docs: DashMap::new(),
            store,
            checkpoint_freq: checkpoint_freq,
            bearer_token: bearer_token,
        })
    }

    pub async fn serve(self, addr: &SocketAddr) -> Result<()> {
        let server_state = Arc::new(self);
        
        let app = Router::new()
            .route("/doc/:doc_id/connect", get(handler))
            .route("/doc/new", post(new_doc))
            .route("/doc/auth", post(auth_doc))
            .with_state(server_state);
    
        axum::Server::bind(addr)
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
) -> Response {
    let Some(doc_service) = server_state.docs.get(&doc_id) else {
        panic!("Doc not found"); // TODO: return a 404
    };

    let broadcast_group = doc_service.broadcast_group.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, broadcast_group.clone()))
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

async fn new_doc(
    State(server_state): State<Arc<Server>>,
) -> Result<Response, Infallible> {
    // let doc_id = nanoid::nanoid!();
    // let doc_service = DocService::new().await?;
    // server_state.insert(doc_id.clone(), doc_service);
    // Ok(Response::new(doc_id))
    todo!()
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

#[derive(Deserialize)]
struct AuthDocRequest {
    doc_id: String,
    authorization: Authorization,
    user_id: String,
    metadata: HashMap<String, Value>,
}

async fn auth_doc(
    TypedHeader(authorization): TypedHeader<headers::Authorization<Bearer>>,
    State(server_state): State<Arc<Server>>,
    Json(body): Json<AuthDocRequest>,
) -> Result<Response, Infallible> {
    let token = authorization.token();

    todo!()
}

