use crate::{stores::Store, sync_kv::SyncKv, throttle::Throttle};
use anyhow::{anyhow, Context, Result};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    task::JoinHandle,
};
use y_sync::{awareness::Awareness, net::BroadcastGroup};
use yrs::{Doc, Options, Subscription, Transact, TransactionMut, UpdateEvent};
use yrs_kvstore::DocOps;

const DOC_NAME: &str = "doc";

pub struct DocService {
    pub broadcast_group: Arc<BroadcastGroup>,
    pub handle: JoinHandle<()>,
    pub subscription: Subscription<Arc<dyn Fn(&TransactionMut<'_>, &UpdateEvent)>>,
}

async fn persist_loop(sync_kv: Arc<SyncKv>, mut receiver: Receiver<()>) {
    loop {
        match receiver.recv().await {
            Some(()) => {
                tracing::info!("Persisting");
                sync_kv.persist().await.unwrap();
            }
            None => {
                tracing::info!("Persist loop ended.");
            }
        }
    }
}

impl DocService {
    pub async fn new(
        store: Arc<Box<dyn Store>>,
        key: String,
        checkpoint_freq: Duration,
    ) -> Result<Self> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        let throttle = Throttle::new(checkpoint_freq, sender.clone());

        let sync_kv = SyncKv::new(store, &key, move || {
            throttle.call();
        })
        .await
        .context("Failed to create SyncKv")?;

        let sync_kv = Arc::new(sync_kv);

        let handle = tokio::spawn(persist_loop(sync_kv.clone(), receiver));

        let doc = Doc::new();

        {
            let mut txn = doc.transact_mut();
            sync_kv
                .load_doc(DOC_NAME, &mut txn)
                .map_err(|_| anyhow!("Failed to load doc"))?;
        }

        let subscription = doc
            .observe_update_v1(move |_, event| {
                sync_kv.push_update(DOC_NAME, &event.update).unwrap();
                sync_kv
                    .flush_doc_with(DOC_NAME, Options::default())
                    .unwrap();
            })
            .map_err(|_| anyhow!("Failed to subscribe to updates"))?;

        let awareness = Arc::new(RwLock::new(Awareness::new(doc)));
        let broadcast_group = Arc::new(BroadcastGroup::new(awareness, 32).await);

        Ok(DocService {
            broadcast_group,
            handle,
            subscription,
        })
    }
}
