use serde::{Deserialize, Serialize};
use std::time::Duration;
use worker::Env;

const BUCKET: &str = "Y_SWEET_DATA";
const AUTH_KEY: &str = "AUTH_KEY";
const CHECKPOINT_FREQ_SECONDS: &str = "CHECKPOINT_FREQ_SECONDS";

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub auth_key: Option<String>,
    pub bucket: String,
    pub bucket_prefix: Option<String>,
    pub url_prefix: Option<String>,
    pub timeout_interval: Duration,
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
        Self {
            auth_key,
            bucket: BUCKET.to_string(),
            bucket_prefix: None,
            url_prefix: None,
            timeout_interval,
        }
    }
}
