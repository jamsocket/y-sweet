use crate::r2_store::R2Store;
use config::Configuration;
use error::{Error, IntoResponse};
use std::collections::HashMap;
use std::sync::Arc;
use worker::{event, Date, Env, Request, Response, Result, RouteContext, Router};
use y_sweet_server_core::{
    api_types::{AuthDocResponse, NewDocResponse},
    doc_sync::DocWithSyncKv,
    store::Store,
};

pub mod config;
pub mod durable_object;
pub mod error;
pub mod r2_store;
pub mod threadless;

const BUCKET: &str = "Y_SWEET_DATA";
const DURABLE_OBJECT: &str = "Y_SWEET";

fn get_time_millis_since_epoch() -> u64 {
    let now = Date::now();
    now.as_millis()
}

pub fn router(env: &Env) -> std::result::Result<Router<'static, Configuration>, Error> {
    let Ok(config) = Configuration::try_from(env) else {
        return Err(Error::ConfigurationError)
    };

    Ok(Router::with_data(config)
        .get("/", |_, _| Response::ok("Hello world!"))
        .post_async("/doc/new", new_doc_handler)
        .post_async("/doc/:doc_id/auth", auth_doc_handler)
        .get_async("/doc/ws/:doc_id", forward_to_durable_object))
}

#[cfg(feature = "fetch-event")]
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = router(&env);
    let router = match router {
        Ok(router) => router,
        Err(err) => return err.into(),
    };

    router.run(req, env).await
}

fn check_server_token(req: &Request, config: &Configuration) -> std::result::Result<(), Error> {
    if let Some(auth) = &config.auth {
        let auth_header = req
            .headers()
            .get("Authorization")
            .map_err(|_| Error::ExpectedAuthHeader)?;
        let auth_header_val = auth_header.as_deref().ok_or(Error::ExpectedAuthHeader)?;

        if let Some(token) = auth_header_val.strip_prefix("Bearer ") {
            if auth.server_token() != token {
                return Err(Error::BadAuthHeader);
            }
        } else {
            return Err(Error::BadAuthHeader);
        }
    }
    Ok(())
}

async fn new_doc_handler(req: Request, ctx: RouteContext<Configuration>) -> Result<Response> {
    new_doc(req, ctx).await.into_response()
}

async fn new_doc(
    req: Request,
    ctx: RouteContext<Configuration>,
) -> std::result::Result<NewDocResponse, Error> {
    check_server_token(&req, &ctx.data)?;

    let doc_id = nanoid::nanoid!();
    let bucket = ctx.env.bucket(BUCKET).unwrap();
    let store = R2Store::new(bucket);
    let store: Arc<Box<dyn Store>> = Arc::new(Box::new(store));
    let dwskv = DocWithSyncKv::new(&doc_id, store, || {}).await.unwrap();

    dwskv.sync_kv().persist().await.unwrap();

    let response = NewDocResponse { doc_id };

    Ok(response)
}

async fn auth_doc_handler(req: Request, ctx: RouteContext<Configuration>) -> Result<Response> {
    auth_doc(req, ctx).await.into_response()
}

async fn auth_doc(
    req: Request,
    ctx: RouteContext<Configuration>,
) -> std::result::Result<AuthDocResponse, Error> {
    check_server_token(&req, &ctx.data)?;

    let host = req
        .headers()
        .get("Host")
        .map_err(|_| Error::MissingHostHeader)?
        .ok_or(Error::MissingHostHeader)?;

    let doc_id = ctx.param("doc_id").unwrap();

    let bucket = ctx.env.bucket(BUCKET).unwrap();
    let store = R2Store::new(bucket);
    if !store
        .exists(&format!("{doc_id}/data.bin"))
        .await
        .map_err(|_| Error::UpstreamConnectionError)?
    {
        return Err(Error::NoSuchDocument);
    }

    let token = ctx
        .data
        .auth
        .as_ref()
        .map(|auth| auth.gen_doc_token(doc_id, get_time_millis_since_epoch()));

    let schema = if ctx.data.use_https { "wss" } else { "ws" };
    let base_url = format!("{schema}://{host}/doc/ws");

    Ok(AuthDocResponse {
        base_url,
        doc_id: doc_id.to_string(),
        token,
    })
}

async fn forward_to_durable_object(
    req: Request,
    ctx: RouteContext<Configuration>,
) -> Result<Response> {
    let doc_id = ctx.param("doc_id").unwrap();

    if let Some(auth) = &ctx.data.auth {
        // Read query params.
        let url = req.url()?;
        let query: HashMap<String, String> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let result = query.get("token").ok_or(Error::ExpectedClientAuthHeader);
        let token = match result {
            Ok(token) => token,
            Err(e) => return e.into(),
        };
        let result = auth
            .verify_doc_token(token, doc_id, get_time_millis_since_epoch())
            .map_err(|_| Error::BadClientAuthHeader);
        if let Err(e) = result {
            return e.into();
        }
    }

    let durable_object = ctx.env.durable_object(DURABLE_OBJECT)?;
    let stub = durable_object.id_from_name(doc_id)?.get_stub()?;
    stub.fetch_with_request(req).await
}

struct DocIdPair {
    id: String,
    doc: DocWithSyncKv,
}
