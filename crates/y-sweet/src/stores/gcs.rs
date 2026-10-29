use async_trait::async_trait;
use google_cloud_storage::client::{Storage, StorageControl};
use std::sync::OnceLock;
use y_sweet_core::store::{Result, Store, StoreError};

fn bucket_resource(bucket: &str) -> String {
    format!("projects/_/buckets/{}", bucket)
}

fn prefixed_key(prefix: &Option<String>, key: &str) -> String {
    if let Some(prefix) = prefix {
        format!("{}/{}", prefix, key)
    } else {
        key.to_string()
    }
}

fn gax_status(err: &google_cloud_gax::error::Error) -> Option<u16> {
    err.http_status_code()
}

fn map_gax_error(op: &str, err: google_cloud_gax::error::Error) -> StoreError {
    match gax_status(&err) {
        Some(401) | Some(403) => StoreError::NotAuthorized(format!("{}: {}", op, err)),
        Some(404) => StoreError::DoesNotExist(format!("{}: {}", op, err)),
        _ => StoreError::ConnectionError(format!("{}: {}", op, err)),
    }
}

fn is_not_found(err: &google_cloud_gax::error::Error) -> bool {
    gax_status(err) == Some(404)
}

pub struct GcsStore {
    storage: Storage,
    control: StorageControl,
    bucket: String,
    prefix: Option<String>,
    bucket_checked: OnceLock<()>,
}

impl GcsStore {
    pub async fn new(bucket: String, prefix: Option<String>) -> anyhow::Result<Self> {
        let storage = Storage::builder().build().await?;
        let control = StorageControl::builder().build().await?;
        Ok(GcsStore {
            storage,
            control,
            bucket,
            prefix,
            bucket_checked: OnceLock::new(),
        })
    }

    async fn init_inner(&self) -> Result<()> {
        if self.bucket_checked.get().is_some() {
            return Ok(());
        }
        match self
            .control
            .get_bucket()
            .set_name(bucket_resource(&self.bucket))
            .send()
            .await
        {
            Ok(_) => {
                let _ = self.bucket_checked.set(());
                Ok(())
            }
            Err(e) if is_not_found(&e) => Err(StoreError::BucketDoesNotExist(format!(
                "Bucket {} does not exist.",
                self.bucket
            ))),
            Err(e) => Err(map_gax_error("get_bucket", e)),
        }
    }
}

#[async_trait]
impl Store for GcsStore {
    async fn init(&self) -> Result<()> {
        self.init_inner().await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        let mut resp = match self
            .storage
            .read_object(bucket_resource(&self.bucket), &key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) if is_not_found(&e) => return Ok(None),
            Err(e) => return Err(map_gax_error("read_object", e)),
        };
        let mut contents = Vec::new();
        while let Some(chunk) = resp
            .next()
            .await
            .transpose()
            .map_err(|e| StoreError::ConnectionError(format!("read_object body: {}", e)))?
        {
            contents.extend_from_slice(&chunk);
        }
        Ok(Some(contents))
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        self.storage
            .write_object(bucket_resource(&self.bucket), &key, bytes::Bytes::from(value))
            .send_buffered()
            .await
            .map_err(|e| map_gax_error("write_object", e))?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        match self
            .control
            .delete_object()
            .set_bucket(bucket_resource(&self.bucket))
            .set_object(&key)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if is_not_found(&e) => Ok(()),
            Err(e) => Err(map_gax_error("delete_object", e)),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        match self
            .control
            .get_object()
            .set_bucket(bucket_resource(&self.bucket))
            .set_object(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) if is_not_found(&e) => Ok(false),
            Err(e) => Err(map_gax_error("get_object", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_resource_name_format() {
        assert_eq!(bucket_resource("my-bucket"), "projects/_/buckets/my-bucket");
    }

    #[test]
    fn prefixed_key_joins_with_slash() {
        assert_eq!(
            prefixed_key(&Some("pre".to_string()), "doc.ysweet"),
            "pre/doc.ysweet"
        );
        assert_eq!(prefixed_key(&None, "doc.ysweet"), "doc.ysweet");
    }
}
