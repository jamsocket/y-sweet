use super::Store;
use anyhow::Result;
use async_trait::async_trait;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;

pub struct S3Store {
    bucket: Bucket,
    prefix: String,
}

impl S3Store {
    pub fn new(region: Region, bucket: String, prefix: String) -> Result<Self, anyhow::Error> {
        let credentials = Credentials::default()?;
        let bucket = Bucket::new(&bucket, region, credentials)?;
        Ok(Self { bucket, prefix })
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}/{}", &self.prefix, key)
    }
}

#[async_trait]
impl Store for S3Store {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let response = self.bucket.get_object(&self.make_key(key)).await;

        match response {
            Ok(result) => Ok(Some(result.to_vec())),
            Err(S3Error::Http(404, _)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let _code = self.bucket.put_object(&self.make_key(key), &value).await?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        let _code = self.bucket.delete_object(&self.make_key(key)).await?;
        Ok(())
    }
}
