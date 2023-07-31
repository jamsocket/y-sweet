use crate::{config::Configuration, error::Error, r2_store::R2Store, BUCKET};
use std::sync::Arc;
use y_sweet_server_core::{auth::Authenticator, store::Store};

pub struct ServerContext {
    pub config: Configuration,

    store: Arc<Box<dyn Store>>,
    auth: Option<Authenticator>,
}

impl ServerContext {
    pub fn new(config: Configuration, env: &worker::Env) -> Self {
        let bucket = env.bucket(BUCKET).unwrap();
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
}
