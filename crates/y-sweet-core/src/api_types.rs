use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
pub struct NewDocResponse {
    #[serde(rename = "docId")]
    pub doc_id: String,
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
    pub authorization: Authorization,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[derive(Serialize)]
pub struct ClientToken {
    pub url: String,
    #[serde(rename = "docId")]
    pub doc_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct DocCreationRequest {
    /// The ID of the document to create. If not provided, a random ID will be generated.
    #[serde(skip_serializing_if = "Option::is_none", rename = "docId")]
    pub doc_id: Option<String>,
}

/// Validate that the document name contains only alphanumeric characters, dashes, and underscores.
/// This is the same alphabet used by nanoid when we generate a document name.
pub fn validate_doc_name(doc_name: &str) -> bool {
    if doc_name.is_empty() {
        return false;
    }
    for c in doc_name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return false;
        }
    }
    true
}
