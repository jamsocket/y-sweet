use crate::metrics::Metrics;
use anyhow::{anyhow, Result};
use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket},
        DefaultBodyLimit, Path, Query, Request, State, WebSocketUpgrade,
    },
    http::{
        header::{HeaderMap, HeaderName},
        StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::typed_header::TypedHeader;
use dashmap::{mapref::one::MappedRef, DashMap};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{channel, Receiver},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{span, Instrument, Level};
use url::Url;
use y_sweet_core::{
    api_types::{
        validate_doc_name, AuthDocRequest, Authorization, ClientToken, DocCreationRequest,
        NewDocResponse,
    },
    auth::{Authenticator, ExpirationTimeEpochMillis, DEFAULT_EXPIRATION_SECONDS},
    doc_connection::DocConnection,
    doc_sync::DocWithSyncKv,
    store::Store,
    sync::awareness::Awareness,
    sync_kv::SyncKv,
};

const PLANE_VERIFIED_USER_DATA_HEADER: &str = "x-verified-user-data";

// Every 20 seconds, we send a ping to the client.
const PING_EVERY: Duration = Duration::from_secs(20);
// If we haven't received a pong in the last 40 seconds, we close the connection.
// All modern browsers will respond to websocket pings with a pong message.
const PONG_TIMEOUT: Duration = Duration::from_secs(40);

// Defaults for the periodic mid-session access re-check; overridable via
// Server::new (see main.rs's Y_SWEET_VALIDATE_* CLI/env options).
pub const DEFAULT_VALIDATE_POLL_EVERY: Duration = Duration::from_secs(10);
pub const DEFAULT_VALIDATE_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_VALIDATE_RETRY_DELAY: Duration = Duration::from_millis(500);

fn current_time_epoch_millis() -> u64 {
    let now = std::time::SystemTime::now();
    let duration_since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    duration_since_epoch.as_millis() as u64
}

#[derive(Debug)]
pub struct AppError(StatusCode, anyhow::Error);
impl std::error::Error for AppError {}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, format!("Something went wrong: {}", self.1)).into_response()
    }
}
impl<E> From<(StatusCode, E)> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from((status_code, err): (StatusCode, E)) -> Self {
        Self(status_code, err.into())
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status code: {} {}", self.0, self.1)?;
        Ok(())
    }
}

