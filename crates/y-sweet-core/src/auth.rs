use std::fmt::Display;

use base64::{
    alphabet::{STANDARD, URL_SAFE},
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    Engine,
};
use bincode::Options;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const EXPIRATION_MILLIS: u64 = 1000 * 60 * 60; // 60 minutes

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AuthError {
    #[error("The token is not a valid format")]
    InvalidToken,
    #[error("The token is expired")]
    Expired,
    #[error("The token is not valid for the requested resource")]
    InvalidResource,
    #[error("The token signature is invalid")]
    InvalidSignature,
    #[error("The key ID did not match")]
    KeyMismatch,
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug)]
pub struct Authenticator {
    #[serde(with = "b64")]
    private_key: Vec<u8>,
    key_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Permission {
    Server,
    Doc(String),
}

#[derive(Serialize, Deserialize)]
pub struct Payload {
    pub payload: Permission,
    pub expiration_millis: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthenticatedRequest {
    pub payload: Payload,
    pub token: Vec<u8>,
}

fn bincode_encode<T: Serialize>(value: &T) -> Result<Vec<u8>, bincode::Error> {
    // This uses different defaults than the default bincode::serialize() function.
    bincode::DefaultOptions::new().serialize(&value)
}

fn bincode_decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, bincode::Error> {
    // This uses different defaults than the default bincode::deserialize() function.
    bincode::DefaultOptions::new().deserialize(bytes)
}

fn b64_encode(bytes: &[u8]) -> String {
    let config = GeneralPurposeConfig::new().with_encode_padding(false);
    let engine = GeneralPurpose::new(&URL_SAFE, config);
    let mut buf = String::new();
    engine.encode_string(bytes, &mut buf);
    buf
}

fn b64_decode(str: &str) -> Result<Vec<u8>, AuthError> {
    let config =
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);
    let engine = GeneralPurpose::new(&STANDARD, config);

    if let Ok(result) = engine.decode(str) {
        return Ok(result);
    }

    // If we failed with STANDARD, try with URL_SAFE.
    let engine = GeneralPurpose::new(&URL_SAFE, config);

    engine.decode(str).map_err(|_| AuthError::InvalidToken)
}

mod b64 {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&b64_encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        b64_decode(&s).map_err(de::Error::custom)
    }
}

impl Payload {
    pub fn new(payload: Permission) -> Self {
        Self {
            payload,
            expiration_millis: None,
        }
    }

    pub fn new_with_expiration(payload: Permission, expiration_millis: u64) -> Self {
        Self {
            payload,
            expiration_millis: Some(expiration_millis),
        }
    }
}

fn hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result.to_vec()
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct KeyId(String);

#[derive(Error, Debug, PartialEq, Eq)]
pub enum KeyIdError {
    #[error("The key ID cannot be an empty string")]
    EmptyString,
    #[error("The key ID contains an invalid character: {ch}")]
    InvalidCharacter { ch: char },
}

impl KeyId {
    pub fn new(key_id: String) -> Result<Self, KeyIdError> {
        if key_id.is_empty() {
            return Err(KeyIdError::EmptyString);
        }

        let valid_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        for ch in key_id.chars() {
            if !valid_chars.contains(ch) {
                return Err(KeyIdError::InvalidCharacter { ch });
            }
        }

        Ok(Self(key_id))
    }
}

impl Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for KeyId {
    type Error = KeyIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_string())
    }
}

impl Authenticator {
    pub fn new(private_key: &str) -> Result<Self, AuthError> {
        let private_key = b64_decode(private_key)?;

        Ok(Self {
            private_key,
            key_id: None,
        })
    }

    pub fn server_token(&self) -> String {
        self.sign(Payload::new(Permission::Server))
    }

    fn sign(&self, payload: Payload) -> String {
        let mut hash_payload =
            bincode_encode(&payload).expect("Bincode serialization should not fail.");
        hash_payload.extend_from_slice(&self.private_key);

        let token = hash(&hash_payload);

        let auth_req = AuthenticatedRequest { payload, token };

        let auth_enc = bincode_encode(&auth_req).expect("Bincode serialization should not fail.");
        let result = b64_encode(&auth_enc);
        if let Some(key_id) = &self.key_id {
            format!("{}.{}", key_id, result)
        } else {
            result
        }
    }

