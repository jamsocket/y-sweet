use anyhow::Result;
use worker::{Env, RouteContext};
use y_serve_core::auth::Authenticator;

const AUTH_KEY: &str = "AUTH_KEY";
const USE_HTTPS: &str = "USE_HTTPS";

pub struct Configuration {
    pub auth: Option<Authenticator>,
    pub use_https: bool,
}

pub trait Get {
    fn get(&self, key: &str) -> Option<String>;
}

impl Get for &RouteContext<()> {
    fn get(&self, key: &str) -> Option<String> {
        self.var(key).ok().map(|s| s.to_string())
    }
}

impl Get for &Env {
    fn get(&self, key: &str) -> Option<String> {
        self.var(key).ok().map(|s| s.to_string())
    }
}

impl Configuration {
    fn new(auth_key: Option<String>, use_https: bool) -> Result<Self> {
        let auth = if let Some(auth_key) = auth_key {
            Some(Authenticator::new(&auth_key)?)
        } else {
            None
        };

        Ok(Self { auth, use_https })
    }

    pub fn from<T: Get>(ctx: T) -> Result<Self> {
        let auth_key = ctx.get(AUTH_KEY);
        let use_https = ctx.get(USE_HTTPS).map(|s| s != "false").unwrap_or(false);
        Configuration::new(auth_key, use_https)
    }
}
