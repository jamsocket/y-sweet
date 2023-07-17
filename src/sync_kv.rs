use crate::stores::Store;
use std::{
    collections::BTreeMap,
    convert::Infallible,
    ops::Bound,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use yrs_kvstore::KVEntry;

const DATA_FILENAME: &str = "data.bin";

enum SyncKvCommand {
    Get(Vec<u8>),
    Set(Vec<u8>, Vec<u8>),
    Remove(Vec<u8>),
    RemoveRange(Vec<u8>, Vec<u8>),
}

struct SyncKv {
    data: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
    store: Box<dyn Store>,
    dirty: AtomicBool,
    dirty_callback: Box<dyn Fn()>,
}

impl SyncKv {
    async fn new(
        store: Box<dyn Store>,
        dirty_callback: Box<dyn Fn()>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let data = if let Some(snapshot) = store.get(DATA_FILENAME).await? {
            bincode::deserialize(&snapshot)?
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            data: Arc::new(Mutex::new(data)),
            store,
            dirty: AtomicBool::new(false),
            dirty_callback,
        })
    }

    fn mark_dirty(&self) {
        if !self.dirty.load(Ordering::Relaxed) {
            self.dirty.store(true, Ordering::Relaxed);
            (self.dirty_callback)();
        }
    }

    async fn persist(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let data = self.data.lock().unwrap();
        let snapshot = bincode::serialize(&*data)?;
        self.store.set(DATA_FILENAME, snapshot).await?;
        self.dirty.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let map = self.data.lock().unwrap();
        map.get(key).cloned()
    }

    fn set(&self, key: &[u8], value: &[u8]) {
        let mut map = self.data.lock().unwrap();
        map.insert(key.to_vec(), value.to_vec());
        self.mark_dirty();
    }
}

struct SyncKvEntry {
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

struct SyncKvCursor {
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
        let result = Ok(SyncKvCursor {
            data: self.data.clone(),
            next_key: Bound::Included(from.to_vec()),
            to: to.to_vec(),
        });
        result
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
    use std::{error::Error, sync::mpsc::channel};
    use tokio;

    #[derive(Default, Clone)]
    struct MemoryStore {
        data: Arc<DashMap<Vec<u8>, Vec<u8>>>,
    }

    #[async_trait]
    impl Store for MemoryStore {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
            Ok(self.data.get(key.as_bytes()).map(|v| v.clone()))
        }

        async fn set(&mut self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn Error>> {
            self.data.insert(key.as_bytes().to_vec(), value);
            Ok(())
        }

        async fn remove(&mut self, key: &str) -> Result<(), Box<dyn Error>> {
            self.data.remove(key.as_bytes());
            Ok(())
        }
    }

    #[tokio::test]
    async fn calls_sync_callback() {
        let (send, recv) = channel();
        let store = MemoryStore::default();
        let sync_kv = SyncKv::new(
            Box::new(store.clone()),
            Box::new(move || {
                send.send(()).unwrap();
            }),
        )
        .await
        .unwrap();

        sync_kv.set(b"foo", b"bar");
        assert_eq!(sync_kv.get(b"foo"), Some(b"bar".to_vec()));

        assert!(store.data.is_empty());

        // We should have received a dirty callback.
        recv.try_recv().unwrap();

        sync_kv.set(b"abc", b"def");

        // We should not receive a dirty callback.
        recv.try_recv().unwrap_err();
    }

    #[tokio::test]
    async fn persists_to_store() {
        let store = MemoryStore::default();

        {
            let mut sync_kv = SyncKv::new(Box::new(store.clone()), Box::new(|| {}))
                .await
                .unwrap();

            sync_kv.set(b"foo", b"bar");
            assert_eq!(sync_kv.get(b"foo"), Some(b"bar".to_vec()));

            assert!(store.data.is_empty());

            sync_kv.persist().await.unwrap();
        }

        {
            let sync_kv = SyncKv::new(Box::new(store.clone()), Box::new(|| {}))
                .await
                .unwrap();

            assert_eq!(sync_kv.get(b"foo"), Some(b"bar".to_vec()));
        }
    }
}
