use crate::r2_store::R2Store;
use config::Configuration;
use error::{Error, IntoResponse};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use threadless::Threadless;
use worker::{
    durable_object, event, Date, Env, Request, Response, Result, RouteContext, Router, State,
    WebSocketPair,
};
#[allow(unused)]
use worker_sys::console_log;
use y_sweet_server_core::{
    api_types::{AuthDocResponse, NewDocResponse},
    doc_connection::DocConnection,
    doc_sync::DocWithSyncKv,
    store::Store,
};

mod config;
mod error;
mod r2_store;
mod threadless;

const BUCKET: &str = "Y_SWEET_DATA";
const DURABLE_OBJECT: &str = "Y_SWEET";

fn get_time_millis_since_epoch() -> u64 {
    let now = Date::now();
    now.as_millis()
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    let response = router
        .get("/", |_, _| Response::ok("Hello world!"))
        .post_async("/doc/new", new_doc_handler)
        .post_async("/doc/:doc_id/auth", auth_doc_handler)
        .get_async("/doc/ws/:doc_id", forward_to_durable_object)
        .run(req, env)
        .await?;

    Ok(response)
}

fn check_server_token(req: &Request, config: &Configuration) -> std::result::Result<(), Error> {
    if let Some(auth) = &config.auth {
        let auth_header = req
            .headers()
            .get("Authorization")
            .map_err(|_| Error::ExpectedAuthHeader)?;
        let auth_header_val = auth_header.as_deref().ok_or(Error::ExpectedAuthHeader)?;

        if auth.server_token() != &auth_header_val[7..] {
            console_log!(
                "auth header '{}' '{}'",
                &auth_header_val[7..],
                auth.server_token()
            );
            return Err(Error::BadAuthHeader);
        }
    }
    Ok(())
}

async fn new_doc_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    new_doc(req, ctx).await.into_response()
}

async fn new_doc(
    req: Request,
    ctx: RouteContext<()>,
) -> std::result::Result<NewDocResponse, Error> {
    let config = Configuration::from(&ctx.env)?;
    check_server_token(&req, &config)?;

    let doc_id = nanoid::nanoid!();
    let bucket = ctx.env.bucket(BUCKET).unwrap();
    let store = R2Store::new(bucket);
    let store: Arc<Box<dyn Store>> = Arc::new(Box::new(store));
    let dwskv = DocWithSyncKv::new(&doc_id, store, || {}).await.unwrap();

    dwskv.sync_kv().persist().await.unwrap();

    let response = NewDocResponse { doc_id };

    Ok(response)
}

async fn auth_doc_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    auth_doc(req, ctx).await.into_response()
}

async fn auth_doc(
    req: Request,
    ctx: RouteContext<()>,
) -> std::result::Result<AuthDocResponse, Error> {
    let config = Configuration::from(&ctx.env).unwrap();
    check_server_token(&req, &config)?;

    let host = req
        .headers()
        .get("Host")
        .map_err(|_| Error::MissingHostHeader)?
        .ok_or_else(|| Error::MissingHostHeader)?;

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

    let token = config
        .auth
        .map(|auth| auth.gen_doc_token(doc_id, get_time_millis_since_epoch()));

    let schema = if config.use_https { "wss" } else { "ws" };
    let base_url = format!("{schema}://{host}/doc/ws");

    Ok(AuthDocResponse {
        base_url,
        doc_id: doc_id.to_string(),
        token,
    })
}

async fn forward_to_durable_object(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let config = Configuration::from(&ctx.env).unwrap();
    let doc_id = ctx.param("doc_id").unwrap();

    if let Some(auth) = config.auth {
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
            let doc = DocWithSyncKv::new(doc_id, store, move || {
                let storage = storage.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    console_log!("Setting alarm.");
                    storage.0.set_alarm(10_000).await.unwrap();
                });
            })
            .await
            .unwrap();

            self.lasy_doc = Some(DocIdPair {
                doc,
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
