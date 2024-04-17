use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use worker::Env;
use y_sweet_core::auth::KeyId;
use y_sweet_core::store::s3::S3Config;

const BUCKET: &str = "Y_SWEET_DATA";
const BUCKET_KIND: &str = "BUCKET_KIND";
const AUTH_KEY: &str = "AUTH_KEY";
const S3_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const S3_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const S3_REGION: &str = "AWS_REGION";
const S3_ENDPOINT: &str = "AWS_ENDPOINT_URL_S3";
const S3_BUCKET_PREFIX: &str = "S3_BUCKET_PREFIX";
const S3_BUCKET_NAME: &str = "S3_BUCKET_NAME";

// Note: unlike the native server, the worker checkpoint frequency is not configurable because
// it directly relates to Cloudflare platform configuration. Per their docs:
//
//     A Durable Object is given 30 seconds of additional CPU time for every request it processes,
//     including WebSocket messages. In the absence of failures, in-memory state should not be
//     reset after less than 30 seconds of inactivity.
//
// Ref: https://ts.cloudflare.community/workers/runtime-apis/durable-objects/#in-memory-state
//
// However, experimentally, this seems wrong. Earlier documentation mentioned 10 seconds as the
// reliable duration in the absence of failures, which seems to solve the problem:
//
// Ref: https://github.com/cloudflare/cloudflare-docs/pull/7625/files
const DEFAULT_CHECKPOINT_FREQ_SECONDS: u64 = 9;

#[derive(Serialize, Deserialize)]
pub enum BucketKind {
    NativeR2,
    S3Compatible,
}

impl FromStr for BucketKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R2" => Ok(Self::NativeR2),
            "S3" => Ok(Self::S3Compatible),
            _ => Err(anyhow::anyhow!("invalid bucket kind")),
        }
    }
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
    let region = env
        .var(S3_REGION)
        .map_err(|_| anyhow::anyhow!("AWS_REGION env var not supplied"))?
        .to_string();

    //default to using aws
    let endpoint = env.var(S3_ENDPOINT).map_or_else(
        |_| format!("https://s3.dualstack.{}.amazonaws.com", region),
        |s| s.to_string(),
    );

    Ok(S3Config {
        key: env
            .var(S3_ACCESS_KEY_ID)
            .map_err(|_| anyhow::anyhow!("AWS_ACCESS_KEY_ID env var not supplied"))?
            .to_string(),
        region,
        endpoint,
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
        let timeout_interval = Duration::from_secs(DEFAULT_CHECKPOINT_FREQ_SECONDS);

        let bucket_kind = env.var(BUCKET_KIND).map_or_else(
            |_| Ok(BucketKind::NativeR2),
            |b| BucketKind::from_str(&b.to_string()),
        )?;
        let s3_config = if let BucketKind::S3Compatible = bucket_kind {
            Some(parse_s3_config(env)?)
        } else {
            None
        };

        Ok(Self {
            auth_key,
            auth_key_id: None,
            bucket: BUCKET.to_string(),
            s3_store_config: s3_config,
            bucket_prefix: env.var(S3_BUCKET_PREFIX).map(|s| s.to_string()).ok(),
            url_prefix: None,
            timeout_interval,
        })
    }
}
