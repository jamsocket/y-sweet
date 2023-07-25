use crate::r2_store::R2Store;
use config::Configuration;
use futures::StreamExt;
use std::sync::Arc;
use threadless::Threadless;
use worker::{
    durable_object, event, Env, Request, Response, Result, RouteContext, Router, State,
    WebSocketPair, Date,
};
#[allow(unused)]
use worker_sys::console_log;
use y_serve_core::{
    api_types::{AuthDocResponse, NewDocResponse},
    doc_connection::DocConnection,
    doc_sync::DocWithSyncKv,
    store::Store,
};

mod config;
mod r2_store;
mod threadless;

const BUCKET: &str = "Y_SERVE_DATA";
const DURABLE_OBJECT: &str = "Y_SERVE";

fn get_time_millis_since_epoch() -> u64 {
    let now = Date::now();
    now.as_millis() as u64
}



#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    let response = router
        .get("/", |_, _| Response::ok("Hello world!"))
        .post_async("/doc/new", new_doc)
        .post_async("/doc/:doc_id/auth", auth_doc)
        .get_async("/doc/ws/:doc_id", forward_to_durable_object)
        .run(req, env)
        .await?;

    Ok(response)
}

fn check_server_token(req: &Request, config: &Configuration) -> Result<()> {
    if let Some(auth) = &config.auth {
        let auth_header = req.headers().get("Authorization")?;
        let auth_header_val = auth_header.as_deref().ok_or_else(|| {
            worker::Error::JsError("No Authorization header provided.".to_string())
        })?;

        if auth.server_token() != &auth_header_val[7..] {
            console_log!("auth header '{}' '{}'", &auth_header_val[7..], auth.server_token());
            return Err(worker::Error::JsError(
                "Invalid Authorization header.".to_string(),
            ));
        }
    }
    Ok(())
}

async fn new_doc(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let config = Configuration::from(&ctx.env).map_err(|e| worker::Error::JsError(format!("Token error: {:?}", e)))?;
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

    // TODO: verify that the doc exists

    let token = if let Some(auth) = config.auth {
        Some(auth.gen_doc_token(&doc_id, get_time_millis_since_epoch()))
    } else {
        None
    };

    let base_url = format!("ws://{}/doc/ws", host);
    Response::from_json(&AuthDocResponse {
        base_url,
        doc_id: doc_id.to_string(),
        token,
    })
}

async fn forward_to_durable_object(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let doc_id = ctx.param("doc_id").unwrap();
    let durable_object = ctx.env.durable_object(DURABLE_OBJECT)?;
    let stub = durable_object.id_from_name(&doc_id)?.get_stub()?;
    stub.fetch_with_request(req).await
}

struct DocIdPair {
    id: String,
    doc: DocWithSyncKv,
}

#[durable_object]
pub struct YServe {
    env: Env,
    lasy_doc: Option<DocIdPair>,
    state: State,
}

impl YServe {
    /// We need to lazily create the doc because the constructor is non-async.
    pub async fn get_doc(&mut self, doc_id: &str) -> Result<&mut DocWithSyncKv> {
        let storage = Arc::new(self.state.storage());

        if self.lasy_doc.is_none() {
            let bucket = self.env.bucket(BUCKET).unwrap();
            let store = R2Store::new(bucket);
            let store: Arc<Box<dyn Store>> = Arc::new(Box::new(store));
            let storage = Threadless(storage);
            let doc = DocWithSyncKv::new(&doc_id, store, move || {
                let storage = storage.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    console_log!("Setting alarm.");
                    storage.0.set_alarm(10_000).await.unwrap();
                });
            })
            .await
            .unwrap();

            self.lasy_doc = Some(DocIdPair {
                doc: doc,
                id: doc_id.to_owned(),
            });
            self.lasy_doc
                .as_mut()
                .unwrap()
                .doc
                .sync_kv()
                .persist()
                .await
                .unwrap();
        }

        Ok(&mut self.lasy_doc.as_mut().unwrap().doc)
    }
}

#[durable_object]
impl DurableObject for YServe {
    fn new(state: State, env: Env) -> Self {
        Self {
            env,
            state,
            lasy_doc: None,
        }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let env: Env = self.env.clone().into();

        Router::with_data(self)
            .get_async("/doc/ws/:doc_id", websocket_connect)
            .run(req, env)
            .await
    }

    async fn alarm(&mut self) -> Result<Response> {
        console_log!("Alarm!");
        let DocIdPair { id, doc } = self.lasy_doc.as_ref().unwrap();
        doc.sync_kv().persist().await.unwrap();
        console_log!("Persisted. {}", id);
        Response::ok("ok")
    }
}

async fn websocket_connect(_req: Request, ctx: RouteContext<&mut YServe>) -> Result<Response> {
    let doc_id = ctx.param("doc_id").unwrap().to_owned();
    let WebSocketPair { client, server } = WebSocketPair::new()?;
    server.accept()?;

    let awareness = ctx.data.get_doc(&doc_id).await.unwrap().awareness();

    let connection = {
        let server = server.clone();
        DocConnection::new(awareness, move |bytes| {
            server.send_with_bytes(bytes).unwrap();
        })
    };

    wasm_bindgen_futures::spawn_local(async move {
        let mut events = server.events().unwrap();

        while let Some(event) = events.next().await {
            match event.unwrap() {
                worker::WebsocketEvent::Message(message) => {
                    if let Some(bytes) = message.bytes() {
                        connection.send(&bytes).await.unwrap();
                    } else {
                        server
                            .send_with_str("Received unexpected text message.")
                            .unwrap()
                    }
                }
                worker::WebsocketEvent::Close(_) => {
                    break;
                }
            }
        }
    });

    let resp = Response::from_websocket(client)?;
    Ok(resp)
}