fn normalize_path_prefix(prefix: String) -> String {
    let trimmed = prefix.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

pub struct Server {
    docs: Arc<DashMap<String, DocWithSyncKv>>,
    doc_worker_tracker: TaskTracker,
    store: Option<Arc<Box<dyn Store>>>,
    checkpoint_freq: Duration,
    authenticator: Option<Authenticator>,
    url_prefix: Option<Url>,
    path_prefix: Option<String>,
    cancellation_token: CancellationToken,
    /// Whether to garbage collect docs that are no longer in use.
    /// Disabled for single-doc mode, since we only have one doc.
    doc_gc: bool,
    max_body_size: Option<usize>,
    /// Whether to skip garbage collection in Yrs documents.
    skip_gc: bool,
    metrics: Metrics,
    backend_url: Option<String>,
    http_client: reqwest::Client,
    validate_poll_every: Duration,
    validate_retry_attempts: u32,
    validate_retry_delay: Duration,
}

impl Server {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        store: Option<Box<dyn Store>>,
        checkpoint_freq: Duration,
        authenticator: Option<Authenticator>,
        url_prefix: Option<Url>,
        path_prefix: Option<String>,
        cancellation_token: CancellationToken,
        doc_gc: bool,
        max_body_size: Option<usize>,
        skip_gc: bool,
        backend_url: Option<String>,
        validate_poll_every: Duration,
        validate_retry_attempts: u32,
        validate_retry_delay: Duration,
    ) -> Result<Self> {
        let docs = Arc::new(DashMap::new());
        let metrics = Metrics::new(docs.clone());
        if backend_url.is_none() {
            tracing::warn!(
                "Y_SWEET_BACKEND_URL is not set. All access-validated requests will be denied."
            );
        }
        Ok(Self {
            docs,
            doc_worker_tracker: TaskTracker::new(),
            store: store.map(Arc::new),
            checkpoint_freq,
            authenticator,
            url_prefix,
            path_prefix: path_prefix
                .map(normalize_path_prefix)
                .filter(|p| !p.is_empty()),
            cancellation_token,
            doc_gc,
            max_body_size,
            skip_gc,
            metrics,
            backend_url,
            http_client: reqwest::Client::new(),
            validate_poll_every,
            validate_retry_attempts,
            validate_retry_delay,
        })
    }

    /// Calls the Xyne backend to check whether `user_id` currently has access
    /// to `doc_id`. Fails closed (denies) if the backend is unconfigured,
    /// unreachable, or returns a non-2xx response.
    async fn validate_user_access(&self, doc_id: &str, user_id: &str) -> bool {
        let Some(backend_url) = &self.backend_url else {
            return false;
        };
        let url = format!("{}/api/ysweet/validate", backend_url.trim_end_matches('/'));
        match self
            .http_client
            .post(&url)
            .json(&serde_json::json!({ "docId": doc_id, "userId": user_id }))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                tracing::error!(?e, doc_id = %doc_id, user_id = %user_id, "Failed to reach backend for access validation");
                false
            }
        }
    }

    /// Like `validate_user_access`, but retries transient failures (network
    /// errors, backend hiccups) up to `VALIDATE_RETRY_ATTEMPTS` times before
    /// treating the check as a real denial. Used by the periodic mid-session
    /// poll, where a brief backend blip shouldn't kick an otherwise-valid user.
    async fn validate_user_access_with_retry(&self, doc_id: &str, user_id: &str) -> bool {
        for attempt in 1..=self.validate_retry_attempts {
            if self.validate_user_access(doc_id, user_id).await {
                return true;
            }
            if attempt < self.validate_retry_attempts {
                tokio::time::sleep(self.validate_retry_delay).await;
            }
        }
        false
    }

    pub async fn doc_exists(&self, doc_id: &str) -> bool {
        if self.docs.contains_key(doc_id) {
            return true;
        }
        if let Some(store) = &self.store {
            store
                .exists(&format!("{}/data.ysweet", doc_id))
                .await
                .unwrap_or_default()
        } else {
            false
        }
    }

    pub async fn create_doc(&self) -> Result<String> {
        let doc_id = nanoid::nanoid!();
        self.load_doc(&doc_id).await?;
        tracing::info!(doc_id=%doc_id, "Created doc");
        Ok(doc_id)
    }

    pub async fn load_doc(&self, doc_id: &str) -> Result<()> {
        let (send, recv) = channel(1024);

        let dwskv = DocWithSyncKv::new(
            doc_id,
            self.store.clone(),
            move || {
                send.try_send(()).unwrap();
            },
            self.skip_gc,
        )
        .await?;

        dwskv
            .sync_kv()
            .persist()
            .await
            .map_err(|e| anyhow!("Error persisting: {:?}", e))?;

        {
            let sync_kv = dwskv.sync_kv();
            let checkpoint_freq = self.checkpoint_freq;
            let doc_id = doc_id.to_string();
            let cancellation_token = self.cancellation_token.clone();

            let metrics = self.metrics.clone();

            // Spawn a task to save the document to the store when it changes.
            self.doc_worker_tracker.spawn(
                Self::doc_persistence_worker(
                    recv,
                    sync_kv,
                    checkpoint_freq,
                    doc_id.clone(),
                    cancellation_token.clone(),
                    metrics.clone(),
                )
                .instrument(span!(Level::INFO, "save_loop", doc_id=%doc_id)),
            );

            if self.doc_gc {
                self.doc_worker_tracker.spawn(
                    Self::doc_gc_worker(
                        self.docs.clone(),
                        doc_id.clone(),
                        checkpoint_freq,
                        cancellation_token,
                        metrics,
                    )
                    .instrument(span!(Level::INFO, "gc_loop", doc_id=%doc_id)),
                );
            }
        }

        self.docs.insert(doc_id.to_string(), dwskv);
        Ok(())
    }

    async fn doc_gc_worker(
        docs: Arc<DashMap<String, DocWithSyncKv>>,
        doc_id: String,
        checkpoint_freq: Duration,
        cancellation_token: CancellationToken,
        metrics: Metrics,
    ) {
        let mut checkpoints_without_refs = 0;

        loop {
            tokio::select! {
                _ = tokio::time::sleep(checkpoint_freq) => {
                    if let Some(doc) = docs.get(&doc_id) {
                        let awareness = Arc::downgrade(&doc.awareness());
                        if awareness.strong_count() > 1 {
                            checkpoints_without_refs = 0;
                            tracing::debug!("doc is still alive - it has {} references", awareness.strong_count());
                        } else {
                            checkpoints_without_refs += 1;
                            tracing::info!("doc has only one reference, candidate for GC. checkpoints_without_refs: {}", checkpoints_without_refs);
                        }
                    } else {
                        break;
                    }

                    if checkpoints_without_refs >= 2 {
                        tracing::info!("GCing doc");
                        if let Some(doc) = docs.get(&doc_id) {
                            doc.sync_kv().shutdown();
                        }

                        docs.remove(&doc_id);
                        metrics.documents_gc.add(1, &[]);
                        break;
                    }
                }
                _ = cancellation_token.cancelled() => {
                    break;
                }
            };
        }
        tracing::info!("Exiting gc_loop");
    }

    async fn doc_persistence_worker(
        mut recv: Receiver<()>,
        sync_kv: Arc<SyncKv>,
        checkpoint_freq: Duration,
        doc_id: String,
        cancellation_token: CancellationToken,
        metrics: Metrics,
    ) {
        let mut last_save = std::time::Instant::now();

        loop {
            let is_done = tokio::select! {
                v = recv.recv() => v.is_none(),
                _ = cancellation_token.cancelled() => true,
                _ = tokio::time::sleep(checkpoint_freq) => {
                    sync_kv.is_shutdown()
                }
            };

            tracing::info!("Received signal. done: {}", is_done);
            let now = std::time::Instant::now();
            if !is_done && now - last_save < checkpoint_freq {
                let sleep = tokio::time::sleep(checkpoint_freq - (now - last_save));
                tokio::pin!(sleep);
                tracing::info!("Throttling.");

                loop {
                    tokio::select! {
                        _ = &mut sleep => {
                            break;
                        }
                        v = recv.recv() => {
                            tracing::info!("Received dirty while throttling.");
                            if v.is_none() {
                                break;
                            }
                        }
                        _ = cancellation_token.cancelled() => {
                            tracing::info!("Received cancellation while throttling.");
                            break;
                        }

                    }
                    tracing::info!("Done throttling.");
                }
            }
            tracing::info!("Persisting.");
            let persist_start = std::time::Instant::now();
            if let Err(e) = sync_kv.persist().await {
                tracing::error!(doc_id = %doc_id, ?e, "Error persisting.");
                metrics.persistence_errors.add(1, &[]);
            } else {
                let elapsed_ms = persist_start.elapsed().as_secs_f64() * 1000.0;
                metrics.persistence_latency_ms.record(elapsed_ms, &[]);
                metrics.persistence_ops.add(1, &[]);
                tracing::info!("Done persisting.");
            }
            last_save = std::time::Instant::now();

            if is_done {
                break;
            }
        }
        tracing::info!("Terminating loop for {}", doc_id);
    }

    pub async fn get_or_create_doc(
        &self,
        doc_id: &str,
    ) -> Result<MappedRef<'_, String, DocWithSyncKv, DocWithSyncKv>> {
        if !self.docs.contains_key(doc_id) {
            tracing::info!(doc_id=%doc_id, "Loading doc");
            self.load_doc(doc_id).await?;
            self.metrics.documents_loaded.add(1, &[]);
        }

        Ok(self
            .docs
            .get(doc_id)
            .ok_or_else(|| anyhow!("Failed to get-or-create doc"))?
            .map(|d| d))
    }

    pub fn check_auth(
        &self,
        auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    ) -> Result<(), AppError> {
        if let Some(auth) = &self.authenticator {
            if let Some(TypedHeader(headers::Authorization(bearer))) = auth_header {
                if let Ok(()) =
                    auth.verify_server_token(bearer.token(), current_time_epoch_millis())
                {
                    return Ok(());
                }
            }
            Err((StatusCode::UNAUTHORIZED, anyhow!("Unauthorized.")))?
        } else {
            Ok(())
        }
    }

    pub async fn redact_error_middleware(req: Request, next: Next) -> impl IntoResponse {
        let resp = next.run(req).await;
        if resp.status().is_server_error() || resp.status().is_client_error() {
            // If we should redact errors, copy over only the status code and
            // not the response body.
            return resp.status().into_response();
        }
        resp
    }

    pub fn routes(self: &Arc<Self>) -> Router {
        let protected = Router::new()
            .route("/doc/ws/:doc_id", get(handle_socket_upgrade_deprecated))
            .route("/doc/:doc_id/as-update", get(get_doc_as_update_deprecated))
            .route("/doc/:doc_id/update", post(update_doc_deprecated))
            .route("/d/:doc_id/as-update", get(get_doc_as_update))
            .route("/d/:doc_id/update", post(update_doc))
            .route(
                "/d/:doc_id/ws/:doc_id2",
                get(handle_socket_upgrade_full_path),
            )
            .route_layer(middleware::from_fn_with_state(
                self.clone(),
                validate_user_middleware,
            ));

        Router::new()
            .route("/ready", get(ready))
            .route("/check_store", post(check_store))
            .route("/check_store", get(check_store_deprecated))
            .route("/doc/new", post(new_doc))
            .route("/doc/:doc_id/auth", post(auth_doc))
            .merge(protected)
            .with_state(self.clone())
    }

    pub fn single_doc_routes(self: &Arc<Self>) -> Router {
        Router::new()
            .route("/ws/:doc_id", get(handle_socket_upgrade_single))
            .route("/as-update", get(get_doc_as_update_single))
            .route("/update", post(update_doc_single))
            .with_state(self.clone())
    }

    async fn serve_internal(
        self: Arc<Self>,
        listener: TcpListener,
        redact_errors: bool,
        routes: Router,
    ) -> Result<()> {
        let token = self.cancellation_token.clone();

        let mut app = if let Some(max_body_size) = self.max_body_size {
            routes.layer(DefaultBodyLimit::max(max_body_size))
        } else {
            routes
        };

        app = if redact_errors {
            app
        } else {
            app.layer(middleware::from_fn(Self::redact_error_middleware))
        };

        if let Some(path_prefix) = &self.path_prefix {
            app = app.clone().nest(path_prefix, app);
        }

        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async move { token.cancelled().await })
            .await?;

        self.doc_worker_tracker.close();
        self.doc_worker_tracker.wait().await;

        Ok(())
    }

    pub async fn serve(self, listener: TcpListener, redact_errors: bool) -> Result<()> {
        let s = Arc::new(self);
        let routes = s.routes();
        s.serve_internal(listener, redact_errors, routes).await
    }

    pub async fn serve_doc(self, listener: TcpListener, redact_errors: bool) -> Result<()> {
        let s = Arc::new(self);
        let routes = s.single_doc_routes();
        s.serve_internal(listener, redact_errors, routes).await
    }

    fn verify_doc_token(
        &self,
        token: Option<&str>,
        doc: &str,
    ) -> Result<(Authorization, Option<String>), AppError> {
        if let Some(authenticator) = &self.authenticator {
            if let Some(token) = token {
                let claims = authenticator
                    .verify_doc_token(token, doc, current_time_epoch_millis())
                    .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;
                Ok((claims.authorization, claims.user_id))
            } else {
                Err((StatusCode::UNAUTHORIZED, anyhow!("No token provided.")))?
            }
        } else {
            Ok((Authorization::Full, None))
        }
    }

    fn get_single_doc_id(&self) -> Result<String, AppError> {
        self.docs
            .iter()
            .next()
            .map(|entry| entry.key().clone())
            .ok_or_else(|| AppError(StatusCode::NOT_FOUND, anyhow!("No document found")))
    }
}