    fn verify(&self, token: &str, current_time: u64) -> Result<Payload, AuthError> {
        let token = if let Some((prefix, token)) = token.split_once('.') {
            if Some(prefix) != self.key_id.as_deref() {
                return Err(AuthError::KeyMismatch);
            }

            token
        } else {
            if self.key_id.is_some() {
                return Err(AuthError::KeyMismatch);
            }

            token
        };

        let auth_req: AuthenticatedRequest =
            bincode_decode(&b64_decode(token)?).or(Err(AuthError::InvalidToken))?;

        let mut payload =
            bincode_encode(&auth_req.payload).expect("Bincode serialization should not fail.");
        payload.extend_from_slice(&self.private_key);

        let expected_token = hash(&payload);

        if expected_token != auth_req.token {
            Err(AuthError::InvalidSignature)
        } else if auth_req.payload.expiration_millis.unwrap_or(u64::MAX) < current_time {
            Err(AuthError::Expired)
        } else {
            Ok(auth_req.payload)
        }
    }

    pub fn with_key_id(self, key_id: KeyId) -> Self {
        Self {
            key_id: Some(key_id.0),
            ..self
        }
    }

    pub fn verify_server_token(
        &self,
        token: &str,
        current_time_epoch_millis: u64,
    ) -> Result<(), AuthError> {
        let payload = self.verify(token, current_time_epoch_millis)?;
        match payload {
            Payload {
                payload: Permission::Server,
                ..
            } => Ok(()),
            _ => Err(AuthError::InvalidResource),
        }
    }

    pub fn private_key(&self) -> String {
        b64_encode(&self.private_key)
    }

    pub fn gen_doc_token(&self, doc_id: &str, current_time_epoch_millis: u64) -> String {
        let expiration_time_epoch_millis = current_time_epoch_millis + EXPIRATION_MILLIS;
        let payload = Payload::new_with_expiration(
            Permission::Doc(doc_id.to_string()),
            expiration_time_epoch_millis,
        );
        self.sign(payload)
    }

    fn verify_token(
        &self,
        token: &str,
        current_time_epoch_millis: u64,
    ) -> Result<Permission, AuthError> {
        let payload = self.verify(token, current_time_epoch_millis)?;
        Ok(payload.payload)
    }

    pub fn verify_doc_token(
        &self,
        token: &str,
        doc: &str,
        current_time_epoch_millis: u64,
    ) -> Result<(), AuthError> {
        let payload = self.verify_token(token, current_time_epoch_millis)?;

        match payload {
            Permission::Doc(doc_id) => {
                if doc_id == doc {
                    Ok(())
                } else {
                    Err(AuthError::InvalidResource)
                }
            }
            _ => Err(AuthError::InvalidResource),
        }
    }

