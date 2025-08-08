use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct NewDocResponse {
    #[serde(rename = "docId")]
    pub doc_id: String,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum Authorization {
    #[serde(rename = "read-only")]
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
pub struct AuthDocRequest {
    #[serde(default = "Authorization::full")]
    pub authorization: Authorization,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "validForSeconds")]
    pub valid_for_seconds: Option<u64>,
}

impl Default for AuthDocRequest {
    fn default() -> Self {
        Self {
            authorization: Authorization::Full,
            user_id: None,
            valid_for_seconds: None,
        }
    }
}

#[derive(Serialize)]
pub struct ClientToken {
    /// The URL compatible with the y-websocket provider. The provider will append
    /// a document ID to this string and establish a WebSocket connection.
    pub url: String,

    /// The base URL for document-level endpoints.
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,

    /// The document ID.
    #[serde(rename = "docId")]
    pub doc_id: String,

    /// An optional token that can be used to authenticate the client to the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    /// The authorization level of the client.
    #[serde(rename = "authorization")]
    pub authorization: Authorization,
}

#[derive(Deserialize)]
pub struct ContentUploadRequest {
    /// The content type of the file to upload
    #[serde(rename = "contentType")]
    pub content_type: String,
}

#[derive(Serialize)]
pub struct ContentUploadResponse {
    /// The signed URL for uploading the content
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,

    /// The asset ID that will be used to store the content
    #[serde(rename = "assetId")]
    pub asset_id: String,
}

#[derive(Serialize)]
pub struct AssetUrl {
    /// The asset ID (without extension) of the asset
    #[serde(rename = "assetId")]
    pub asset_id: String,

    /// The signed URL for downloading the asset
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
}

#[derive(Serialize)]
pub struct AssetsResponse {
    /// List of asset URLs with signed download URLs
    pub assets: Vec<AssetUrl>,
}

#[derive(Deserialize, Debug)]
pub struct DocCreationRequest {
    /// The ID of the document to create. If not provided, a random ID will be generated.
    #[serde(skip_serializing_if = "Option::is_none", rename = "docId")]
    pub doc_id: Option<String>,
}

#[derive(Deserialize)]
pub struct DocCopyRequest {
    /// The ID of the destination document where the source document will be copied to
    #[serde(rename = "destinationDocId")]
    pub destination_doc_id: String,
}

#[derive(Serialize)]
pub struct DocCopyResponse {
    /// The ID of the source document that was copied
    #[serde(rename = "sourceDocId")]
    pub source_doc_id: String,
    /// The ID of the destination document where the copy was created
    #[serde(rename = "destinationDocId")]
    pub destination_doc_id: String,
    /// Whether the copy operation was successful
    pub success: bool,
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
