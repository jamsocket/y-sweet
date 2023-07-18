use crate::{api::serve_api, stores::Store, sync_kv::SyncKv, throttle::Throttle};
use anyhow::{anyhow, Context};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{mpsc::Receiver, RwLock};
use y_sync::{awareness::Awareness, net::BroadcastGroup};
use yrs::{Doc, Options, Transact};
use yrs_kvstore::DocOps;

const DOC_NAME: &str = "doc";

pub struct Server {
    pub store: Box<dyn Store>,
    pub addr: SocketAddr,
    pub checkpoint_freq: Duration,
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
        let broadcast_group = BroadcastGroup::new(awareness, 32).await;

        serve_api(&self.addr, broadcast_group).await
    }
}
