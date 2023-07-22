use crate::r2_store::R2Store;
use std::sync::Arc;
use worker::{durable_object, event, Env, Request, Response, Result, RouteContext, Router};
use y_serve_core::{api_types::NewDocResponse, doc_sync::DocWithSyncKv, store::Store};

mod r2_store;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    let response = router
        .get("/", |_, _| Response::ok("Hello world!"))
        .post_async("/doc/new", new_doc)
        .run(req, env)
        .await?;

    Ok(response)
}

async fn new_doc(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let doc_id = nanoid::nanoid!();
    let bucket = ctx.env.bucket("Y_SERVE_DATA").unwrap();
    let store = R2Store::new(bucket);
    let store: Arc<Box<dyn Store>> = Arc::new(Box::new(store));
    let dwskv = DocWithSyncKv::new(&doc_id, store, || {}).await.unwrap();

    dwskv.sync_kv().persist().await.unwrap();

    let response = NewDocResponse { doc_id };

    Response::ok(serde_json::to_string(&response).unwrap())
}

#[durable_object]
pub struct YServe {}

#[durable_object]
impl DurableObject for YServe {
    fn new(state: State, _env: Env) -> Self {
        Self {}
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let url = req.url()?;
        let (_, path) = url.path().rsplit_once('/').unwrap_or_default();
        let method = req.method();
        match (method, path) {
            _ => Response::error("Document command not found", 404),
        }
    }
}
