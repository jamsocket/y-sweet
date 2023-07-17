use async_trait::async_trait;
use std::error::Error;

pub mod filesystem;

#[async_trait]
pub trait Store: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    async fn set(&self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn Error>>;
    async fn remove(&self, key: &str) -> Result<(), Box<dyn Error>>;
}
