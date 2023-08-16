use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use rusty_s3::{Bucket, Credentials, S3Action};
use time::OffsetDateTime;
use std::{time::Duration, cell::OnceCell};
use y_sweet_core::store::Store;

const PRESIGNED_URL_DURATION_SECONDS: u64 = 60 * 60;
pub struct S3Store {
    bucket: Bucket,
	init_bucket: OnceCell<()>,
    credentials: Credentials,
    prefix: Option<String>,
    presigned_url_duration: Duration,
}

impl S3Store {
    pub fn new(
        region: String,
        bucket_name: String,
        prefix: Option<String>,
        aws_access_key_id: String,
        aws_secret: String,
    ) -> Self {
        let credentials = Credentials::new(aws_access_key_id, aws_secret);
		let bucket_name = format!(
			"{}-{}", prefix.clone().unwrap_or("NA".to_string()), bucket_name);
        let endpoint = format!("https://s3.dualstack.{}.amazonaws.com", region)
            .parse()
            .expect("endpoint is a valid Url");
        let path_style = rusty_s3::UrlStyle::VirtualHost;
        let bucket = Bucket::new(endpoint, path_style, bucket_name, region)
            .expect("Url has a valid scheme and host");

		let presigned_url_duration = Duration::from_secs(PRESIGNED_URL_DURATION_SECONDS);
        S3Store {
            bucket,
			init_bucket: OnceCell::new(),
            credentials,
            prefix,
            presigned_url_duration
        }
    }

	async fn init_bucket(&self) -> Result<()> {
		if self.init_bucket.get().is_none() {
			let action = self.bucket.create_bucket(&self.credentials);
			let presigned_url = action.sign_with_time(
				self.presigned_url_duration, &OffsetDateTime::now_utc());
			let client = Client::new();
			let _response = client.put(presigned_url).send().await?;
			self.init_bucket.set(()).map_err(|_|anyhow::anyhow!("failed to set"))?;
		}

		self.init_bucket.get().ok_or(anyhow::anyhow!("should be ready!")).map(|_| ())
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
		self.init_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let object_get = self
            .bucket
            .get_object(Some(&self.credentials), &prefixed_key);
        let presigned_url = object_get.sign_with_time(
			self.presigned_url_duration, &OffsetDateTime::now_utc());
        let client = Client::new();
        let response = client.get(presigned_url).send().await?;
		match response.status() {
			StatusCode::NOT_FOUND => Ok(None),
			StatusCode::OK => Ok(Some(response.bytes().await?.to_vec())),
			_ => Err(anyhow::anyhow!("Not Ok"))
		}
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
		self.init_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .put_object(Some(&self.credentials), &prefixed_key);
        let presigned_url = action.sign_with_time(
			self.presigned_url_duration, &OffsetDateTime::now_utc());
        let client = Client::new();
        let _response = client.put(presigned_url).body(value).send().await?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
		self.init_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .delete_object(Some(&self.credentials), &prefixed_key);
        let presigned_url = action.sign_with_time(
			self.presigned_url_duration, &OffsetDateTime::now_utc());
        let client = Client::new();
        client.delete(presigned_url).send().await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
		self.init_bucket().await?;
        let prefixed_key = self.prefixed_key(key);
        let action = self
            .bucket
            .head_object(Some(&self.credentials), &prefixed_key);
        let presigned_url = action.sign_with_time(
			self.presigned_url_duration, &OffsetDateTime::now_utc());
        let client = Client::new();
        //this should only return false if 404? I think?
        let res_status = client.head(presigned_url).send().await?.status();
        match res_status {
            StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            _ => Err(anyhow::anyhow!(format!(
                "Existence check for bucket failed with HTTP Error Code: {res_status}"
            ))),
        }
    }
}