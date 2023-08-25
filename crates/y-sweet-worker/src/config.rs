use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use worker::Env;
use y_sweet_core::auth::KeyId;

const BUCKET: &str = "Y_SWEET_DATA";
const BUCKET_KIND: &str = "BUCKET_KIND";
const AUTH_KEY: &str = "AUTH_KEY";
const CHECKPOINT_FREQ_SECONDS: &str = "CHECKPOINT_FREQ_SECONDS";
const S3_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const S3_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const S3_REGION: &str = "AWS_REGION";
const S3_BUCKET_PREFIX: &str = "S3_BUCKET_PREFIX";
const S3_BUCKET_NAME: &str = "S3_BUCKET_NAME";

#[derive(Serialize, Deserialize)]
pub enum BucketKind {
    NativeR2,
    S3Compatible,
}

impl FromStr for BucketKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s == "R2" => Ok(Self::NativeR2),
            s if s == "S3" => Ok(Self::S3Compatible),
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
    pub auth_key_id: Option<KeyId>,
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
            .map_err(|_| anyhow::anyhow!("AWS_ACCESS_KEY_ID env var not supplied"))?
            .to_string(),
        region: env
            .var(S3_REGION)
            .map_err(|_| anyhow::anyhow!("AWS_REGION env var not supplied"))?
            .to_string(),
        secret: env
            .var(S3_SECRET_ACCESS_KEY)
            .map_err(|_| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY env var not supplied"))?
            .to_string(),
        bucket: env
            .var(S3_BUCKET_NAME)
            .map_err(|_| anyhow::anyhow!("S3_BUCKET_NAME env var not supplied"))?
            .to_string(),
        bucket_prefix: env.var(S3_BUCKET_PREFIX).ok().map(|t| t.to_string()),
    })
}

impl TryFrom<&Env> for Configuration {
    type Error = anyhow::Error;

    fn try_from(env: &Env) -> Result<Self, Self::Error> {
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

        let bucket_kind = env.var(BUCKET_KIND).map_or_else(
            |_| Ok(BucketKind::NativeR2),
            |b| BucketKind::from_str(&b.to_string()),
        )?;
        let s3_config = if let BucketKind::S3Compatible = bucket_kind {
            Some(parse_s3_config(&env)?)
        } else {
            None
        };

        Ok(Self {
            auth_key,
            auth_key_id: None,
            bucket: BUCKET.to_string(),
            s3_store_config: s3_config,
            bucket_prefix: None,
            url_prefix: None,
            timeout_interval,
        })
    }
}
