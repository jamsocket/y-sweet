use super::{Result, StoreError};
use crate::store::Store;
use async_trait::async_trait;
use std::sync::OnceLock;
use std::time::Duration;

use aws_credential_types::Credentials as AwsCredentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Config};
use aws_types::region::Region;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct S3Config {
    pub key: String,
    pub secret: String,
    pub token: Option<String>,
    pub bucket: String,
    pub region: String,
    pub endpoint: String, // 例: "https://s3.amazonaws.com" or "http://localhost:9000"
    pub bucket_prefix: Option<String>, // 例: Some("app-prefix")
    pub path_style: bool, // MinIO などで true 推奨
}

const PRESIGNED_URL_DURATION: Duration = Duration::from_secs(60 * 60); // 60 min
const UPLOAD_PRESIGNED_URL_DURATION: Duration = Duration::from_secs(15 * 60); // 15 min

pub struct S3Store {
    client: Client,
    bucket: String,
    prefix: Option<String>,
    _bucket_checked: OnceLock<()>,
}

impl S3Store {
    /// 公式 SDK を使った初期化
    pub async fn new(config: S3Config) -> Result<Self> {
        // 既定のローダにリージョンを設定
        let loader = aws_config::from_env().region(Region::new(config.region.clone()));
        let base = loader.load().await;

        // Explicit credentials (not needed if environment variables or ~/.aws exist, but useful for compatible S3 and CI)
        let creds = AwsCredentials::new(
            config.key,
            config.secret,
            config.token,
            None,     // expires_after
            "manual", // provider_name
        );

        let mut builder = aws_sdk_s3::config::Builder::from(&base)
            .region(Region::new(config.region))
            .credentials_provider(creds)
            .force_path_style(config.path_style);

        // Override endpoint for compatible S3 or local (MinIO) usage
        if !config.endpoint.is_empty() {
            builder = builder.endpoint_url(config.endpoint);
        }

        let conf: Config = builder.build();
        let client = Client::from_conf(conf);

        Ok(Self {
            client,
            bucket: config.bucket,
            prefix: config.bucket_prefix,
            _bucket_checked: OnceLock::new(),
        })
    }

