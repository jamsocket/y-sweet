use crate::error::Error;
use worker::Env;
use y_sweet_server_core::auth::Authenticator;

const AUTH_KEY: &str = "AUTH_KEY";
const USE_HTTPS: &str = "USE_HTTPS";

pub struct Configuration {
    pub auth: Option<Authenticator>,
    pub use_https: bool,
}

impl Configuration {
    fn new(auth_key: Option<String>, use_https: bool) -> Result<Self, Error> {
        let auth = if let Some(auth_key) = auth_key {
            Some(Authenticator::new(&auth_key).map_err(|_| Error::ConfigurationError)?)
        } else {
            None
        };

        Ok(Self { auth, use_https })
    }
}

impl TryFrom<&Env> for Configuration {
    type Error = Error;

    fn try_from(env: &Env) -> Result<Self, Error> {
        let auth_key = env.var(AUTH_KEY).map(|s| s.to_string()).ok();
        let use_https = env
            .var(USE_HTTPS)
            .map(|s| s.to_string() != "false")
            .unwrap_or(false);
        Configuration::new(auth_key, use_https)
    }
}
