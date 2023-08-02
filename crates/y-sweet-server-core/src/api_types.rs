use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
pub struct NewDocResponse {
    pub doc: String,
}

#[derive(Deserialize)]
pub enum Authorization {
    #[serde(rename = "none")]
    Nothing,
    #[serde(rename = "readonly")]
    ReadOnly,
    #[serde(rename = "full")]
    Full,
}

impl Authorization {
    fn full() -> Self {
        Self::Full
    }
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct AuthDocRequest {
    #[serde(default = "Authorization::full")]
    authorization: Authorization,
    user_id: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, Value>,
}

#[derive(Serialize)]
pub struct ClientToken {
    pub url: String,
    pub doc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}
