use crate::doc_service::DocService;
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
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{convert::Infallible, future::ready, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use y_serve_core::{
    api_types::{AuthDocRequest, AuthDocResponse, NewDocResponse},
    auth::Authenticator,
    store::Store,
};
use y_sync::net::BroadcastGroup;

pub struct Server {
    docs: DashMap<String, DocService>,
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
