use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait Store: 'static {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()>;
    async fn remove(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait Store: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()>;
    async fn remove(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
}

#[derive(Default, Clone)]
pub struct MemoryStore {
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

impl MemoryStore {
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
