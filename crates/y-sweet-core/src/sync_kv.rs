use crate::store::Store;
use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    ops::Bound,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use yrs_kvstore::{DocOps, KVEntry};

pub struct SyncKv {
    data: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
    store: Option<Arc<Box<dyn Store>>>,
    key: String,
    dirty: AtomicBool,
    dirty_callback: Box<dyn Fn() + Send + Sync>,
}

impl SyncKv {
    pub async fn new<Callback: Fn() + Send + Sync + 'static>(
        store: Option<Arc<Box<dyn Store>>>,
        key: &str,
        callback: Callback,
    ) -> Result<Self> {
        let key = format!("{}/data.ysweet", key);

        let data = if let Some(store) = &store {
            if let Some(snapshot) = store.get(&key).await.context("Failed to get from store.")? {
                tracing::info!(size=?snapshot.len(), "Loaded snapshot");
                bincode::deserialize(&snapshot).context("Failed to deserialize.")?
            } else {
                BTreeMap::new()
            }
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            data: Arc::new(Mutex::new(data)),
            store,
            key,
            dirty: AtomicBool::new(false),
            dirty_callback: Box::new(callback),
        })
    }

    fn mark_dirty(&self) {
        if !self.dirty.load(Ordering::Relaxed) {
            self.dirty.store(true, Ordering::Relaxed);
            (self.dirty_callback)();
        }
    }

    pub async fn persist(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(store) = &self.store {
            let snapshot = {
                let data = self.data.lock().unwrap();
                bincode::serialize(&*data)?
            };

            tracing::info!(size=?snapshot.len(), "Persisting snapshot");
            store.set(&self.key, snapshot).await?;
        }
        self.dirty.store(false, Ordering::Relaxed);
        Ok(())
    }

    #[cfg(test)]
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let map = self.data.lock().unwrap();
        map.get(key).cloned()
    }

    #[cfg(test)]
    fn set(&self, key: &[u8], value: &[u8]) {
        let mut map = self.data.lock().unwrap();
        map.insert(key.to_vec(), value.to_vec());
        self.mark_dirty();
    }
}

impl<'d> DocOps<'d> for SyncKv {}

pub struct SyncKvEntry {
    key: Vec<u8>,
    value: Vec<u8>,
}

impl KVEntry for SyncKvEntry {
    fn key(&self) -> &[u8] {
        &self.key
    }

    fn value(&self) -> &[u8] {
        &self.value
    }
}

pub struct SyncKvCursor {
    data: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
    next_key: Bound<Vec<u8>>,
    to: Vec<u8>,
}

impl Iterator for SyncKvCursor {
    type Item = SyncKvEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let map = self.data.lock().unwrap();
        let next = map
            .range((self.next_key.clone(), Bound::Excluded(self.to.clone())))
            .next()?;
        self.next_key = Bound::Excluded(next.0.clone());
        Some(SyncKvEntry {
            key: next.0.clone(),
            value: next.1.clone(),
        })
    }
}

impl<'a> yrs_kvstore::KVStore<'a> for SyncKv {
    type Error = std::convert::Infallible;
    type Cursor = SyncKvCursor;
    type Entry = SyncKvEntry;
    type Return = Vec<u8>;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Infallible> {
        let map = self.data.lock().unwrap();
        Ok(map.get(key).cloned())
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        let mut map = self.data.lock().unwrap();
        map.remove(key);
        self.mark_dirty();
        Ok(())
    }

    fn iter_range(&self, from: &[u8], to: &[u8]) -> Result<Self::Cursor, Self::Error> {
        Ok(SyncKvCursor {
            data: self.data.clone(),
            next_key: Bound::Included(from.to_vec()),
            to: to.to_vec(),
        })
    }

    fn peek_back(&self, key: &[u8]) -> Result<Option<Self::Entry>, Self::Error> {
        let map = self.data.lock().unwrap();
        let prev = map.range(..key.to_vec()).next_back();
        Ok(prev.map(|(k, v)| SyncKvEntry {
            key: k.clone(),
            value: v.clone(),
        }))
    }

    fn upsert(&self, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        let mut map = self.data.lock().unwrap();
        map.insert(key.to_vec(), value.to_vec());
        self.mark_dirty();
        Ok(())
    }

    fn remove_range(&self, from: &[u8], to: &[u8]) -> Result<(), Self::Error> {
        for entry in self.iter_range(from, to)? {
            let mut map = self.data.lock().unwrap();
            map.remove(&entry.key);
        }
        self.mark_dirty();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_trait::async_trait;
    use dashmap::DashMap;
    use std::sync::atomic::AtomicUsize;
    use tokio;

    #[derive(Default, Clone)]
    struct MemoryStore {
        data: Arc<DashMap<String, Vec<u8>>>,
    }

    #[cfg_attr(not(feature = "single-threaded"), async_trait)]
    #[cfg_attr(feature = "single-threaded", async_trait(?Send))]
    impl Store for MemoryStore {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            Ok(self.data.get(key).map(|v| v.clone()))
        }

        async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
            self.data.insert(key.to_owned(), value);
            Ok(())
        }

        async fn remove(&self, key: &str) -> Result<()> {
            self.data.remove(key);
            Ok(())
        }

        async fn exists(&self, key: &str) -> Result<bool> {
            Ok(self.data.contains_key(key))
        }
    }

    #[derive(Default, Clone)]
    struct CallbackCounter {
        data: Arc<AtomicUsize>,
    }

    impl CallbackCounter {
        fn callback(&self) -> Box<dyn Fn() + Send + Sync> {
            let data = self.data.clone();
            Box::new(move || {
                data.fetch_add(1, Ordering::Relaxed);
            })
        }

        fn count(&self) -> usize {
            self.data.load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn calls_sync_callback() {
        let store = MemoryStore::default();
        let c = CallbackCounter::default();
        let sync_kv = SyncKv::new(Some(Arc::new(Box::new(store.clone()))), "foo", c.callback())
            .await
            .unwrap();

        assert_eq!(c.count(), 0);
        sync_kv.set(b"foo", b"bar");
        assert_eq!(sync_kv.get(b"foo"), Some(b"bar".to_vec()));

        assert!(store.data.is_empty());

        // We should have received a dirty callback.
        assert_eq!(c.count(), 1);

        sync_kv.set(b"abc", b"def");

        // We should not receive a dirty callback.
        assert_eq!(c.count(), 1);
    }

    #[tokio::test]
    async fn persists_to_store() {
        let store = MemoryStore::default();

        {
            let sync_kv = SyncKv::new(Some(Arc::new(Box::new(store.clone()))), "foo", || ())
                .await
                .unwrap();

            sync_kv.set(b"foo", b"bar");
            assert_eq!(sync_kv.get(b"foo"), Some(b"bar".to_vec()));

            assert!(store.data.is_empty());

            sync_kv.persist().await.unwrap();
        }

        {
            let sync_kv = SyncKv::new(Some(Arc::new(Box::new(store.clone()))), "foo", || ())
                .await
                .unwrap();

            assert_eq!(sync_kv.get(b"foo"), Some(b"bar".to_vec()));
        }
    }
}