    /// Check bucket existence (HeadBucket)
    pub async fn init(&self) -> Result<()> {
        if self._bucket_checked.get().is_some() {
            return Ok(());
        }

        // Check existence with HeadBucket
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                self._bucket_checked.set(()).ok();
                Ok(())
            }
            Err(e) => {
                // AWS SDK v1.x has changed detailed error classification,
                // so use message-based detection
                let err_str = format!("{e:?}");
                if err_str.contains("NoSuchBucket") {
                    Err(StoreError::BucketDoesNotExist(format!(
                        "Bucket '{}' does not exist: {e}",
                        self.bucket
                    )))
                } else if err_str.contains("AccessDenied") || err_str.contains("Forbidden") {
                    Err(StoreError::ConnectionError(format!(
                        "Not authorized to access bucket '{}': {e}",
                        self.bucket
                    )))
                } else {
                    Err(StoreError::ConnectionError(format!(
                        "Failed to access bucket '{}': {e}",
                        self.bucket
                    )))
                }
            }
        }
    }

    fn prefixed_key(&self, key: &str) -> String {
        if let Some(pref) = &self.prefix {
            if key.is_empty() {
                pref.clone()
            } else {
                format!(
                    "{}/{}",
                    pref.trim_end_matches('/'),
                    key.trim_start_matches('/')
                )
            }
        } else {
            key.to_string()
        }
    }

    // ========== Single Object Operations ==========
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.init().await?;
        let k = self.prefixed_key(key);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(k)
            .send()
            .await
        {
            Ok(out) => {
                let data = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| {
                        StoreError::ConnectionError(format!(
                            "Failed to read object body for key '{}': {e}",
                            key
                        ))
                    })?
                    .into_bytes()
                    .to_vec();
                Ok(Some(data))
            }
            Err(err) => {
                // NotFound -> None
                if is_not_found(&err) {
                    Ok(None)
                } else {
                    Err(StoreError::ConnectionError(format!(
                        "Failed to get object '{}' from bucket '{}': {err}",
                        key, self.bucket
                    )))
                }
            }
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.init().await?;
        let k = self.prefixed_key(key);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(k)
            .body(ByteStream::from(value))
            .send()
            .await
            .map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to put object '{}' to bucket '{}': {e}",
                    key, self.bucket
                ))
            })?;

        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.init().await?;
        let k = self.prefixed_key(key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(k)
            .send()
            .await
            .map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to delete object '{}' from bucket '{}': {e}",
                    key, self.bucket
                ))
            })?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.init().await?;
        let k = self.prefixed_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(k)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => {
                if is_not_found(&err) {
                    Ok(false)
                } else {
                    Err(StoreError::ConnectionError(format!(
                        "Failed to check existence of object '{}' in bucket '{}': {err}",
                        key, self.bucket
                    )))
                }
            }
        }
    }

    // ========== Presigned URL ==========
    pub async fn generate_upload_presigned_url(&self, key: &str) -> Result<String> {
        self.init().await?;
        let k = self.prefixed_key(key);

        let presign_conf =
            aws_sdk_s3::presigning::PresigningConfig::expires_in(UPLOAD_PRESIGNED_URL_DURATION)
                .map_err(|e| {
                    StoreError::ConnectionError(format!("Failed to create presigning config: {e}"))
                })?;

        let req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(k)
            // 必要に応じて content_type 等をここで指定
            .presigned(presign_conf)
            .await
            .map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to generate upload presigned URL for '{}' in bucket '{}': {e}",
                    key, self.bucket
                ))
            })?;

        Ok(req.uri().to_string())
    }

    pub async fn generate_download_presigned_url(&self, key: &str) -> Result<String> {
        self.init().await?;
        let k = self.prefixed_key(key);

        let presign_conf =
            aws_sdk_s3::presigning::PresigningConfig::expires_in(PRESIGNED_URL_DURATION).map_err(
                |e| StoreError::ConnectionError(format!("Failed to create presigning config: {e}")),
            )?;

        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(k)
            .presigned(presign_conf)
            .await
            .map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to generate download presigned URL for '{}' in bucket '{}': {e}",
                    key, self.bucket
                ))
            })?;

        Ok(req.uri().to_string())
    }

    // ========== List Objects (prefix) ==========
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        self.init().await?;
        let full_prefix = self.prefixed_key(prefix).trim_end_matches('/').to_string() + "/";

        let mut results = Vec::new();
        let mut cont: Option<String> = None;

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&full_prefix);

            if let Some(token) = &cont {
                req = req.continuation_token(token);
            }

            let out = req.send().await.map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to list objects with prefix '{}' in bucket '{}': {e}",
                    prefix, self.bucket
                ))
            })?;

            // AWS SDK v1.x returns &[Object] from contents()
            for obj in out.contents() {
                if let Some(key) = obj.key() {
                    // Remove bucket prefix to get relative path
                    if let Some(rel) = key.strip_prefix(&full_prefix) {
                        if !rel.is_empty() {
                            results.push(rel.to_string());
                        }
                    }
                }
            }

            if out.is_truncated().unwrap_or(false) {
                cont = out.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(results)
    }

    // ========== Prefix Copy (Server Side) ==========
    async fn copy_object(&self, source_key: &str, destination_key: &str) -> Result<()> {
        // copy_source format is "bucket/source_key" (SDK handles proper encoding)
        let copy_source = format!("{}/{}", self.bucket, self.prefixed_key(source_key));

        // destination should already be prefixed_key
        let dest = self.prefixed_key(destination_key);

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(copy_source)
            .key(dest)
            .send()
            .await
            .map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to copy object from '{}' to '{}' in bucket '{}': {e}",
                    source_key, destination_key, self.bucket
                ))
            })?;

        Ok(())
    }

    /// Copy all objects under source_doc_id to destination_doc_id
    async fn copy_document(&self, source_doc_id: &str, destination_doc_id: &str) -> Result<()> {
        self.init().await?;

        // 1) Get relative key list from source full prefix
        let source_prefix = format!("{}/", source_doc_id.trim_matches('/'));
        let entries = self.list_objects(&source_prefix).await?;

        // 2) Copy each object server-side
        for rel in entries {
            let src_key = format!("{}/{}", source_doc_id.trim_matches('/'), rel);
            let dst_key = format!("{}/{}", destination_doc_id.trim_matches('/'), rel);
            self.copy_object(&src_key, &dst_key).await?;
        }

        Ok(())
    }
}

// S3 NotFound detection utility
fn is_not_found(err: &aws_sdk_s3::error::SdkError<impl std::fmt::Debug>) -> bool {
    // AWS SDK v1.x has changed detailed error classification,
    // so use message-based detection
    let s = format!("{err:?}");
    s.contains("NotFound")
        || s.contains("404")
        || s.contains("NoSuchKey")
        || s.contains("NoSuchBucket")
}

#[async_trait]
impl Store for S3Store {
    async fn init(&self) -> Result<()> {
        S3Store::init(self).await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        S3Store::get(self, key).await
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        S3Store::set(self, key, value).await
    }

    async fn remove(&self, key: &str) -> Result<()> {
        S3Store::remove(self, key).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        S3Store::exists(self, key).await
    }

    async fn generate_upload_presigned_url(&self, key: &str) -> Result<String> {
        S3Store::generate_upload_presigned_url(self, key).await
    }

    async fn generate_download_presigned_url(&self, key: &str) -> Result<String> {
        S3Store::generate_download_presigned_url(self, key).await
    }

    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        S3Store::list_objects(self, prefix).await
    }

    async fn copy_document(&self, source_doc_id: &str, destination_doc_id: &str) -> Result<()> {
        S3Store::copy_document(self, source_doc_id, destination_doc_id).await
    }
}
