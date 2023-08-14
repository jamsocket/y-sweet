use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use rusty_s3::{Bucket, Credentials, S3Action};
use std::time::Duration;
use y_sweet_core::store::Store;

const PRESIGNED_URL_DURATION_SECONDS: u64 = 60 * 60;
pub struct S3Store {
    bucket: Bucket,
    credentials: Credentials,
    prefix: Option<String>,
    presigned_url_duration: Duration,
}

impl S3Store {
    pub fn new(
        region: String,
        bucket_name: String,
        prefix: Option<String>,
        credentials: Credentials,
    ) -> Result<Self, anyhow::Error> {
        let endpoint = format!("https://s3.dualstack.{}.amazonaws.com", region)
            .parse()
            .expect("endpoint is a valid Url");
        let path_style = rusty_s3::UrlStyle::VirtualHost;
        let bucket = Bucket::new(endpoint, path_style, bucket_name, region)
            .expect("Url has a valid scheme and host");

        Ok(S3Store {
            bucket,
            credentials,
            prefix,
            presigned_url_duration: Duration::from_secs(PRESIGNED_URL_DURATION_SECONDS),
        })
    }

    fn prefixed_key(&self, key: &str) -> String {
        if let Some(path_prefix) = &self.prefix {
            format!("{}/{}", path_prefix, key)
        } else {
            key.to_string()
        }
    }
}

#[async_trait(?Send)]
impl Store for S3Store {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let object_get = self
            .bucket
            .get_object(Some(&self.credentials), &self.prefixed_key(key));
        let presigned_url = object_get.sign(self.presigned_url_duration);
        let client = Client::new();
        let response = client.get(presigned_url).send().await?;
        Ok(Some(response.bytes().await?.to_vec()))
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let action = self
            .bucket
            .put_object(Some(&self.credentials), &self.prefixed_key(key));
        let presigned_url = action.sign(self.presigned_url_duration);
        let client = Client::new();
        let response = client.put(presigned_url).body(value).send().await?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        let action = self
            .bucket
            .delete_object(Some(&self.credentials), &self.prefixed_key(key));
        let presigned_url = action.sign(self.presigned_url_duration);
        let client = Client::new();
        client.delete(presigned_url).send().await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let action = self
            .bucket
            .head_object(Some(&self.credentials), &self.prefixed_key(key));
        let presigned_url = action.sign(self.presigned_url_duration);
        let client = Client::new();
        Ok(client.head(presigned_url).send().await?.status() == StatusCode::OK)
    }
}
