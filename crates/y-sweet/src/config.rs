use serde::{Deserialize, Serialize};
use std::env::Vars;

#[derive(Serialize, Deserialize)]
pub struct S3Config {
    pub key: String,
    pub secret: String,
    pub bucket: String,
    pub region: String,
    pub bucket_prefix: Option<String>,
}
const S3_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const S3_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const S3_REGION: &str = "AWS_REGION";

const DEFAULT_S3_REGION: &str = "us-east-1";

pub fn parse_s3_config(env: &mut Vars, store_path: &str) -> anyhow::Result<S3Config> {
    let url = url::Url::parse(store_path)?;
    let bucket = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid S3 URL"))?
        .to_owned();
    let bucket_prefix = url.path().trim_start_matches('/').to_owned();

    Ok(S3Config {
        key: env
            .find_map(|(key, v)| (S3_ACCESS_KEY_ID == key).then_some(v))
            .ok_or_else(|| anyhow::anyhow!("AWS_ACCESS_KEY_ID env var not supplied"))?,
        region: env
            .find_map(|(key, v)| (S3_REGION == key).then_some(v))
            .unwrap_or_else(|| {
                println!("AWS_REGION env var not supplied, using us-east-1 as default");
                DEFAULT_S3_REGION.to_string()
            }),
        secret: env
            .find_map(|(key, v)| (S3_SECRET_ACCESS_KEY == key).then_some(v))
            .ok_or_else(|| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY env var not supplied"))?,
        bucket,
        bucket_prefix: Some(bucket_prefix),
    })
}
