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
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug)]
pub struct Authenticator {
    #[serde(with = "b64")]
    private_key: Vec<u8>,
    server_token: String,
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
    pub fn sign(self, private_key: &[u8]) -> String {
        let mut payload = bincode_encode(&self).expect("Bincode serialization should not fail.");
        payload.extend_from_slice(private_key);

        let token = hash(&payload);

        let auth_req = AuthenticatedRequest {
            payload: self,
            token,
        };

        let auth_enc = bincode_encode(&auth_req).expect("Bincode serialization should not fail.");
        b64_encode(&auth_enc)
    }

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

    pub fn verify(
        token: &str,
        private_key: &[u8],
        current_time: u64,
    ) -> Result<Payload, AuthError> {
        let auth_req: AuthenticatedRequest =
            bincode_decode(&b64_decode(token)?).map_err(|_| AuthError::InvalidToken)?;

        let mut payload =
            bincode_encode(&auth_req.payload).expect("Bincode serialization should not fail.");
        payload.extend_from_slice(private_key);

        let expected_token = hash(&payload);

        if expected_token != auth_req.token {
            Err(AuthError::InvalidSignature)
        } else if auth_req.payload.expiration_millis.unwrap_or(u64::MAX) < current_time {
            Err(AuthError::Expired)
        } else {
            Ok(auth_req.payload)
        }
    }
}

fn hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result.to_vec()
}

impl Authenticator {
    pub fn new(private_key: &str) -> Result<Self, AuthError> {
        let private_key = b64_decode(private_key)?;
        let server_token = Payload::new(Permission::Server).sign(&private_key);

        Ok(Self {
            private_key,
            server_token,
        })
    }

    pub fn server_token(&self) -> &str {
        &self.server_token
    }

    pub fn verify_server_token(
        &self,
        token: &str,
        current_time_epoch_millis: u64,
    ) -> Result<(), AuthError> {
        let payload = Payload::verify(token, &self.private_key, current_time_epoch_millis)?;
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
        payload.sign(&self.private_key)
    }

    fn verify_token(
        &self,
        token: &str,
        current_time_epoch_millis: u64,
    ) -> Result<Permission, AuthError> {
        let payload = Payload::verify(token, &self.private_key, current_time_epoch_millis)?;
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
