use async_trait::async_trait;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use std::error::Error;

use super::Store;

pub struct S3Store {
    bucket: Bucket,
    prefix: String,
}

impl S3Store {
    pub async fn new(
        region: Region,
        bucket: String,
        prefix: String,
    ) -> Result<Self, Box<dyn Error>> {
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
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let response = self.bucket.get_object(&self.make_key(key)).await?;
        Ok(Some(response.to_vec()))
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let _code = self.bucket.put_object(&self.make_key(key), &value).await?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let _code = self.bucket.delete_object(&self.make_key(key)).await?;
        Ok(())
    }
}
