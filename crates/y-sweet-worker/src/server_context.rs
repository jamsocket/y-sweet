use crate::{config::Configuration, error::Error, r2_store::R2Store, s3_store::S3Store};
use std::sync::Arc;
use worker::{Env, Request};
use y_sweet_core::{auth::Authenticator, store::Store};

const CONTEXT_HEADER: &str = "X-Y-Sweet-Context";
const ROUTE_HEADER: &str = "X-Y-Sweet-Route";

pub struct ServerContext {
    pub config: Configuration,

    store: Arc<Box<dyn Store>>,
    auth: Option<Authenticator>,
}

impl ServerContext {
    pub fn new(config: Configuration, env: &Env) -> Self {
        let bucket = env.bucket(&config.bucket).unwrap();
        let store: Box<dyn Store> = if let Some(ref s3) = config.s3_store_config.as_ref() {
            Box::new(S3Store::new(
                s3.region.clone(),
                s3.bucket.clone(),
                s3.bucket_prefix.clone(),
                s3.key.clone(),
                s3.secret.clone(),
            ))
        } else {
            Box::new(R2Store::new(bucket, config.bucket_prefix.clone()))
        };
        let store: Arc<Box<dyn Store>> = Arc::new(store);

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
                    return Ok(None);
                };
                auth_key.to_owned()
            };

            let mut auth =
                Authenticator::new(&auth_key).map_err(|_| Error::ConfigurationError {
                    field: "auth_key".to_string(),
                    value: auth_key,
                })?;

            if let Some(auth_key_id) = &self.config.auth_key_id {
                auth = auth.with_key_id(auth_key_id.clone());
            }

            self.auth = Some(auth);
        }

        Ok(Some(self.auth.as_ref().unwrap()))
    }

    pub fn store(&mut self) -> Arc<Box<dyn Store>> {
        self.store.clone()
    }

    pub fn install_on_request(&self, req: &mut Request) -> worker::Result<()> {
        let path = req.path();
        req.headers_mut()?
            .append(CONTEXT_HEADER, &serde_json::to_string(&self.config)?)?;
        req.headers_mut()?.append(ROUTE_HEADER, &path)?;
        Ok(())
    }

    /// When a request is forwarded to a durable object, it loses any local mutation applied to its path.
    /// To avoid this, in install_on_request we take the path and store it in a header. Then, this function
    /// can be called on a request to extract the path from the header and apply it back to the request.
    pub fn reconstruct_request(req: &Request) -> worker::Result<Request> {
        let route_header = req
            .headers()
            .get(ROUTE_HEADER)
            .map_err(|_| worker::Error::RustError("Missing route header.".to_string()))?;
        let route_header_val = route_header
            .as_deref()
            .ok_or_else(|| worker::Error::RustError("Couldn't deref route header.".to_string()))?;

        let mut req = req.clone_mut()?;
        *req.path_mut()? = route_header_val.to_string();

        Ok(req)
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
