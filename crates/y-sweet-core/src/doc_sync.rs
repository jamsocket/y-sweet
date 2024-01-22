use crate::{doc_connection::DOC_NAME, store::Store, sync::awareness::Awareness, sync_kv::SyncKv};
use anyhow::{anyhow, Context, Result};
use std::sync::{Arc, RwLock};
use serde_json::json;
use yrs::{Doc, Options, ReadTxn, Text, TextRef, Transact, UpdateSubscription};

use yrs_kvstore::DocOps;

pub struct DocWithSyncKv {
    awareness: Arc<RwLock<Awareness>>,
    sync_kv: Arc<SyncKv>,
    #[allow(unused)] // acts as RAII guard
    subscription: UpdateSubscription,
}

impl DocWithSyncKv {
    pub fn awareness(&self) -> Arc<RwLock<Awareness>> {
        self.awareness.clone()
    }

    pub fn sync_kv(&self) -> Arc<SyncKv> {
        self.sync_kv.clone()
    }

    pub async fn new<F>(
        key: &str,
        store: Option<Arc<Box<dyn Store>>>,
        dirty_callback: F,
    ) -> Result<Self>
        where
            F: Fn() + Send + Sync + 'static,
    {
        let sync_kv = SyncKv::new(store, key, dirty_callback)
            .await
            .context("Failed to create SyncKv")?;

        let sync_kv = Arc::new(sync_kv);
        let doc = Doc::new();

        {
            let mut txn = doc.transact_mut();
            sync_kv
                .load_doc(DOC_NAME, &mut txn)
                .map_err(|_| anyhow!("Failed to load doc"))?;
        }

        let subscription = {
            let sync_kv = sync_kv.clone();
            doc.observe_update_v1(move |_, event| {
                sync_kv.push_update(DOC_NAME, &event.update).unwrap();
                sync_kv
                    .flush_doc_with(DOC_NAME, Options::default())
                    .unwrap();
            })
                .map_err(|_| anyhow!("Failed to subscribe to updates"))?
        };

        let awareness = Arc::new(RwLock::new(Awareness::new(doc)));
        Ok(Self {
            awareness,
            sync_kv,
            subscription,
        })
    }

    pub async fn get_content(&self) -> Result<serde_json::Value> {
        // Access the Yjs Doc (yrs::Doc)
        let awareness = self.awareness.read().unwrap();
        let doc = awareness.doc();
        // Begin a read transaction on the document
        let txn = doc.transact();
        let store_string = txn.store().to_string();
        println!("store_string: {:?}", store_string);
        let json_content = json!( store_string);
        Ok(json_content)
    }
}
