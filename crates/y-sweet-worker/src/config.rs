use serde::{Deserialize, Serialize};
use worker::Env;

const BUCKET: &str = "Y_SWEET_DATA";
const AUTH_KEY: &str = "AUTH_KEY";

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub auth_key: Option<String>,
    pub bucket: String,
    pub bucket_prefix: Option<String>,
    pub url_prefix: Option<String>,
}

impl From<&Env> for Configuration {
    fn from(env: &Env) -> Self {
        let auth_key = env.var(AUTH_KEY).map(|s| s.to_string()).ok();
        Self {
            auth_key,
            bucket: BUCKET.to_string(),
            bucket_prefix: None,
            url_prefix: None,
        }
    }
}
