use crate::r2_store::R2Store;
use config::Configuration;
use std::sync::Arc;
use worker::{event, Date, Env, Request, Response, Result, RouteContext, Router};
use worker_sys::console_log;
use y_sweet_server_core::{
    api_types::{AuthDocResponse, NewDocResponse},
    doc_sync::DocWithSyncKv,
    store::Store,
};

pub mod config;
pub mod durable_object;
pub mod r2_store;
pub mod threadless;

const BUCKET: &str = "Y_SWEET_DATA";
const DURABLE_OBJECT: &str = "Y_SWEET";

fn get_time_millis_since_epoch() -> u64 {
    let now = Date::now();
    now.as_millis()
}

pub fn router() -> Router<'static, ()> {
    let router = Router::new();

    router
        .get("/", |_, _| Response::ok("Hello world!"))
        .post_async("/doc/new", new_doc)
        .post_async("/doc/:doc_id/auth", auth_doc)
        .get_async("/doc/ws/:doc_id", forward_to_durable_object)
}

#[cfg(feature = "fetch-event")]
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let response = router().run(req, env).await?;

    Ok(response)
}

fn check_server_token(req: &Request, config: &Configuration) -> Result<()> {
    if let Some(auth) = &config.auth {
        let auth_header = req.headers().get("Authorization")?;
        let auth_header_val = auth_header.as_deref().ok_or_else(|| {
            worker::Error::JsError("No Authorization header provided.".to_string())
        })?;

        if auth.server_token() != &auth_header_val[7..] {
            console_log!(
                "auth header '{}' '{}'",
                &auth_header_val[7..],
                auth.server_token()
            );
            return Err(worker::Error::JsError(
                "Invalid Authorization header.".to_string(),
            ));
        }
    }
    Ok(())
}

async fn new_doc(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let config = Configuration::from(&ctx.env)
        .map_err(|e| worker::Error::JsError(format!("Token error: {:?}", e)))?;
    check_server_token(&req, &config)?;

    let doc_id = nanoid::nanoid!();
    let bucket = ctx.env.bucket(BUCKET).unwrap();
    let store = R2Store::new(bucket);
    let store: Arc<Box<dyn Store>> = Arc::new(Box::new(store));
    let dwskv = DocWithSyncKv::new(&doc_id, store, || {}).await.unwrap();

    dwskv.sync_kv().persist().await.unwrap();

    let response = NewDocResponse { doc_id };

    Response::from_json(&response)
}

async fn auth_doc(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let config = Configuration::from(&ctx.env).unwrap();
    check_server_token(&req, &config)?;

    let host = req
        .headers()
        .get("Host")?
        .ok_or_else(|| worker::Error::JsError("No Host header provided.".to_string()))?;

    let doc_id = ctx.param("doc_id").unwrap();

    let bucket = ctx.env.bucket(BUCKET).unwrap();
    let store = R2Store::new(bucket);
    if !store.exists(&format!("{doc_id}/data.bin")).await.unwrap() {
        return Ok(Response::ok(format!("Doc '{doc_id}' does not exist."))?.with_status(404));
    }

    let token = config
        .auth
        .map(|auth| auth.gen_doc_token(doc_id, get_time_millis_since_epoch()));

    let schema = if config.use_https { "wss" } else { "ws" };
    let base_url = format!("{schema}://{host}/doc/ws");
    Response::from_json(&AuthDocResponse {
        base_url,
        doc_id: doc_id.to_string(),
        token,
    })
}

async fn forward_to_durable_object(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let doc_id = ctx.param("doc_id").unwrap();
    let durable_object = ctx.env.durable_object(DURABLE_OBJECT)?;
    let stub = durable_object.id_from_name(doc_id)?.get_stub()?;
    stub.fetch_with_request(req).await
}

struct DocIdPair {
    id: String,
    doc: DocWithSyncKv,
}
