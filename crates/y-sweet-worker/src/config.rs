use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use worker::Env;

const BUCKET: &str = "Y_SWEET_DATA";
const BUCKET_KIND: &str = "BUCKET_KINDS";
const AUTH_KEY: &str = "AUTH_KEY";
const CHECKPOINT_FREQ_SECONDS: &str = "CHECKPOINT_FREQ_SECONDS";
const S3_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const S3_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const S3_REGION: &str = "AWS_REGION";
const S3_BUCKET_PREFIX: &str = "S3_BUCKET_PREFIX";
const S3_BUCKET_NAME: &str = "S3_BUCKET_NAME";

#[derive(Serialize, Deserialize)]
pub enum BucketKinds {
    R2,
    S3,
}

impl FromStr for BucketKinds {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s == "R2" => Ok(Self::R2),
            s if s == "S3" => Ok(Self::S3),
            _ => Err(anyhow::anyhow!("invalid bucket kind")),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct S3Config {
    pub key: String,
    pub secret: String,
    pub bucket: String,
    pub region: String,
    pub bucket_prefix: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub auth_key: Option<String>,
    pub bucket: String,
    pub s3_store_config: Option<S3Config>,
    pub bucket_prefix: Option<String>,
    pub url_prefix: Option<String>,
    pub timeout_interval: Duration,
}

fn parse_s3_config(env: &Env) -> anyhow::Result<S3Config> {
    Ok(S3Config {
        key: env
            .var(S3_ACCESS_KEY_ID)
            .map_err(|_| anyhow::anyhow!("ok"))?
            .to_string(),
        region: env
            .var(S3_REGION)
            .map_err(|_| anyhow::anyhow!("ok"))?
            .to_string(),
        secret: env
            .var(S3_SECRET_ACCESS_KEY)
            .map_err(|_| anyhow::anyhow!("ok!"))?
            .to_string(),
        bucket: env
            .var(S3_BUCKET_NAME)
            .map_err(|_| anyhow::anyhow!("no"))?
            .to_string(),
        bucket_prefix: env.var(S3_BUCKET_PREFIX).ok().map(|t| t.to_string()),
    })
}

impl From<&Env> for Configuration {
    fn from(env: &Env) -> Self {
        let auth_key = env.var(AUTH_KEY).map(|s| s.to_string()).ok();
        let timeout_interval = Duration::from_secs(
            env.var(CHECKPOINT_FREQ_SECONDS)
                .map(|s| {
                    s.to_string()
                        .parse()
                        .expect("Could not parse CHECKPOINT_FREQ_SECONDS as u64")
                })
                .unwrap_or(45),
        );

        let bucket_kind = env
            .var(BUCKET_KIND)
            .map_or_else(
                |_| Ok(BucketKinds::R2),
                |b| BucketKinds::from_str(&b.to_string()),
            )
            .unwrap();
        let s3_config = if let BucketKinds::S3 = bucket_kind {
            Some(parse_s3_config(&env).unwrap())
        } else {
            None
        };

        Self {
            auth_key,
            bucket: BUCKET.to_string(),
            s3_store_config: s3_config,
            bucket_prefix: None,
            url_prefix: None,
            timeout_interval,
        }
    }
}
