use super::{Result, StoreError};
use crate::store::Store;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{Client, Method, Response, StatusCode, Url};
use rusty_s3::{Bucket, Credentials, S3Action};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct S3Config {
    pub key: String,
    pub endpoint: String,
    pub secret: String,
    pub token: Option<String>,
    pub bucket: String,
    pub region: String,
    pub bucket_prefix: Option<String>,

    // Use old path-style URLs, needed to support some S3-compatible APIs (including some minio setups)
    pub path_style: bool,
}

const PRESIGNED_URL_DURATION: Duration = Duration::from_secs(60 * 60);
const UPLOAD_PRESIGNED_URL_DURATION: Duration = Duration::from_secs(15 * 60); // 15 minutes

pub struct S3Store {
    bucket: Bucket,
    _bucket_checked: OnceLock<()>,
    client: Client,
    credentials: Credentials,
    prefix: Option<String>,
}

impl S3Store {
    pub fn new(config: S3Config) -> Self {
        let credentials = if let Some(token) = config.token {
            Credentials::new_with_token(config.key, config.secret, token)
        } else {
            Credentials::new(config.key, config.secret)
        };
        let endpoint: Url = config.endpoint.parse().expect("endpoint is a valid url");

        let path_style = if config.path_style {
            rusty_s3::UrlStyle::Path
        } else if endpoint.host_str() == Some("localhost") {
            // Since this was the old behavior before we added AWS_S3_USE_PATH_STYLE,
            // we continue to support it, but complain a bit.
            tracing::warn!("Inferring path-style URLs for localhost for backwards-compatibility. This behavior may change in the future. Set AWS_S3_USE_PATH_STYLE=true to ensure that path-style URLs are used.");
            rusty_s3::UrlStyle::Path
        } else {
            rusty_s3::UrlStyle::VirtualHost
        };

        let bucket = Bucket::new(endpoint, path_style, config.bucket, config.region)
            .expect("Url has a valid scheme and host");
        let client = Client::new();

        S3Store {
            bucket,
            _bucket_checked: OnceLock::new(),
            client,
            credentials,
            prefix: config.bucket_prefix,
        }
    }

    pub async fn generate_upload_presigned_url(&self, key: &str) -> Result<String> {
        self.init().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .put_object(Some(&self.credentials), &prefixed_key);
        let url = action.sign_with_time(UPLOAD_PRESIGNED_URL_DURATION, &OffsetDateTime::now_utc());
        Ok(url.to_string())
    }

