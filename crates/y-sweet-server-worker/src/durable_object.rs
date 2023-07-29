use crate::{r2_store::R2Store, threadless::Threadless, DocIdPair, BUCKET};
use futures::StreamExt;
use std::sync::Arc;
use worker::{
    durable_object, Env, Request, Response, Result, RouteContext, Router, State, WebSocketPair,
};
#[allow(unused)]
use worker_sys::console_log;
use y_sweet_server_core::{doc_connection::DocConnection, doc_sync::DocWithSyncKv, store::Store};

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
