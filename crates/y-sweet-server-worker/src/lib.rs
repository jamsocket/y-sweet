use error::{Error, IntoResponse};
use server_context::ServerContext;
use std::collections::HashMap;
use worker::{event, Date, Env, Request, Response, Result, RouteContext, Router};
use worker_sys::console_log;
use y_sweet_server_core::{
    api_types::{AuthDocResponse, NewDocResponse},
    auth::Authenticator,
    doc_sync::DocWithSyncKv,
};

pub mod config;
pub mod durable_object;
pub mod error;
pub mod r2_store;
pub mod server_context;
pub mod threadless;

const BUCKET: &str = "Y_SWEET_DATA";
const DURABLE_OBJECT: &str = "Y_SWEET";

fn get_time_millis_since_epoch() -> u64 {
    let now = Date::now();
    now.as_millis()
}

pub fn router(
    context: ServerContext,
) -> std::result::Result<Router<'static, ServerContext>, Error> {
    Ok(Router::with_data(context)
        .get("/", |_, _| Response::ok("Hello world!"))
        .post_async("/doc/new", new_doc_handler)
        .post_async("/doc/:doc_id/auth", auth_doc_handler)
        .get_async("/doc/ws/:doc_id", forward_to_durable_object))
}

#[cfg(feature = "fetch-event")]
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    use config::Configuration;

    console_error_panic_hook::set_once();

    let configuration = Configuration::from(&env);
    let context = ServerContext::new(configuration, &env);

    let router = router(context);
    let router = match router {
        Ok(router) => router,
        Err(err) => return err.into(),
    };

    router.run(req, env).await
}

fn check_server_token(
    req: &Request,
    auth: Option<&Authenticator>,
) -> std::result::Result<(), Error> {
    if let Some(auth) = auth {
        let auth_header = req
            .headers()
            .get("Authorization")
            .map_err(|_| Error::ExpectedAuthHeader)?;
        let auth_header_val = auth_header.as_deref().ok_or(Error::ExpectedAuthHeader)?;

        console_log!("auth_header_val: {:?}", auth_header_val);
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

async fn new_doc_handler(req: Request, ctx: RouteContext<ServerContext>) -> Result<Response> {
    new_doc(req, ctx).await.into_response()
}

async fn new_doc(
    req: Request,
    mut ctx: RouteContext<ServerContext>,
) -> std::result::Result<NewDocResponse, Error> {
    check_server_token(&req, ctx.data.auth()?)?;

    let doc_id = nanoid::nanoid!();
    let store = ctx.data.store();
    let dwskv = DocWithSyncKv::new(&doc_id, store, || {}).await.unwrap();

    dwskv.sync_kv().persist().await.unwrap();

    let response = NewDocResponse { doc_id };

    Ok(response)
}

async fn auth_doc_handler(req: Request, ctx: RouteContext<ServerContext>) -> Result<Response> {
    auth_doc(req, ctx).await.into_response()
}

async fn auth_doc(
    req: Request,
    mut ctx: RouteContext<ServerContext>,
) -> std::result::Result<AuthDocResponse, Error> {
    check_server_token(&req, ctx.data.auth()?)?;

    let host = req
        .headers()
        .get("Host")
        .map_err(|_| Error::MissingHostHeader)?
        .ok_or(Error::MissingHostHeader)?;

    let doc_id = ctx.param("doc_id").unwrap().to_string();

    let store = ctx.data.store();
    if !store
        .exists(&format!("{doc_id}/data.bin"))
        .await
        .map_err(|_| Error::UpstreamConnectionError)?
    {
        return Err(Error::NoSuchDocument);
    }

    let token = ctx
        .data
        .auth()?
        .map(|auth| auth.gen_doc_token(&doc_id, get_time_millis_since_epoch()));

    let schema = if ctx.data.config.use_https {
        "wss"
    } else {
        "ws"
    };
    let base_url = format!("{schema}://{host}/doc/ws");

    Ok(AuthDocResponse {
        base_url,
        doc_id: doc_id.to_string(),
        token,
    })
}

async fn forward_to_durable_object(
    req: Request,
    mut ctx: RouteContext<ServerContext>,
) -> Result<Response> {
    let doc_id = ctx.param("doc_id").unwrap().to_string();

    if let Some(auth) = ctx.data.auth().unwrap() {
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
            .verify_doc_token(token, &doc_id, get_time_millis_since_epoch())
            .map_err(|_| Error::BadClientAuthHeader);
        if let Err(e) = result {
            return e.into();
        }
    }

    let durable_object = ctx.env.durable_object(DURABLE_OBJECT)?;
    let stub = durable_object.id_from_name(&doc_id)?.get_stub()?;
    stub.fetch_with_request(req).await
}

struct DocIdPair {
    id: String,
    doc: DocWithSyncKv,
}