    pub async fn generate_download_presigned_url(&self, key: &str) -> Result<String> {
        self.init().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .get_object(Some(&self.credentials), &prefixed_key);
        let url = action.sign_with_time(PRESIGNED_URL_DURATION, &OffsetDateTime::now_utc());
        Ok(url.to_string())
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        self.init().await?;
        let prefixed_prefix = self.prefixed_key(prefix);
        let mut action = self.bucket.list_objects_v2(Some(&self.credentials));
        action.with_prefix(&prefixed_prefix);
        let url = action.sign_with_time(PRESIGNED_URL_DURATION, &OffsetDateTime::now_utc());

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(StoreError::ConnectionError(format!(
                "Received {} from S3-compatible API.",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

        // Parse XML response to extract object keys
        let objects = self.parse_list_objects_response(&body, prefix)?;
        Ok(objects)
    }

    fn parse_list_objects_response(&self, xml: &str, prefix: &str) -> Result<Vec<String>> {
        // Simple XML parsing for ListObjectsV2 response
        let mut objects = Vec::new();
        let lines: Vec<&str> = xml.lines().collect();

        for line in lines {
            if line.trim().starts_with("<Key>") && line.trim().ends_with("</Key>") {
                let key = line
                    .trim()
                    .trim_start_matches("<Key>")
                    .trim_end_matches("</Key>");

                // Remove the prefix from the key to get relative path
                if let Some(relative_key) = key.strip_prefix(&self.prefixed_key(prefix)) {
                    if !relative_key.is_empty() {
                        objects.push(relative_key.trim_start_matches('/').to_string());
                    }
                }
            }
        }

        Ok(objects)
    }

    async fn store_request<'a, A: S3Action<'a>>(
        &self,
        method: Method,
        action: A,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        let url = action.sign_with_time(PRESIGNED_URL_DURATION, &OffsetDateTime::now_utc());
        let mut request = self.client.request(method, url);

        request = if let Some(body) = body {
            request.body(body.to_vec())
        } else {
            request
        };

        let response = request.send().await;

        let response = match response {
            Ok(response) => response,
            Err(e) => return Err(StoreError::ConnectionError(e.to_string())),
        };

        match response.status() {
            StatusCode::OK => Ok(response),
            StatusCode::NOT_FOUND => Err(StoreError::DoesNotExist(
                "Received NOT_FOUND from S3-compatible API.".to_string(),
            )),
            StatusCode::FORBIDDEN => Err(StoreError::NotAuthorized(
                "Received FORBIDDEN from S3-compatible API.".to_string(),
            )),
            StatusCode::UNAUTHORIZED => Err(StoreError::NotAuthorized(
                "Received UNAUTHORIZED from S3-compatible API.".to_string(),
            )),
            _ => Err(StoreError::ConnectionError(format!(
                "Received {} from S3-compatible API.",
                response.status()
            ))),
        }
    }

    async fn read_response_bytes(response: Response) -> Result<Bytes> {
        match response.bytes().await {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(StoreError::ConnectionError(e.to_string())),
        }
    }

    pub async fn init(&self) -> Result<()> {
        if self._bucket_checked.get().is_some() {
            return Ok(());
        }

        let action = self.bucket.head_bucket(Some(&self.credentials));
        let result = self.store_request(Method::HEAD, action, None).await;

        match result {
            // Normally a 404 indicates that we are attempting to fetch an object that does
            // not exist, but we have only attempted to retrieve a bucket, so here it
            // indicates that the bucket does not exist.
            Err(StoreError::DoesNotExist(_)) => {
                return Err(StoreError::BucketDoesNotExist(
                    "Bucket does not exist.".to_string(),
                ))
            }
            Err(e) => return Err(e),
            Ok(response) => response,
        };

        self._bucket_checked.set(()).unwrap();
        Ok(())
    }

    fn prefixed_key(&self, key: &str) -> String {
        if let Some(path_prefix) = &self.prefix {
            format!("{}/{}", path_prefix, key)
        } else {
            key.to_string()
        }
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.init().await?;
        let prefixed_key = self.prefixed_key(key);
        let object_get = self
            .bucket
            .get_object(Some(&self.credentials), &prefixed_key);
        let response = self.store_request(Method::GET, object_get, None).await;

        match response {
            Ok(response) => {
                let result = Self::read_response_bytes(response).await?;
                Ok(Some(result.to_vec()))
            }
            Err(StoreError::DoesNotExist(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.init().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .put_object(Some(&self.credentials), &prefixed_key);
        self.store_request(Method::PUT, action, Some(value)).await?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.init().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .delete_object(Some(&self.credentials), &prefixed_key);
        self.store_request(Method::DELETE, action, None).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.init().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .head_object(Some(&self.credentials), &prefixed_key);
        let response = self.store_request(Method::HEAD, action, None).await;
        match response {
            Ok(_) => Ok(true),
            Err(StoreError::DoesNotExist(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn copy_document(&self, source_doc_id: &str, destination_doc_id: &str) -> Result<()> {
        self.init().await?;

        // List all objects with the source document prefix
        let source_prefix = format!("{}/", source_doc_id);
        let source_objects = self.list_objects(&source_prefix).await?;

        // Copy each object from source to destination (overwrite if exists)
        for object_key in source_objects {
            // Get the relative path from the source document
            let relative_path = if object_key.starts_with(&source_prefix) {
                &object_key[source_prefix.len()..]
            } else {
                continue; // Skip if not properly prefixed
            };

            // Create the destination key
            let destination_key = format!("{}/{}", destination_doc_id, relative_path);

            // Get the source object content
            if let Some(content) = self.get(&object_key).await? {
                // Set the content to the destination (this will overwrite if exists)
                self.set(&destination_key, content).await?;
            }
        }

        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Store for S3Store {
    async fn init(&self) -> Result<()> {
        self.init().await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.get(key).await
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.set(key, value).await
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.remove(key).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.exists(key).await
    }

    async fn generate_upload_presigned_url(&self, key: &str) -> Result<String> {
        self.generate_upload_presigned_url(key).await
    }

    async fn generate_download_presigned_url(&self, key: &str) -> Result<String> {
        self.generate_download_presigned_url(key).await
    }

    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        self.list_objects(prefix).await
    }

    async fn copy_document(&self, source_doc_id: &str, destination_doc_id: &str) -> Result<()> {
        self.copy_document(source_doc_id, destination_doc_id).await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Store for S3Store {
    async fn init(&self) -> Result<()> {
        self.init().await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.get(key).await
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.set(key, value).await
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.remove(key).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.exists(key).await
    }

    async fn generate_upload_presigned_url(&self, key: &str) -> Result<String> {
        self.generate_upload_presigned_url(key).await
    }

    async fn generate_download_presigned_url(&self, key: &str) -> Result<String> {
        self.generate_download_presigned_url(key).await
    }

    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        self.list_objects(prefix).await
    }

    async fn copy_document(&self, source_doc_id: &str, destination_doc_id: &str) -> Result<()> {
        self.copy_document(source_doc_id, destination_doc_id).await
    }
}
