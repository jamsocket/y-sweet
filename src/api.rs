use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use std::{convert::Infallible, future::ready, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use y_sync::net::BroadcastGroup;

async fn handler(
    ws: WebSocketUpgrade,
    State(broadcast_group): State<Arc<BroadcastGroup>>,
) -> Response {
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

pub async fn serve_api(addr: &SocketAddr, broadcast_group: BroadcastGroup) -> Result<()> {
    let broadcast_group = Arc::new(broadcast_group);
    let app = Router::new()
        .route("/my-room", get(handler))
        .with_state(broadcast_group);

    axum::Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .map_err(|_| anyhow!("Failed to serve"))?;

    Ok(())
}