#[derive(Deserialize)]
struct HandlerParams {
    token: Option<String>,
}

async fn get_doc_as_update(
    State(server_state): State<Arc<Server>>,
    Path(doc_id): Path<String>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
) -> Result<Response, AppError> {
    // All authorization types allow reading the document.
    let token = get_token_from_header(auth_header);
    let _ = server_state.verify_doc_token(token.as_deref(), &doc_id)?;

    let dwskv = server_state
        .get_or_create_doc(&doc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let update = dwskv.as_update();
    tracing::debug!("update: {:?}", update);
    Ok(update.into_response())
}

async fn get_doc_as_update_deprecated(
    Path(doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
) -> Result<Response, AppError> {
    tracing::warn!("/doc/:doc_id/as-update is deprecated; call /doc/:doc_id/auth instead and then call as-update on the returned base URL.");
    get_doc_as_update(State(server_state), Path(doc_id), auth_header).await
}

async fn update_doc_deprecated(
    Path(doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    body: Bytes,
) -> Result<Response, AppError> {
    tracing::warn!("/doc/:doc_id/update is deprecated; call /doc/:doc_id/auth instead and then call update on the returned base URL.");
    update_doc(Path(doc_id), State(server_state), auth_header, body).await
}

async fn get_doc_as_update_single(
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
) -> Result<Response, AppError> {
    let doc_id = server_state.get_single_doc_id()?;
    get_doc_as_update(State(server_state), Path(doc_id), auth_header).await
}

async fn update_doc(
    Path(doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    body: Bytes,
) -> Result<Response, AppError> {
    let token = get_token_from_header(auth_header);
    let (authorization, _user_id) = server_state.verify_doc_token(token.as_deref(), &doc_id)?;
    update_doc_inner(doc_id, server_state, authorization, body).await
}

async fn update_doc_inner(
    doc_id: String,
    server_state: Arc<Server>,
    authorization: Authorization,
    body: Bytes,
) -> Result<Response, AppError> {
    if !matches!(authorization, Authorization::Full) {
        return Err(AppError(StatusCode::FORBIDDEN, anyhow!("Unauthorized.")));
    }

    let dwskv = server_state
        .get_or_create_doc(&doc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let start = std::time::Instant::now();
    if let Err(err) = dwskv.apply_update(&body) {
        tracing::error!(doc_id = %doc_id, ?err, "Failed to apply update");
        return Err(AppError(StatusCode::INTERNAL_SERVER_ERROR, err));
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    server_state.metrics.http_latency_ms.record(elapsed_ms, &[]);
    server_state.metrics.http_updates.add(1, &[]);

    Ok(StatusCode::OK.into_response())
}

async fn update_doc_single(
    State(server_state): State<Arc<Server>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    let doc_id = server_state.get_single_doc_id()?;
    // the doc server is meant to be run in Plane, so we expect verified plane
    // headers to be used for authorization.
    let authorization = get_authorization_from_plane_header(headers)?;
    update_doc_inner(doc_id, server_state, authorization, body).await
}

async fn handle_socket_upgrade(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    authorization: Authorization,
    user_id: Option<String>,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, AppError> {
    if !matches!(authorization, Authorization::Full) && !server_state.docs.contains_key(&doc_id) {
        return Err(AppError(
            StatusCode::NOT_FOUND,
            anyhow!("Doc {} not found", doc_id),
        ));
    }

    let dwskv = server_state
        .get_or_create_doc(&doc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let awareness = dwskv.awareness();
    let cancellation_token = server_state.cancellation_token.clone();
    let metrics = server_state.metrics.clone();
    let server_state_for_socket = server_state.clone();

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            awareness,
            authorization,
            user_id,
            cancellation_token,
            metrics,
            doc_id,
            server_state_for_socket,
        )
    }))
}

async fn handle_socket_upgrade_deprecated(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    Query(params): Query<HandlerParams>,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, AppError> {
    tracing::warn!(
        "/doc/ws/:doc_id is deprecated; call /doc/:doc_id/auth instead and use the returned URL."
    );
    let (authorization, user_id) = server_state.verify_doc_token(params.token.as_deref(), &doc_id)?;
    handle_socket_upgrade(ws, Path(doc_id), authorization, user_id, State(server_state)).await
}

async fn handle_socket_upgrade_full_path(
    ws: WebSocketUpgrade,
    Path((doc_id, doc_id2)): Path<(String, String)>,
    Query(params): Query<HandlerParams>,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, AppError> {
    if doc_id != doc_id2 {
        return Err(AppError(
            StatusCode::BAD_REQUEST,
            anyhow!("For Yjs compatibility, the doc_id appears twice in the URL. It must be the same in both places, but we got {} and {}.", doc_id, doc_id2),
        ));
    }
    let (authorization, user_id) = server_state.verify_doc_token(params.token.as_deref(), &doc_id)?;
    handle_socket_upgrade(ws, Path(doc_id), authorization, user_id, State(server_state)).await
}

async fn handle_socket_upgrade_single(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    headers: HeaderMap,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, AppError> {
    let single_doc_id = server_state.get_single_doc_id()?;
    if doc_id != single_doc_id {
        return Err(AppError(
            StatusCode::NOT_FOUND,
            anyhow!("Document not found"),
        ));
    }

    // the doc server is meant to be run in Plane, so we expect verified plane
    // headers to be used for authorization. Plane mode carries no userId claim,
    // so the periodic mid-session poll never runs for these connections.
    let authorization = get_authorization_from_plane_header(headers)?;
    handle_socket_upgrade(ws, Path(single_doc_id), authorization, None, State(server_state)).await
}

async fn handle_socket(
    socket: WebSocket,
    awareness: Arc<RwLock<Awareness>>,
    authorization: Authorization,
    user_id: Option<String>,
    cancellation_token: CancellationToken,
    metrics: Metrics,
    doc_id: String,
    server_state: Arc<Server>,
) {
    metrics.active_connections.add(1, &[]);
    let (mut sink, mut stream) = socket.split();
    let (send, mut recv) = channel(1024);

    let last_pong = Arc::new(RwLock::new(tokio::time::Instant::now()));
    let last_pong_clone = last_pong.clone();
    let pong_metrics = metrics.clone();
    let pong_doc_id = doc_id.clone();
    let conn_cancel = CancellationToken::new();
    let conn_cancel_ping = conn_cancel.clone();

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(PING_EVERY);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_ping_sent: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                msg = recv.recv() => {
                    let Some(msg) = msg else {
                        break;
                    };
                    let _ = sink.send(Message::Binary(msg)).await;
                }
                _ = ticker.tick() => {
                    let pong_overdue = last_pong_clone.read().expect("Failed to get read lock on last_pong").elapsed() > PONG_TIMEOUT;
                    if pong_overdue && last_ping_sent.is_some_and(|t| t.elapsed() < PONG_TIMEOUT) {
                        tracing::info!(doc_id = %pong_doc_id, "Pong timeout, closing connection");
                        pong_metrics.pong_timeouts.add(1, &[]);
                        let _ = sink.send(Message::Close(None)).await;
                        conn_cancel_ping.cancel();
                        break;
                    }
                    if sink.send(Message::Ping(vec![])).await.is_ok() {
                        last_ping_sent = Some(tokio::time::Instant::now());
                    } else {
                        conn_cancel_ping.cancel();
                        break;
                    }
                }
            }
        }
    });

    // Periodically re-checks with the Xyne backend that this connection's
    // user still has access, so a mid-session permission revocation closes
    // the connection instead of leaving it open until the client reconnects.
    // Only runs when the token carried a userId (i.e. a signing key is
    // configured) — see validate_user_middleware for why that's optional.
    if let Some(user_id) = user_id {
        let server_state_poll = server_state.clone();
        let doc_id_poll = doc_id.clone();
        let conn_cancel_poll = conn_cancel.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(server_state_poll.validate_poll_every);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // The first tick fires immediately; skip it since access was
            // already checked once by validate_user_middleware at connect time.
            ticker.tick().await;

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if !server_state_poll.validate_user_access_with_retry(&doc_id_poll, &user_id).await {
                            tracing::info!(doc_id = %doc_id_poll, user_id = %user_id, "Access revoked mid-session; closing connection");
                            conn_cancel_poll.cancel();
                            break;
                        }
                    }
                    _ = conn_cancel_poll.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    let connection = DocConnection::new(awareness, authorization, move |bytes| {
        if let Err(e) = send.try_send(bytes.to_vec()) {
            tracing::warn!(?e, "Error sending message");
        }
    });

    loop {
        tokio::select! {
            msg = stream.next() => {
                let Some(msg) = msg else {
                    break;
                };
                let msg = match msg {
                    Ok(Message::Binary(bytes)) => bytes,
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Pong(_)) => {
                        *last_pong.write().expect("Failed to get write lock on last_pong") = tokio::time::Instant::now();
                        continue;
                    }
                    Err(_e) => {
                        tracing::warn!(doc_id = %doc_id, "WebSocket stream error");
                        metrics.websocket_failures.add(1, &[]);
                        break;
                    }
                    msg => {
                        tracing::warn!(doc_id = %doc_id, ?msg, "Received non-binary message");
                        continue;
                    }
                };

                let start = std::time::Instant::now();
                if let Err(e) = connection.send(&msg).await {
                    tracing::warn!(doc_id = %doc_id, ?e, "Error handling message");
                }
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                metrics.sync_latency_ms.record(elapsed_ms, &[]);
                metrics.sync_updates.add(1, &[]);
            }
            _ = cancellation_token.cancelled() => {
                tracing::debug!(doc_id = %doc_id, "Closing doc connection due to server cancel...");
                break;
            }
            _ = conn_cancel.cancelled() => {
                tracing::debug!(doc_id = %doc_id, "Closing doc connection after pong timeout");
                break;
            }
        }
    }

    metrics.active_connections.add(-1, &[]);
}

async fn check_store(
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    State(server_state): State<Arc<Server>>,
) -> Result<Json<Value>, AppError> {
    server_state.check_auth(auth_header)?;

    if server_state.store.is_none() {
        return Ok(Json(json!({"ok": false, "error": "No store set."})));
    };

    // The check_store endpoint for the native server is kind of moot, since
    // the server will not start if store is not ok.
    Ok(Json(json!({"ok": true})))
}

async fn check_store_deprecated(
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    State(server_state): State<Arc<Server>>,
) -> Result<Json<Value>, AppError> {
    tracing::warn!(
        "GET check_store is deprecated, use POST check_store with an empty body instead."
    );
    check_store(auth_header, State(server_state)).await
}

/// Always returns a 200 OK response, as long as we are listening.
async fn ready() -> Result<Json<Value>, AppError> {
    Ok(Json(json!({"ok": true})))
}

async fn new_doc(
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    State(server_state): State<Arc<Server>>,
    Json(body): Json<DocCreationRequest>,
) -> Result<Json<NewDocResponse>, AppError> {
    server_state.check_auth(auth_header)?;

    let doc_id = if let Some(doc_id) = body.doc_id {
        if !validate_doc_name(doc_id.as_str()) {
            Err((StatusCode::BAD_REQUEST, anyhow!("Invalid document name")))?
        }

        server_state
            .get_or_create_doc(doc_id.as_str())
            .await
            .map_err(|e| {
                tracing::error!(?e, "Failed to create doc");
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            })?;

        doc_id
    } else {
        server_state.create_doc().await.map_err(|d| {
            tracing::error!(?d, "Failed to create doc");
            (StatusCode::INTERNAL_SERVER_ERROR, d)
        })?
    };

    Ok(Json(NewDocResponse { doc_id }))
}

async fn auth_doc(
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    TypedHeader(host): TypedHeader<headers::Host>,
    State(server_state): State<Arc<Server>>,
    Path(doc_id): Path<String>,
    body: Option<Json<AuthDocRequest>>,
) -> Result<Json<ClientToken>, AppError> {
    server_state.check_auth(auth_header)?;

    let Json(AuthDocRequest {
        authorization,
        user_id,
        valid_for_seconds,
    }) = body.unwrap_or_default();

    let Some(user_id) = user_id else {
        Err((StatusCode::FORBIDDEN, anyhow!("userId is required")))?
    };

    if !server_state.doc_exists(&doc_id).await {
        Err((StatusCode::NOT_FOUND, anyhow!("Doc {} not found", doc_id)))?;
    }

    let valid_for_seconds = valid_for_seconds.unwrap_or(DEFAULT_EXPIRATION_SECONDS);
    let expiration_time =
        ExpirationTimeEpochMillis(current_time_epoch_millis() + valid_for_seconds * 1000);

    let token = server_state
        .authenticator
        .as_ref()
        .map(|auth| auth.gen_doc_token(&doc_id, Some(user_id), authorization, expiration_time));

    let url = if let Some(url_prefix) = &server_state.url_prefix {
        let mut url_prefix = url_prefix.clone();
        let scheme = if url_prefix.scheme() == "https" { "wss" } else { "ws" };
        url_prefix.set_scheme(scheme).unwrap();
        let mut url_str = url_prefix.to_string();
        if !url_str.ends_with('/') {
            url_str.push('/');
        }
        format!("{url_str}d/{doc_id}/ws?docId={doc_id}")
    } else {
        format!("ws://{host}/d/{doc_id}/ws?docId={doc_id}")
    };

    let base_url = if let Some(url_prefix) = &server_state.url_prefix {
        let mut url_prefix = url_prefix.to_string();
        if !url_prefix.ends_with('/') {
            url_prefix = format!("{url_prefix}/");
        }

        format!("{url_prefix}d/{doc_id}")
    } else {
        format!("http://{host}/d/{doc_id}")
    };

    Ok(Json(ClientToken {
        url,
        base_url: Some(base_url),
        doc_id,
        token,
        authorization,
    }))
}

/// Runs in front of `as-update`, `update`, and the WebSocket-upgrade routes.
/// Resolves the request's token to a userId (via `token_identities`, set at
/// `/doc/:doc_id/auth` time) and asks the Xyne backend whether that user
/// still has access to this doc. Rejects with 403 if there's no token, no
/// associated userId, or the backend denies access.
async fn validate_user_middleware(
    State(server_state): State<Arc<Server>>,
    Path(params): Path<HashMap<String, String>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if server_state.authenticator.is_none() {
        return Ok(next.run(req).await);
    }

    let doc_id = params
        .get("doc_id")
        .cloned()
        .ok_or_else(|| AppError(StatusCode::BAD_REQUEST, anyhow!("Missing doc_id")))?;

    let token = extract_request_token(&req)
        .ok_or_else(|| AppError(StatusCode::FORBIDDEN, anyhow!("Missing token")))?;

    let (_authorization, user_id) = server_state.verify_doc_token(Some(&token), &doc_id)?;

    let Some(user_id) = user_id else {
        return Err(AppError(
            StatusCode::FORBIDDEN,
            anyhow!("Token has no associated userId"),
        ));
    };

    if !server_state.validate_user_access(&doc_id, &user_id).await {
        return Err(AppError(StatusCode::FORBIDDEN, anyhow!("Access denied")));
    }

    Ok(next.run(req).await)
}

fn extract_request_token(req: &Request) -> Option<String> {
    if let Some(query) = req.uri().query() {
        if let Some((_, v)) = url::form_urlencoded::parse(query.as_bytes()).find(|(k, _)| k == "token")
        {
            return Some(v.into_owned());
        }
    }
    req.headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn get_token_from_header(
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
) -> Option<String> {
    if let Some(TypedHeader(headers::Authorization(bearer))) = auth_header {
        Some(bearer.token().to_string())
    } else {
        None
    }
}

#[derive(Deserialize)]
struct PlaneVerifiedUserData {
    authorization: Authorization,
}

fn get_authorization_from_plane_header(headers: HeaderMap) -> Result<Authorization, AppError> {
    if let Some(token) = headers.get(HeaderName::from_static(PLANE_VERIFIED_USER_DATA_HEADER)) {
        let token_str = token.to_str().map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        let user_data: PlaneVerifiedUserData =
            serde_json::from_str(token_str).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        Ok(user_data.authorization)
    } else {
        Err((StatusCode::UNAUTHORIZED, anyhow!("No token provided.")))?
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use y_sweet_core::api_types::Authorization;

    #[tokio::test]
    async fn test_auth_doc() {
        let server_state = Server::new(
            None,
            Duration::from_secs(60),
            None,
            None,
            None,
            CancellationToken::new(),
            true,
            None,
            false,
        )
        .await
        .unwrap();

        let doc_id = server_state.create_doc().await.unwrap();

        let token = auth_doc(
            None,
            TypedHeader(headers::Host::from(http::uri::Authority::from_static(
                "localhost",
            ))),
            State(Arc::new(server_state)),
            Path(doc_id.clone()),
            Some(Json(AuthDocRequest {
                authorization: Authorization::Full,
                user_id: None,
                valid_for_seconds: None,
            })),
        )
        .await
        .unwrap();

        let expected_url = format!("ws://localhost/d/{doc_id}/ws?docId={doc_id}");
        assert_eq!(token.url, expected_url);
        assert_eq!(token.doc_id, doc_id);
        assert!(token.token.is_none());
    }

    #[tokio::test]
    async fn test_auth_doc_with_prefix() {
        let prefix: Url = "https://foo.bar".parse().unwrap();
        let server_state = Server::new(
            None,
            Duration::from_secs(60),
            None,
            Some(prefix),
            None,
            CancellationToken::new(),
            true,
            None,
            false,
        )
        .await
        .unwrap();

        let doc_id = server_state.create_doc().await.unwrap();

        let token = auth_doc(
            None,
            TypedHeader(headers::Host::from(http::uri::Authority::from_static(
                "localhost",
            ))),
            State(Arc::new(server_state)),
            Path(doc_id.clone()),
            None,
        )
        .await
        .unwrap();

        let expected_url = format!("wss://foo.bar/d/{doc_id}/ws?docId={doc_id}");
        assert_eq!(token.url, expected_url);
        assert_eq!(token.doc_id, doc_id);
        assert!(token.token.is_none());
    }

    #[tokio::test]
    async fn test_auth_doc_with_prefix_and_path() {
        let prefix: Url = "https://foo.bar/my-prefix".parse().unwrap();
        let server_state = Server::new(
            None,
            Duration::from_secs(60),
            None,
            Some(prefix),
            None,
            CancellationToken::new(),
            true,
            None,
            false,
        )
        .await
        .unwrap();

        let doc_id = server_state.create_doc().await.unwrap();

        let token = auth_doc(
            None,
            TypedHeader(headers::Host::from(http::uri::Authority::from_static(
                "localhost",
            ))),
            State(Arc::new(server_state)),
            Path(doc_id.clone()),
            None,
        )
        .await
        .unwrap();

        let expected_url = format!("wss://foo.bar/my-prefix/d/{doc_id}/ws?docId={doc_id}");
        assert_eq!(token.url, expected_url);
        let expected_base_url = format!("https://foo.bar/my-prefix/d/{doc_id}");
        assert_eq!(token.base_url, Some(expected_base_url));
        assert_eq!(token.doc_id, doc_id);
        assert!(token.token.is_none());
    }

    #[test]
    fn test_normalize_path_prefix() {
        assert_eq!(normalize_path_prefix("/ysweet".to_string()), "/ysweet");
        assert_eq!(normalize_path_prefix("ysweet".to_string()), "/ysweet");
        assert_eq!(normalize_path_prefix("/ysweet/".to_string()), "/ysweet");
        assert_eq!(normalize_path_prefix("  /ysweet/  ".to_string()), "/ysweet");
        assert_eq!(normalize_path_prefix("/a/b/c".to_string()), "/a/b/c");
        assert_eq!(normalize_path_prefix("".to_string()), "");
        assert_eq!(normalize_path_prefix("/".to_string()), "");
    }

    #[tokio::test]
    async fn test_empty_path_prefix_is_none() {
        for p in ["", "/", "   "] {
            let server = Server::new(
                None,
                Duration::from_secs(60),
                None,
                None,
                Some(p.to_string()),
                CancellationToken::new(),
                true,
                None,
                false,
            )
            .await
            .unwrap();
            assert_eq!(server.path_prefix, None, "prefix {p:?} should be None");
        }
    }

    #[tokio::test]
    async fn test_path_prefix_normalized_on_server() {
        let server = Server::new(
            None,
            Duration::from_secs(60),
            None,
            None,
            Some("ysweet/".to_string()),
            CancellationToken::new(),
            true,
            None,
            false,
        )
        .await
        .unwrap();
        assert_eq!(server.path_prefix.as_deref(), Some("/ysweet"));
    }
}
