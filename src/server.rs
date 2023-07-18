use crate::{stores::Store, sync_kv::SyncKv};
use anyhow::{anyhow, Context};
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
use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        Mutex, RwLock,
    },
    task::JoinHandle,
    time::Instant,
};
use y_sync::{awareness::Awareness, net::BroadcastGroup};
use yrs::{Doc, Options, Transact};
use yrs_kvstore::DocOps;

const DOC_NAME: &str = "doc";

pub struct Server {
    pub store: Box<dyn Store>,
    pub addr: SocketAddr,
    pub checkpoint_freq: Duration,
}

#[derive(Default)]
struct ThrottleInner {
    last: Option<Instant>,
    handle: Option<JoinHandle<()>>,
}

struct Throttle {
    freq: Duration,
    sender: Sender<()>,
    inner: Arc<std::sync::Mutex<ThrottleInner>>,
}

impl Throttle {
    fn new(freq: Duration, sender: Sender<()>) -> Self {
        Self {
            freq,
            sender,
            inner: Arc::default(),
        }
    }

    fn call(&self) {
        tracing::info!("Throttle called");
        let mut inner = self.inner.lock().unwrap();
        if inner.handle.is_some() {
            tracing::info!("Throttle already deferred.");
            return;
        }
        let now = Instant::now();
        if let Some(last) = inner.last {
            if now - last < self.freq {
                tracing::info!("Deferring throttle");
                let freq = self.freq;
                let sender = self.sender.clone();
                let inner_clone = self.inner.clone();
                inner.handle.replace(tokio::spawn(async move {
                    tokio::time::sleep_until(last + freq).await;
                    tracing::info!("Deferred throttle ready.");
                    sender.try_send(()).unwrap();

                    let mut inner_clone = inner_clone.lock().unwrap();
                    inner_clone.last.replace(now);
                    inner_clone.handle.take();
                }));
                return;
            }
        }

        tracing::info!("Persisting.");
        self.sender.try_send(()).unwrap();
        inner.last.replace(now);
    }
}

impl Server {
    async fn persist_loop(sync_kv: Arc<SyncKv>, mut receiver: Receiver<()>) {
        loop {
            match receiver.recv().await {
                Some(_) => {
                    tracing::info!("Persisting");
                    sync_kv.persist().await.unwrap();
                }
                None => {
                    tracing::info!("Persist loop ended.");
                }
            }
        }
    }

    pub async fn serve(self) -> Result<(), anyhow::Error> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        let throttle = Throttle::new(self.checkpoint_freq, sender.clone());

        let sync_kv = SyncKv::new(self.store, move || {
            throttle.call();
        })
        .await
        .context("Failed to create SyncKv")?;

        let sync_kv = Arc::new(sync_kv);

        tokio::spawn(Self::persist_loop(sync_kv.clone(), receiver));

        let doc = Doc::new();

        {
            let mut txn = doc.transact_mut();
            sync_kv
                .load_doc(DOC_NAME, &mut txn)
                .map_err(|_| anyhow!("Failed to load doc"))?;
        }

        let _subscription_guard = doc
            .observe_update_v1(move |_, event| {
                sync_kv.push_update(DOC_NAME, &event.update).unwrap();
                sync_kv
                    .flush_doc_with(DOC_NAME, Options::default())
                    .unwrap();
            })
            .map_err(|_| anyhow!("Failed to subscribe to updates"))?;

        let awareness = Arc::new(RwLock::new(Awareness::new(doc)));
        let broadcast_group = Arc::new(BroadcastGroup::new(awareness, 32).await);

        let app = Router::new()
            .route("/my-room", get(handler))
            .with_state(broadcast_group);

        axum::Server::bind(&self.addr)
            .serve(app.into_make_service())
            .await
            .map_err(|_| anyhow!("Failed to serve"))?;

        Ok(())
    }
}

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
