use anyhow::{anyhow, Result};
use async_trait::async_trait;
use worker::Bucket;
use y_sweet_core::store::Store;

pub struct R2Store {
    bucket: Bucket,
    path_prefix: Option<String>,
}

impl R2Store {
    pub fn new(bucket: Bucket, path_prefix: Option<String>) -> Self {
        Self {
            bucket,
            path_prefix,
        }
    }

    fn prefixed_key(&self, key: &str) -> String {
        if let Some(path_prefix) = &self.path_prefix {
            format!("{}/{}", path_prefix, key)
        } else {
            key.to_string()
        }
    }
}

#[async_trait(?Send)]
impl Store for R2Store {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let object = self
            .bucket
            .get(&self.prefixed_key(key))
            .execute()
            .await
            .map_err(|_| anyhow!("Failed to get object"))?;
        if let Some(object) = object {
            let bytes = object
                .body()
                .ok_or_else(|| anyhow!("Object does not have body."))?
                .bytes()
                .await
                .map_err(|e| anyhow!("Failed to get object bytes {e}"))?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.bucket
            .put(&self.prefixed_key(key), value)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to put object {e}"))?;
        Ok(())
    }

    async fn remove(&self, _key: &str) -> Result<()> {
        todo!()
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.bucket
            .head(&self.prefixed_key(key))
            .await
            .map(|r| r.is_some())
            .map_err(|e| anyhow!("Failed to head object {e}"))
    }
}
