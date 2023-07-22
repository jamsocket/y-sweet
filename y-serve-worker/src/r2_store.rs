use anyhow::{anyhow, Result};
use async_trait::async_trait;
use worker::Bucket;
use y_serve_core::store::Store;

pub struct R2Store {
    bucket: Bucket,
}

impl R2Store {
    pub fn new(bucket: Bucket) -> Self {
        Self { bucket }
    }
}

#[async_trait(?Send)]
impl Store for R2Store {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let object = self
            .bucket
            .get(key)
            .execute()
            .await
            .map_err(|_| anyhow!("Failed to get object"))?;
        if let Some(object) = object {
            let bytes = object
                .body()
                .ok_or_else(|| anyhow!("Object does not have body."))?
                .bytes()
                .await
                .map_err(|_| anyhow!("Failed to get object bytes"))?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.bucket
            .put(key, value)
            .execute()
            .await
            .map_err(|_| anyhow!("Failed to put object"))?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        todo!()
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        todo!()
    }
}
