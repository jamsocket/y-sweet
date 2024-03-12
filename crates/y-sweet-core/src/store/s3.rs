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
    pub bucket: String,
    pub region: String,
    pub bucket_prefix: Option<String>,
}

const PRESIGNED_URL_DURATION: Duration = Duration::from_secs(60 * 60);

pub struct S3Store {
    bucket: Bucket,
    _bucket_checked: OnceLock<()>,
    client: Client,
    credentials: Credentials,
    prefix: Option<String>,
}

impl S3Store {
    pub fn new(config: S3Config) -> Self {
        let credentials = Credentials::new(config.key, config.secret);
        let endpoint: Url = config.endpoint.parse().expect("endpoint is a valid url");
        let path_style =
            // if endpoint is localhost then bucket url must be of forme http://localhost:<port>/<bucket>
            // instead of <method>:://<bucket>.<endpoint>
            if endpoint.host_str().expect("endpoint Url should have host") == "localhost" {
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
}
