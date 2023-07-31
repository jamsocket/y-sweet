use crate::{config::Configuration, error::Error, r2_store::R2Store};
use std::sync::Arc;
use worker::{Env, Request};
use y_sweet_server_core::{auth::Authenticator, store::Store};

const CONTEXT_HEADER: &str = "X-Y-Sweet-Context";

pub struct ServerContext {
    pub config: Configuration,

    store: Arc<Box<dyn Store>>,
    auth: Option<Authenticator>,
}

impl ServerContext {
    pub fn new(config: Configuration, env: &Env) -> Self {
        let bucket = env.bucket(&config.bucket).unwrap();
        let store = R2Store::new(bucket);
        let store: Arc<Box<dyn Store>> = Arc::new(Box::new(store));

        Self {
            config,
            store,
            auth: None,
        }
    }

    pub fn auth(&mut self) -> Result<Option<&Authenticator>, Error> {
        if self.auth.is_none() {
            let auth_key = {
                let Some(auth_key) = &self.config.auth_key else {
                    return Ok(None)
                };
                auth_key.to_owned()
            };

            self.auth = Some(Authenticator::new(&auth_key).map_err(|_| Error::ConfigurationError)?);
        }

        Ok(Some(self.auth.as_ref().unwrap()))
    }

    pub fn store(&mut self) -> Arc<Box<dyn Store>> {
        self.store.clone()
    }

    pub fn install_on_request(&self, req: &mut Request) -> worker::Result<()> {
        req.headers_mut()?
            .append(CONTEXT_HEADER, &serde_json::to_string(&self.config)?)?;
        Ok(())
    }

    pub fn from_request(req: &Request, env: &Env) -> Result<Self, Error> {
        let context_header = req
            .headers()
            .get(CONTEXT_HEADER)
            .map_err(|_| Error::InternalError)?;
        let context_header_val = context_header.as_deref().ok_or(Error::InternalError)?;

        let config: Configuration =
            serde_json::from_str(context_header_val).map_err(|_| Error::InternalError)?;

        Ok(Self::new(config, env))
    }
}
