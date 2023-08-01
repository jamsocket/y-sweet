use serde::{Deserialize, Serialize};
use worker::Env;

const BUCKET: &str = "Y_SWEET_DATA";
const AUTH_KEY: &str = "AUTH_KEY";
const USE_HTTPS: &str = "USE_HTTPS";

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub auth_key: Option<String>,
    pub use_https: bool,
    pub bucket: String,
    pub bucket_prefix: Option<String>,
    pub url_prefix: Option<String>,
}

impl From<&Env> for Configuration {
    fn from(env: &Env) -> Self {
        let auth_key = env.var(AUTH_KEY).map(|s| s.to_string()).ok();
        let use_https = env
            .var(USE_HTTPS)
            .map(|s| s.to_string() != "false")
            .unwrap_or(false);
        Self {
            auth_key,
            use_https,
            bucket: BUCKET.to_string(),
            bucket_prefix: None,
            url_prefix: None,
        }
    }
}
