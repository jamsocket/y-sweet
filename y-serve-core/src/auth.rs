use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use rand::Rng;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub struct Authenticator {
    private_key: String,
    server_token: String,
}

fn hash_string(str: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(str);
    let result = hasher.finalize();
    format!("{:x}", result)
}

fn b64_encode_str(str: &str) -> String {
    b64_encode(str.as_bytes())
}

fn b64_encode(bytes: &[u8]) -> String {
    let mut buf = String::new();
    general_purpose::STANDARD.encode_string(bytes, &mut buf);
    buf
}

impl Authenticator {
    pub fn new(private_key: &str) -> Result<Self> {
        let server_token = hash_string(&private_key);

        Ok(Self {
            private_key: private_key.to_string(),
            server_token,
        })
    }

    pub fn server_token(&self) -> &str {
        &self.server_token
    }

    pub fn server_token_b64(&self) -> String {
        let mut buf = String::new();
        general_purpose::STANDARD.encode_string(self.server_token.as_bytes(), &mut buf);
        buf
    }

    pub fn private_key(&self) -> &str {
        &self.private_key
    }

    pub fn gen_token(&self, payload: &str) -> String {
        let st = format!("{}|{}", self.server_token, payload);
        let hash = hash_string(&st);

        format!("{}:{}", payload, hash)
    }

    pub fn verify_token(&self, token: &str) -> Result<String> {
        let Some((payload, hash)) = token.rsplit_once(':') else {
            return Err(anyhow!("Invalid token format"));
        };

        let expected_hash = hash_string(&format!("{}|{}", self.server_token, payload));

        if expected_hash != hash {
            return Err(anyhow!("Invalid token"));
        }

        Ok(payload.to_string())
    }

    pub fn gen_key() -> Result<Authenticator> {
        let key = rand::thread_rng().gen::<[u8; 32]>();
        let key = b64_encode(&key);

        let authenticator = Authenticator::new(&key)?;
        Ok(authenticator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_auth() {
        let authenticator = Authenticator::gen_key().unwrap();
        let token = authenticator.gen_token("doc=doc123");
        assert_eq!(authenticator.verify_token(&token).unwrap(), "doc=doc123");

        let token2 = format!("{}:{}", "doc=doc123", hash_string("foobar"));
        assert!(authenticator.verify_token(&token2).is_err());
    }
}