    pub fn gen_key() -> Result<Authenticator, AuthError> {
        let key = rand::thread_rng().gen::<[u8; 30]>();
        let key = b64_encode(&key);

        let authenticator = Authenticator::new(&key)?;
        Ok(authenticator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_b64() {
        let expect = [3, 242, 3, 248, 6, 220, 118];

        assert_eq!(b64_decode("A/ID+Abcdg==").unwrap(), expect);
        assert_eq!(b64_decode("A/ID+Abcdg").unwrap(), expect);

        assert_eq!(b64_decode("A_ID-Abcdg==").unwrap(), expect);
        assert_eq!(b64_decode("A_ID-Abcdg").unwrap(), expect);
    }

    #[test]
    fn test_b64_encode_options() {
        let data = [3, 242, 3, 248, 6, 220, 118];

        assert_eq!(b64_encode(&data), "A_ID-Abcdg");
    }

    #[test]
    fn test_simple_auth() {
        let authenticator = Authenticator::gen_key().unwrap();
        let token = authenticator.gen_doc_token("doc123", 0);
        assert_eq!(authenticator.verify_doc_token(&token, "doc123", 0), Ok(()));
        assert_eq!(
            authenticator.verify_doc_token(&token, "doc123", EXPIRATION_MILLIS + 1),
            Err(AuthError::Expired)
        );
        assert_eq!(
            authenticator.verify_doc_token(&token, "doc456", 0),
            Err(AuthError::InvalidResource)
        );
    }

    #[test]
    fn test_key_id() {
        let authenticator = Authenticator::gen_key()
            .unwrap()
            .with_key_id("myKeyId".try_into().unwrap());
        let token = authenticator.gen_doc_token("doc123", 0);
        assert!(
            token.starts_with("myKeyId."),
            "Token {} does not start with myKeyId.",
            token
        );
        assert_eq!(authenticator.verify_doc_token(&token, "doc123", 0), Ok(()));

        let token = authenticator.server_token();
        assert!(
            token.starts_with("myKeyId."),
            "Token {} does not start with myKeyId.",
            token
        );
        assert_eq!(authenticator.verify_server_token(&token, 0), Ok(()));
    }

    #[test]
    fn test_construct_key_id() {
        assert_eq!(KeyId::new("".to_string()), Err(KeyIdError::EmptyString));
        assert_eq!(
            KeyId::new("*".to_string()),
            Err(KeyIdError::InvalidCharacter { ch: '*' })
        );
        assert_eq!(
            KeyId::new("myKeyId".to_string()),
            Ok(KeyId("myKeyId".to_string()))
        );
    }

    #[test]
    fn test_key_id_mismatch() {
        let authenticator = Authenticator::gen_key()
            .unwrap()
            .with_key_id("myKeyId".try_into().unwrap());
        let token = authenticator.gen_doc_token("doc123", 0);
        let token = token.replace("myKeyId.", "aDifferentKeyId.");
        assert!(token.starts_with("aDifferentKeyId."));
        assert_eq!(
            authenticator.verify_doc_token(&token, "doc123", 0),
            Err(AuthError::KeyMismatch)
        );
    }

    #[test]
    fn test_missing_key_id() {
        let authenticator = Authenticator::gen_key()
            .unwrap()
            .with_key_id("myKeyId".try_into().unwrap());
        let token = authenticator.gen_doc_token("doc123", 0);
        let token = token.replace("myKeyId.", "");
        assert_eq!(
            authenticator.verify_doc_token(&token, "doc123", 0),
            Err(AuthError::KeyMismatch)
        );
    }

    #[test]
    fn test_unexpected_key_id() {
        let authenticator = Authenticator::gen_key().unwrap();
        let token = authenticator.gen_doc_token("doc123", 0);
        let token = format!("unexpectedKeyId.{}", token);
        assert_eq!(
            authenticator.verify_doc_token(&token, "doc123", 0),
            Err(AuthError::KeyMismatch)
        );
    }

    #[test]
    fn test_invalid_signature() {
        let authenticator = Authenticator::gen_key().unwrap();
        let actual_payload = Payload::new(Permission::Doc("doc123".to_string()));
        let mut encoded_payload =
            bincode_encode(&actual_payload).expect("Bincode serialization should not fail.");
        encoded_payload.extend_from_slice(&authenticator.private_key);

        let token = hash(&encoded_payload);

        let auth_req = AuthenticatedRequest {
            payload: Payload::new(Permission::Doc("abc123".to_string())),
            token: token,
        };

        let auth_enc = bincode_encode(&auth_req).expect("Bincode serialization should not fail.");
        let signed = b64_encode(&auth_enc);

        assert_eq!(
            authenticator.verify_doc_token(&signed, "doc123", 0),
            Err(AuthError::InvalidSignature)
        );
        assert_eq!(
            authenticator.verify_doc_token(&signed, "abc123", 0),
            Err(AuthError::InvalidSignature)
        );
    }

    #[test]
    fn test_roundtrip_serde_authenticator() {
        let authenticator = Authenticator::gen_key().unwrap();
        let serialized = serde_json::to_string(&authenticator).unwrap();
        let deserialized: Authenticator = serde_json::from_str(&serialized).unwrap();
        assert_eq!(authenticator, deserialized);
    }
}
