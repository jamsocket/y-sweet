use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::primitives::ByteStream;
use std::sync::OnceLock;
use y_sweet_core::store::{Result, Store, StoreError};

pub struct S3StoreConfig {
    pub bucket: String,
    pub prefix: Option<String>,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub force_path_style: bool,
}

pub struct S3Store {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: Option<String>,
    bucket_checked: OnceLock<()>,
}

fn prefixed_key(prefix: &Option<String>, key: &str) -> String {
    if let Some(prefix) = prefix {
        format!("{}/{}", prefix, key)
    } else {
        key.to_string()
    }
}

fn map_sdk_error<E>(op: &str, err: &SdkError<E>) -> StoreError
where
    E: std::fmt::Debug,
{
    if let SdkError::ServiceError(ctx) = err {
        let status = http_status(err);
        match status {
            Some(401) | Some(403) => {
                return StoreError::NotAuthorized(format!("{}: {:?}", op, ctx.err()))
            }
            Some(404) => return StoreError::DoesNotExist(format!("{}: {:?}", op, ctx.err())),
            _ => {
                return StoreError::ConnectionError(format!(
                    "{}: status {:?}: {:?}",
                    op,
                    status,
                    ctx.err()
                ))
            }
        }
    }
    StoreError::ConnectionError(format!("{}: {}", op, err))
}

fn http_status<E>(err: &SdkError<E>) -> Option<u16> {
    match err {
        SdkError::ServiceError(ctx) => Some(ctx.raw().status().as_u16()),
        SdkError::ResponseError(ctx) => Some(ctx.raw().status().as_u16()),
        _ => None,
    }
}

fn is_not_found<E>(err: &SdkError<E>) -> bool {
    http_status(err) == Some(404)
}

impl S3Store {
    pub async fn new(config: S3StoreConfig) -> Self {
        let mut loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = &config.region {
            loader = loader.region(Region::new(region.clone()));
        } else {
            loader = loader.region(
                aws_config::meta::region::RegionProviderChain::default_provider()
                    .or_else(Region::new("us-east-1")),
            );
        }

        if let Some(endpoint) = &config.endpoint {
            loader = loader.endpoint_url(endpoint.clone());
        } else {
            loader = loader.use_dual_stack(true);
        }

        let sdk_config = loader.load().await;
        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .force_path_style(config.force_path_style)
            .build();

        S3Store {
            client: aws_sdk_s3::Client::from_conf(s3_config),
            bucket: config.bucket,
            prefix: config.prefix,
            bucket_checked: OnceLock::new(),
        }
    }

    async fn init_inner(&self) -> Result<()> {
        if self.bucket_checked.get().is_some() {
            return Ok(());
        }
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                let _ = self.bucket_checked.set(());
                Ok(())
            }
            Err(e) if is_not_found(&e) => Err(StoreError::BucketDoesNotExist(format!(
                "Bucket {} does not exist.",
                self.bucket
            ))),
            Err(e) => Err(map_sdk_error("head_bucket", &e)),
        }
    }
}

#[async_trait]
impl Store for S3Store {
    async fn init(&self) -> Result<()> {
        self.init_inner().await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| StoreError::ConnectionError(format!("get_object body: {}", e)))?;
                Ok(Some(bytes.into_bytes().to_vec()))
            }
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(map_sdk_error("get_object", &e)),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(value))
            .send()
            .await
            .map_err(|e| map_sdk_error("put_object", &e))?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| map_sdk_error("delete_object", &e))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.init_inner().await?;
        let key = prefixed_key(&self.prefix, key);
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) if is_not_found(&e) => Ok(false),
            Err(e) => Err(map_sdk_error("head_object", &e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixed_key_joins_with_slash() {
        assert_eq!(
            prefixed_key(&Some("pre".to_string()), "doc/data.ysweet"),
            "pre/doc/data.ysweet"
        );
        assert_eq!(prefixed_key(&None, "doc/data.ysweet"), "doc/data.ysweet");
    }
}
