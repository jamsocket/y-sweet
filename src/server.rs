use crate::{
    stores::Store,
    sync_kv::{self, SyncKv},
};
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
use std::{convert::Infallible, future::ready, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{mpsc::Receiver, Mutex, RwLock};
use y_sync::{awareness::Awareness, net::BroadcastGroup};
use yrs::{Doc, Options, Transact};
use yrs_kvstore::DocOps;

const DOC_NAME: &str = "doc";

pub struct Server<S: Store + 'static> {
    pub store: S,
    pub addr: SocketAddr,
    pub checkpoint_freq: Duration,
}

enum ServerMessage {
    Persist,
    Shutdown,
}

impl<S: Store> Server<S> {
    async fn persist_loop(sync_kv: Arc<SyncKv>, mut receiver: Receiver<ServerMessage>) {
        loop {
            match receiver.recv().await {
                Some(ServerMessage::Persist) => {
                    println!("persisting");
                    sync_kv.persist().await.unwrap();
                }
                Some(ServerMessage::Shutdown) => {
                    return;
                }
                None => {
                    println!("persist loop ended");
                }
            }
        }
    }

    pub async fn serve(self) -> Result<(), &'static str> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        let sync_kv = SyncKv::new(self.store, move || sender.try_send(ServerMessage::Persist).unwrap())
            .await
            .map_err(|_| "Failed to create SyncKv")?;

        let sync_kv = Arc::new(sync_kv);

        tokio::spawn(Self::persist_loop(sync_kv.clone(), receiver));

        let doc = Doc::new();

        {
            let mut txn = doc.transact_mut();
            sync_kv
                .load_doc(DOC_NAME, &mut txn)
                .map_err(|_| "Failed to load doc")?;
        }

        let _subscription_guard = doc
            .observe_update_v1(move |_, event| {
                sync_kv.push_update(DOC_NAME, &event.update).unwrap();
                sync_kv
                    .flush_doc_with(DOC_NAME, Options::default())
                    .unwrap();
            })
            .map_err(|_| "Failed to subscribe to updates")?;

        let awareness = Arc::new(RwLock::new(Awareness::new(doc)));
        let broadcast_group = Arc::new(BroadcastGroup::new(awareness, 32).await);

        let app = Router::new()
            .route("/my-room", get(handler))
            .with_state(broadcast_group);

        axum::Server::bind(&self.addr)
            .serve(app.into_make_service())
            .await
            .map_err(|_| "Failed to serve")?;

        Ok(())
    }
}

async fn handler(
    ws: WebSocketUpgrade,
    State(broadcast_group): State<Arc<BroadcastGroup>>,
) -> Response {
    println!("connection opened");
    ws.on_upgrade(move |socket| handle_socket(socket, broadcast_group.clone()))
}

async fn handle_socket(socket: WebSocket, broadcast_group: Arc<BroadcastGroup>) {
    let (sink, stream) = socket.split();

    let stream = tokio_stream::StreamExt::filter_map(stream, |d| match d {
        Ok(Message::Binary(s)) => Some(Ok::<_, Infallible>(s)),
        _ => {
            println!("got here");
            None
        }
    });

    let sink = sink.with(|d| ready(Ok::<_, axum::Error>(Message::Binary(d))));
    let sink = Arc::new(Mutex::new(sink));
    let sub = broadcast_group.subscribe(sink, stream);

    match sub.completed().await {
        Ok(_) => println!("socket closed"),
        Err(e) => println!("socket closed with error: {}", e),
    }
}
