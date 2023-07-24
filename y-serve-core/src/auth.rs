use anyhow::{anyhow, Result};
use pasetors::{
    claims::{Claims, ClaimsValidationRules},
    keys::{Generate, SymmetricKey},
    local::{decrypt, encrypt},
    paserk::FormatAsPaserk,
    token::UntrustedToken,
    version4::V4,
    Local,
};
use serde_json::Value;
use sha2::{Sha256, Digest};

pub struct Authenticator {
    paseto: SymmetricKey<V4>,
    server_token: String,
}

fn hash_string(str: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(str);
    let result = hasher.finalize();
    format!("{:x}", result)
}

impl Authenticator {
    pub fn new(key: &str) -> Result<Self> {
        let paseto = SymmetricKey::<V4>::try_from(key)?;
        let server_token = hash_string(key);

        Ok(Self { paseto, server_token })
    }

    pub fn server_token(&self) -> &str {
        &self.server_token
    }

    pub fn paseto_token(&self) -> String {
        let mut paserk = String::new();
        self.paseto.fmt(&mut paserk).unwrap();
        paserk
    }

    pub fn gen_token(&self, doc_id: &str) -> Result<String> {
        let mut claims = Claims::new()?;
        claims.add_additional("doc", doc_id)?;

        Ok(encrypt(&self.paseto, &claims, None, None)?)
    }

    pub fn verify_token(&self, token: &str, doc_id: &str) -> Result<bool> {
        let validation_rules = ClaimsValidationRules::new();
        let untrusted_token = UntrustedToken::<Local, V4>::try_from(token)?;
        let trusted_token = decrypt(
            &self.paseto,
            &untrusted_token,
            &validation_rules,
            None,
            None,
        )?;
        let claims = trusted_token
            .payload_claims()
            .ok_or_else(|| anyhow!("No claims"))?;
        let claim_doc_id = claims.get_claim("doc").ok_or_else(|| anyhow!("No doc"))?;
        if let Value::String(value) = claim_doc_id {
            Ok(value == doc_id)
        } else {
            Ok(false)
        }
    }

    pub fn gen_key() -> Result<Authenticator> {
        let key = SymmetricKey::<V4>::generate()?;
        let mut paserk = String::new();
        key.fmt(&mut paserk)?;

        let authenticator = Authenticator::new(&paserk)?;
        Ok(authenticator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_auth() {
        let authenticator = Authenticator::gen_key().unwrap();
        let token = authenticator.gen_token("doc123").unwrap();
        assert!(authenticator.verify_token(&token, "doc123").unwrap());
        assert!(!authenticator.verify_token(&token, "doc1234").unwrap());
    }
}
