use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub key: String,
    pub endpoint: String,
    pub secret: String,
    pub bucket: String,
    pub region: String,
    pub bucket_prefix: Option<String>,
}
