use crate::store::Store;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use rusty_s3::{Bucket, Credentials, S3Action};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub key: String,
    pub endpoint: String,
    pub secret: String,
    pub bucket: String,
    pub region: String,
    pub bucket_prefix: Option<String>,
}

const PRESIGNED_URL_DURATION_SECONDS: u64 = 60 * 60;
pub struct S3Store {
    bucket: Bucket,
    _bucket_inited: AtomicBool,
    client: Client,
    credentials: Credentials,
    prefix: Option<String>,
    presigned_url_duration: Duration,
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

        let presigned_url_duration = Duration::from_secs(PRESIGNED_URL_DURATION_SECONDS);
        S3Store {
            bucket,
            _bucket_inited: AtomicBool::new(false),
            client,
            credentials,
            prefix: config.bucket_prefix,
            presigned_url_duration,
        }
    }

    //lazily checks bucket exists on first use
    async fn inited_bucket(&self) -> Result<&Bucket> {
        if self._bucket_inited.load(SeqCst) {
            return Ok(&self.bucket);
        }

        let action = self.bucket.head_bucket(Some(&self.credentials));
        let presigned_url =
            action.sign_with_time(self.presigned_url_duration, &OffsetDateTime::now_utc());
        let response = self.client.head(presigned_url).send().await?;
        match response.status() {
            StatusCode::OK => {
                self._bucket_inited.store(true, SeqCst);
                return Ok(&self.bucket);
            }
            StatusCode::NOT_FOUND => {
                return Err(anyhow::anyhow!(
                    "No such bucket {} exists!",
                    self.bucket.name()
                ))
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Other AWS Error: Code {} Err {}",
                    response.status(),
                    response.text().await?
                ))
            }
        }
    }

    fn prefixed_key(&self, key: &str) -> String {
        if let Some(path_prefix) = &self.prefix {
            format!("{}/{}", path_prefix, key)
        } else {
            key.to_string()
        }
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let bucket = self.inited_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let object_get = bucket.get_object(Some(&self.credentials), &prefixed_key);
        let presigned_url =
            object_get.sign_with_time(self.presigned_url_duration, &OffsetDateTime::now_utc());
        let response = self.client.get(presigned_url).send().await?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(None),
            StatusCode::OK => Ok(Some(response.bytes().await?.to_vec())),
            _ => Err(anyhow::anyhow!(
                "Other AWS Error: Code {} Err {}",
                response.status(),
                response.text().await?
            )),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let bucket = self.inited_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = bucket.put_object(Some(&self.credentials), &prefixed_key);
        let presigned_url =
            action.sign_with_time(self.presigned_url_duration, &OffsetDateTime::now_utc());
        let _response = self.client.put(presigned_url).body(value).send().await?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        let bucket = self.inited_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = bucket.delete_object(Some(&self.credentials), &prefixed_key);
        let presigned_url =
            action.sign_with_time(self.presigned_url_duration, &OffsetDateTime::now_utc());
        self.client.delete(presigned_url).send().await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let bucket = self.inited_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = bucket.head_object(Some(&self.credentials), &prefixed_key);
        let presigned_url =
            action.sign_with_time(self.presigned_url_duration, &OffsetDateTime::now_utc());
        let res_status = self.client.head(presigned_url).send().await?.status();
        match res_status {
            StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            _ => Err(anyhow::anyhow!(
                "Existence check for bucket failed with HTTP Error Code: {res_status}"
            )),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Store for S3Store {
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
