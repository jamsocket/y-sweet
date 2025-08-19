use anyhow::{anyhow, Result};
use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket},
        Path, Query, Request, State, WebSocketUpgrade,
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
use cuid::cuid2;
use dashmap::{mapref::one::MappedRef, DashMap};
use futures::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{channel, Receiver},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{error, info, span, warn, Level};
use url::Url;
use y_sweet_core::{
    api_types::{
        validate_doc_name, AssetUrl, AssetsResponse, AuthDocRequest, Authorization, ClientToken,
        ContentUploadRequest, ContentUploadResponse, DocCopyRequest, DocCopyResponse,
        DocCreationRequest, NewDocResponse,
    },
    auth::{Authenticator, ExpirationTimeEpochMillis, DEFAULT_EXPIRATION_SECONDS},
    doc_connection::DocConnection,
    doc_sync::DocWithSyncKv,
    store::Store,
    sync::awareness::Awareness,
    sync_kv::SyncKv,
};

const PLANE_VERIFIED_USER_DATA_HEADER: &str = "x-verified-user-data";

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
        // Log the error with detailed message at top level
        let error_message = format!("{}", self.1);
        let error_debug = format!("{:?}", self.1);

        error!(
            message = %error_message,
            event = "app_error",
            status_code = %self.0,
            error = %self.1,
            error_debug = %error_debug,
            error_type = "application_error"
        );
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

pub struct Server {
    docs: Arc<DashMap<String, DocWithSyncKv>>,
    doc_worker_tracker: TaskTracker,
    store: Option<Arc<Box<dyn Store>>>,
    checkpoint_freq: Duration,
    authenticator: Option<Authenticator>,
    url_prefix: Option<Url>,
    cancellation_token: CancellationToken,
    /// Whether to garbage collect docs that are no longer in use.
    /// Disabled for single-doc mode, since we only have one doc.
    doc_gc: bool,
}

impl Server {
    pub async fn new(
        store: Option<Box<dyn Store>>,
        checkpoint_freq: Duration,
        authenticator: Option<Authenticator>,
        url_prefix: Option<Url>,
        cancellation_token: CancellationToken,
        doc_gc: bool,
    ) -> Result<Self> {
        Ok(Self {
            docs: Arc::new(DashMap::new()),
            doc_worker_tracker: TaskTracker::new(),
            store: store.map(Arc::new),
            checkpoint_freq,
            authenticator,
            url_prefix,
            cancellation_token,
            doc_gc,
        })
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
        info!(
            message = format!("Document creation started: {}", doc_id),
            event = "document_creation_started",
            doc_id = %doc_id
        );
        self.load_doc(&doc_id).await?;
        info!(
            message = format!("Document created: {}", doc_id),
            event = "document_created",
            doc_id = %doc_id
        );
        Ok(doc_id)
    }

    pub async fn load_doc(&self, doc_id: &str) -> Result<()> {
        let (send, recv) = channel(1024);

        let dwskv = DocWithSyncKv::new(doc_id, self.store.clone(), move || {
            send.try_send(()).unwrap();
        })
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

            // Spawn a task to save the document to the store when it changes.
            self.doc_worker_tracker.spawn(Self::doc_persistence_worker(
                recv,
                sync_kv,
                checkpoint_freq,
                doc_id.clone(),
                cancellation_token.clone(),
            ));

            if self.doc_gc {
                self.doc_worker_tracker.spawn(Self::doc_gc_worker(
                    self.docs.clone(),
                    doc_id.clone(),
                    checkpoint_freq,
                    cancellation_token,
                ));
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
                            tracing::debug!("doc has only one reference, candidate for GC. checkpoints_without_refs: {}", checkpoints_without_refs);
                        }
                    } else {
                        break;
                    }

                    if checkpoints_without_refs >= 2 {
                        tracing::debug!("GCing doc");
                        if let Some(doc) = docs.get(&doc_id) {
                            doc.sync_kv().shutdown();
                        }

                        docs.remove(&doc_id);
                        break;
                    }
                }
                _ = cancellation_token.cancelled() => {
                    break;
                }
            };
        }
        tracing::debug!("Exiting gc_loop");
    }

    async fn doc_persistence_worker(
        mut recv: Receiver<()>,
        sync_kv: Arc<SyncKv>,
        checkpoint_freq: Duration,
        doc_id: String,
        cancellation_token: CancellationToken,
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

            tracing::debug!("Received signal. done: {}", is_done);
            let now = std::time::Instant::now();
            if !is_done && now - last_save < checkpoint_freq {
                let sleep = tokio::time::sleep(checkpoint_freq - (now - last_save));
                tokio::pin!(sleep);
                tracing::debug!("Throttling.");

                loop {
                    tokio::select! {
                        _ = &mut sleep => {
                            break;
                        }
                        v = recv.recv() => {
                            tracing::debug!("Received dirty while throttling.");
                            if v.is_none() {
                                break;
                            }
                        }
                        _ = cancellation_token.cancelled() => {
                            tracing::debug!("Received cancellation while throttling.");
                            break;
                        }

                    }
                    tracing::debug!("Done throttling.");
                }
            }
            tracing::debug!("Persisting.");
            if let Err(e) = sync_kv.persist().await {
                tracing::error!(
                    message = format!("Error persisting: {}", e),
                    event = "persist_error",
                    error = ?e
                );
            } else {
                tracing::debug!(message = "Done persisting", event = "persist_completed");
            }
            last_save = std::time::Instant::now();

            if is_done {
                break;
            }
        }
        tracing::debug!(
            message = format!("Terminating loop for: {}", doc_id),
            event = "doc_loop_terminated",
            doc_id = %doc_id
        );
    }

    pub async fn get_or_create_doc(
        &self,
        doc_id: &str,
    ) -> Result<MappedRef<String, DocWithSyncKv, DocWithSyncKv>> {
        if !self.docs.contains_key(doc_id) {
            tracing::debug!(
                message = format!("Loading doc: {}", doc_id),
                event = "doc_loading_started",
                doc_id = ?doc_id
            );
            self.load_doc(doc_id).await?;
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

    /// Structured logging middleware for request/response logging
    pub async fn logging_middleware(req: Request, next: Next) -> impl IntoResponse {
        let start = Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();

        // Extract path parameters for better logging
        let path_params = if let Some(path) = uri.path().split('/').collect::<Vec<_>>().get(2..) {
            path.join("/")
        } else {
            "".to_string()
        };

        // Extract and log request body for POST/PUT requests
        let (request_body, req) = if method == "POST" || method == "PUT" {
            // Clone the request to avoid consuming it
            let (parts, body) = req.into_parts();
            let bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => Bytes::new(),
            };

            // Process body content for logging
            let request_body = if !bytes.is_empty() {
                // Try to parse as JSON for better readability
                if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    Some(json_value) // Store the parsed JSON value directly
                } else {
                    // For non-JSON content, show first 1000 characters
                    let body_str = String::from_utf8_lossy(&bytes);
                    if body_str.len() > 1000 {
                        Some(serde_json::Value::String(format!(
                            "{}... (truncated)",
                            &body_str[..1000]
                        )))
                    } else {
                        Some(serde_json::Value::String(body_str.to_string()))
                    }
                }
            } else {
                None
            };

            // Reconstruct the request for the next middleware
            let body_stream = axum::body::Body::from(bytes);
            let req = Request::from_parts(parts, body_stream);

            (request_body, req)
        } else {
            (None, req)
        };

        // Now extract headers after req has been potentially reconstructed
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown");
        let remote_addr = req
            .extensions()
            .get::<std::net::SocketAddr>()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let span = span!(
            Level::INFO,
            "http_request",
            method = %method,
            uri = %uri,
            user_agent = %user_agent,
            remote_addr = %remote_addr,
            path = %path_params
        );

        let _enter = span.enter();

        // Store variables before moving req
        let method_clone = method.clone();
        let uri_clone = uri.clone();
        let user_agent_clone = user_agent.to_string();
        let remote_addr_clone = remote_addr.clone();
        let path_params_clone = path_params.clone();
        let request_body_clone = request_body.clone();

        let response = next.run(req).await;
        let status = response.status();
        let duration = start.elapsed();

        // Single log per request with message at top level
        let message = if status.is_server_error() {
            format!(
                "Request failed with server error: {} {} - {}ms",
                method_clone,
                uri_clone,
                duration.as_millis()
            )
        } else if status.is_client_error() {
            format!(
                "Request failed with client error: {} {} - {}ms",
                method_clone,
                uri_clone,
                duration.as_millis()
            )
        } else {
            format!(
                "Request completed: {} {} - {}ms",
                method_clone,
                uri_clone,
                duration.as_millis()
            )
        };

        if status.is_server_error() {
            error!(
                message = %message,
                event = "request_failed",
                method = %method_clone,
                uri = %uri_clone,
                status = %status,
                duration_ms = %duration.as_millis(),
                error_type = "server_error",
                remote_addr = %remote_addr_clone,
                path = %path_params_clone,
                user_agent = %user_agent_clone,
                request_body = %request_body_clone.as_ref().map(|b| serde_json::to_string(b).unwrap_or_default()).unwrap_or_default(),
            );
        } else if status.is_client_error() {
            warn!(
                message = %message,
                event = "request_failed",
                method = %method_clone,
                uri = %uri_clone,
                status = %status,
                duration_ms = %duration.as_millis(),
                error_type = "client_error",
                remote_addr = %remote_addr_clone,
                path = %path_params_clone,
                user_agent = %user_agent_clone,
                request_body = %request_body_clone.as_ref().map(|b| serde_json::to_string(b).unwrap_or_default()).unwrap_or_default(),
            );
        } else {
            info!(
                message = %message,
                event = "request_completed",
                method = %method_clone,
                uri = %uri_clone,
                status = %status,
                duration_ms = %duration.as_millis(),
                remote_addr = %remote_addr_clone,
                path = %path_params_clone,
                user_agent = %user_agent_clone,
                request_body = %request_body_clone.as_ref().map(|b| serde_json::to_string(b).unwrap_or_default()).unwrap_or_default(),
            );
        }

        response
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
        Router::new()
            .route("/ready", get(ready))
            .route("/check_store", post(check_store))
            .route("/check_store", get(check_store_deprecated))
            .route("/doc/ws/:doc_id", get(handle_socket_upgrade_deprecated))
            .route("/doc/new", post(new_doc))
            .route("/doc/:doc_id/auth", post(auth_doc))
            .route("/doc/:doc_id/as-update", get(get_doc_as_update_deprecated))
            .route("/doc/:doc_id/update", post(update_doc_deprecated))
            .route("/d/:doc_id/as-update", get(get_doc_as_update))
            .route("/d/:doc_id/update", post(update_doc))
            .route("/d/:doc_id/assets", post(generate_upload_presigned_url))
            .route("/d/:doc_id/assets", get(get_doc_assets))
            .route(
                "/d/:doc_id/ws/:doc_id2",
                get(handle_socket_upgrade_full_path),
            )
            .route("/d/:doc_id/copy", post(copy_document))
            .layer(middleware::from_fn(Self::logging_middleware))
            .with_state(self.clone())
    }

    pub fn single_doc_routes(self: &Arc<Self>) -> Router {
        Router::new()
            .route("/ws/:doc_id", get(handle_socket_upgrade_single))
            .route("/as-update", get(get_doc_as_update_single))
            .route("/update", post(update_doc_single))
            .route("/assets", post(generate_upload_presigned_url_single))
            .route("/assets", get(get_doc_assets_single))
            .layer(middleware::from_fn(Self::logging_middleware))
            .with_state(self.clone())
    }

    async fn serve_internal(
        self: Arc<Self>,
        listener: TcpListener,
        redact_errors: bool,
        routes: Router,
    ) -> Result<()> {
        let token = self.cancellation_token.clone();

        let app = if redact_errors {
            routes
        } else {
            routes.layer(middleware::from_fn(Self::redact_error_middleware))
        };

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

    fn verify_doc_token(&self, token: Option<&str>, doc: &str) -> Result<Authorization, AppError> {
        if let Some(authenticator) = &self.authenticator {
            if let Some(token) = token {
                let authorization = authenticator
                    .verify_doc_token(token, doc, current_time_epoch_millis())
                    .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;
                Ok(authorization)
            } else {
                Err((StatusCode::UNAUTHORIZED, anyhow!("No token provided.")))?
            }
        } else {
            Ok(Authorization::Full)
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
    tracing::debug!(
        message = format!("update: {:?}", update),
        event = "update_debug",
        update = ?update
    );
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
    let authorization = server_state.verify_doc_token(token.as_deref(), &doc_id)?;
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

    if let Err(err) = dwskv.apply_update(&body) {
        tracing::error!(?err, "Failed to apply update");
        return Err(AppError(StatusCode::INTERNAL_SERVER_ERROR, err));
    }

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

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(socket, awareness, authorization, cancellation_token)
    }))
}

async fn handle_socket_upgrade_deprecated(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    Query(params): Query<HandlerParams>,
    State(server_state): State<Arc<Server>>,
) -> Result<Response, AppError> {
    warn!(
        event = "deprecated_endpoint_used",
        endpoint = "/doc/ws/:doc_id",
        suggestion = "call /doc/:doc_id/auth instead and use the returned URL"
    );
    let authorization = server_state.verify_doc_token(params.token.as_deref(), &doc_id)?;
    handle_socket_upgrade(ws, Path(doc_id), authorization, State(server_state)).await
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
    let authorization = server_state.verify_doc_token(params.token.as_deref(), &doc_id)?;
    handle_socket_upgrade(ws, Path(doc_id), authorization, State(server_state)).await
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
    // headers to be used for authorization.
    let authorization = get_authorization_from_plane_header(headers)?;
    handle_socket_upgrade(ws, Path(single_doc_id), authorization, State(server_state)).await
}

async fn handle_socket(
    socket: WebSocket,
    awareness: Arc<RwLock<Awareness>>,
    authorization: Authorization,
    cancellation_token: CancellationToken,
) {
    let (mut sink, mut stream) = socket.split();
    let (send, mut recv) = channel(1024);

    info!(
        message = "WebSocket connected",
        event = "websocket_connected",
        authorization_type = %match authorization {
            Authorization::Full => "Full",
            Authorization::ReadOnly => "ReadOnly",
        }
    );

    tokio::spawn(async move {
        while let Some(msg) = recv.recv().await {
            if let Err(e) = sink.send(Message::Binary(msg)).await {
                let error_message = format!("WebSocket send error: {}", e);
                error!(
                    message = %error_message,
                    event = "websocket_send_error",
                    error = %e
                );
                break;
            }
        }
    });

    let connection = DocConnection::new(awareness, authorization, move |bytes| {
        if let Err(e) = send.try_send(bytes.to_vec()) {
            let error_message = format!("WebSocket message error: {}", e);
            warn!(
                message = %error_message,
                event = "websocket_message_error",
                error = %e
            );
        }
    });

    let mut message_count = 0u64;
    loop {
        tokio::select! {
            Some(msg) = stream.next() => {
                let msg = match msg {
                    Ok(Message::Binary(bytes)) => {
                        message_count += 1;
                        bytes
                    }
                    Ok(Message::Close(_)) => {
                        info!(
                            message = "WebSocket closed by client",
                            event = "websocket_closed",
                            total_messages = %message_count,
                            reason = "client_close"
                        );
                        break;
                    }
                    Err(e) => {
                        // The stream will complain about things like
                        // connections being lost without handshake.
                        let error_message = format!("WebSocket stream error: {}", e);
                        warn!(
                            message = %error_message,
                            event = "websocket_stream_error",
                            error = %e
                        );
                        continue;
                    }
                    msg => {
                        let error_message = format!("WebSocket invalid message: {:?}", msg);
                        warn!(
                            message = %error_message,
                            event = "websocket_invalid_message",
                            message = ?msg
                        );
                        continue;
                    }
                };

                if let Err(e) = connection.send(&msg).await {
                    let error_message = format!("WebSocket message handling error: {}", e);
                    error!(
                        message = %error_message,
                        event = "websocket_message_handling_error",
                        error = %e,
                        message_count = %message_count
                    );
                }
            }
            _ = cancellation_token.cancelled() => {
                info!(
                    message = "WebSocket closed due to server shutdown",
                    event = "websocket_closed",
                    total_messages = %message_count,
                    reason = "server_shutdown"
                );
                break;
            }
        }
    }
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
    warn!(
        message = "Deprecated endpoint used",
        event = "deprecated_endpoint_used",
        endpoint = "GET /check_store",
        suggestion = "use POST /check_store with an empty body instead"
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
                let error_message = format!("Failed to create doc: {}", e);
                tracing::error!(
                    message = %error_message,
                    event = "doc_creation_failed",
                    error = %e,
                    error_debug = ?e,
                    doc_id = %doc_id
                );
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            })?;

        doc_id
    } else {
        server_state.create_doc().await.map_err(|d| {
            let error_message = format!("Failed to create doc: {}", d);
            tracing::error!(
                message = %error_message,
                event = "doc_creation_failed",
                error = %d,
                error_debug = ?d
            );
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
        valid_for_seconds,
        ..
    }) = body.unwrap_or_default();

    if !server_state.doc_exists(&doc_id).await {
        Err((StatusCode::NOT_FOUND, anyhow!("Doc {} not found", doc_id)))?;
    }

    let valid_for_seconds = valid_for_seconds.unwrap_or(DEFAULT_EXPIRATION_SECONDS);
    let expiration_time =
        ExpirationTimeEpochMillis(current_time_epoch_millis() + valid_for_seconds * 1000);

    let token = if let Some(auth) = &server_state.authenticator {
        let token = auth.gen_doc_token(&doc_id, authorization, expiration_time);
        Some(token)
    } else {
        None
    };

    let url = if let Some(url_prefix) = &server_state.url_prefix {
        let mut url = url_prefix.clone();
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme).unwrap();
        url = url.join(&format!("/d/{doc_id}/ws")).unwrap();
        url.to_string()
    } else {
        format!("ws://{host}/d/{doc_id}/ws")
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

fn get_extension_from_content_type(content_type: &str) -> String {
    let mime = content_type
        .parse::<mime::Mime>()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let extension = mime_guess::get_mime_extensions(&mime)
        .and_then(|exts| exts.first())
        .unwrap_or(&"bin");
    format!(".{}", extension)
}

fn extract_asset_id_from_filename(filename: &str) -> Option<String> {
    // Find the last dot to separate asset_id and extension
    if let Some(last_dot_pos) = filename.rfind('.') {
        if last_dot_pos > 0 {
            return Some(filename[..last_dot_pos].to_string());
        }
    }
    // If no extension found, return the filename as is
    Some(filename.to_string())
}

async fn generate_upload_presigned_url(
    Path(doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    Json(body): Json<ContentUploadRequest>,
) -> Result<Json<ContentUploadResponse>, AppError> {
    let token = get_token_from_header(auth_header);
    let _ = server_state.verify_doc_token(token.as_deref(), &doc_id)?;

    // Check if document exists
    if !server_state.doc_exists(&doc_id).await {
        Err((StatusCode::NOT_FOUND, anyhow!("Doc {} not found", doc_id)))?;
    }

    // Generate asset ID with cuid and extension
    let asset_id = cuid2();
    let extension = get_extension_from_content_type(&body.content_type);
    let asset_name = format!("{}{}", asset_id, extension);

    // Create the key path: {doc_id}/assets/{asset_name}
    let key = format!("{}/assets/{}", doc_id, asset_name);

    let upload_url = if let Some(store) = &server_state.store {
        store
            .generate_upload_presigned_url(&key)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!("Failed to generate upload URL: {:?}", e),
                )
            })?
    } else {
        // For local development without store, return a dummy URL
        format!("file://localhost/{}", key)
    };

    Ok(Json(ContentUploadResponse {
        upload_url,
        asset_id: asset_name,
    }))
}

async fn generate_upload_presigned_url_single(
    State(server_state): State<Arc<Server>>,
    headers: HeaderMap,
    Json(body): Json<ContentUploadRequest>,
) -> Result<Json<ContentUploadResponse>, AppError> {
    let doc_id = server_state.get_single_doc_id()?;

    // the doc server is meant to be run in Plane, so we expect verified plane
    // headers to be used for authorization.
    let _ = get_authorization_from_plane_header(headers)?;

    // Generate asset ID with cuid and extension
    let asset_id = cuid2();
    let extension = get_extension_from_content_type(&body.content_type);
    let asset_name = format!("{}{}", asset_id, extension);

    // Create the key path: {doc_id}/assets/{asset_name}
    let key = format!("{}/assets/{}", doc_id, asset_name);

    let upload_url = if let Some(store) = &server_state.store {
        store
            .generate_upload_presigned_url(&key)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!("Failed to generate upload URL: {:?}", e),
                )
            })?
    } else {
        // For local development without store, return a dummy URL
        format!("file://localhost/{}", key)
    };

    Ok(Json(ContentUploadResponse {
        upload_url,
        asset_id: asset_name,
    }))
}

async fn get_doc_assets(
    Path(doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
) -> Result<Json<AssetsResponse>, AppError> {
    let token = get_token_from_header(auth_header);
    let _ = server_state.verify_doc_token(token.as_deref(), &doc_id)?;

    // Check if document exists
    if !server_state.doc_exists(&doc_id).await {
        Err((StatusCode::NOT_FOUND, anyhow!("Doc {} not found", doc_id)))?;
    }

    let assets = if let Some(store) = &server_state.store {
        // List assets in the assets directory
        let assets_prefix = format!("{}/assets/", doc_id);
        let asset_names = store.list_objects(&assets_prefix).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow!("Failed to list assets: {:?}", e),
            )
        })?;

        // Generate signed URLs for each asset
        let mut asset_urls = Vec::new();
        for filename in asset_names {
            // Extract asset_id from filename (remove extension)
            if let Some(asset_id) = extract_asset_id_from_filename(&filename) {
                let key = format!("{}/assets/{}", doc_id, filename);
                let download_url =
                    store
                        .generate_download_presigned_url(&key)
                        .await
                        .map_err(|e| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                anyhow!(
                                    "Failed to generate download URL for {}: {:?}",
                                    filename,
                                    e
                                ),
                            )
                        })?;

                asset_urls.push(AssetUrl {
                    asset_id,
                    download_url,
                });
            }
        }

        asset_urls
    } else {
        // For local development without store, return empty list
        Vec::new()
    };

    Ok(Json(AssetsResponse { assets }))
}

async fn get_doc_assets_single(
    State(server_state): State<Arc<Server>>,
    headers: HeaderMap,
) -> Result<Json<AssetsResponse>, AppError> {
    let doc_id = server_state.get_single_doc_id()?;
    let _authorization = get_authorization_from_plane_header(headers)?;

    if let Some(store) = &server_state.store {
        let mut assets = Vec::new();

        // List all objects in the document's assets directory
        let assets_prefix = format!("{}/assets/", doc_id);
        let objects = store.list_objects(&assets_prefix).await.map_err(|e| {
            AppError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow!("Failed to list assets: {}", e),
            )
        })?;

        for object_key in objects {
            // Extract asset ID from the object key
            if let Some(asset_id) = extract_asset_id_from_filename(&object_key) {
                let download_url = store
                    .generate_download_presigned_url(&object_key)
                    .await
                    .map_err(|e| {
                        AppError(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            anyhow!("Failed to generate download URL: {}", e),
                        )
                    })?;

                assets.push(AssetUrl {
                    asset_id,
                    download_url,
                });
            }
        }

        Ok(Json(AssetsResponse { assets }))
    } else {
        Err(AppError(
            StatusCode::INTERNAL_SERVER_ERROR,
            anyhow!("No store configured"),
        ))
    }
}

async fn copy_document(
    Path(source_doc_id): Path<String>,
    State(server_state): State<Arc<Server>>,
    auth_header: Option<TypedHeader<headers::Authorization<headers::authorization::Bearer>>>,
    Json(body): Json<DocCopyRequest>,
) -> Result<Json<DocCopyResponse>, AppError> {
    // Check authentication - this is an admin-only API
    server_state.check_auth(auth_header)?;

    let destination_doc_id = body.destination_doc_id;

    // Validate document IDs
    if !validate_doc_name(&source_doc_id) {
        return Err(AppError(
            StatusCode::BAD_REQUEST,
            anyhow!("Invalid source document ID"),
        ));
    }

    if !validate_doc_name(&destination_doc_id) {
        return Err(AppError(
            StatusCode::BAD_REQUEST,
            anyhow!("Invalid destination document ID"),
        ));
    }

    // Check if source document exists
    if !server_state.doc_exists(&source_doc_id).await {
        return Err(AppError(
            StatusCode::NOT_FOUND,
            anyhow!("Source document not found"),
        ));
    }

    // Perform the copy operation (will overwrite if destination exists)
    if let Some(store) = &server_state.store {
        store
            .copy_document(&source_doc_id, &destination_doc_id)
            .await
            .map_err(|e| {
                AppError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!("Failed to copy document: {}", e),
                )
            })?;

        Ok(Json(DocCopyResponse {
            source_doc_id,
            destination_doc_id,
            success: true,
        }))
    } else {
        Err(AppError(
            StatusCode::INTERNAL_SERVER_ERROR,
            anyhow!("No store configured"),
        ))
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
            CancellationToken::new(),
            true,
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

        let expected_url = format!("ws://localhost/d/{doc_id}/ws");
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
            CancellationToken::new(),
            true,
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

        let expected_url = format!("wss://foo.bar/d/{doc_id}/ws");
        assert_eq!(token.url, expected_url);
        assert_eq!(token.doc_id, doc_id);
        assert!(token.token.is_none());
    }

    #[test]
    fn test_get_extension_from_content_type() {
        // Test with actual extensions returned by mime_guess
        let jpeg_ext = get_extension_from_content_type("image/jpeg");
        assert!(jpeg_ext == ".jfif" || jpeg_ext == ".jpeg" || jpeg_ext == ".jpg");

        assert_eq!(get_extension_from_content_type("image/png"), ".png");
        assert_eq!(get_extension_from_content_type("video/mp4"), ".mp4");
        assert_eq!(get_extension_from_content_type("application/pdf"), ".pdf");

        let text_ext = get_extension_from_content_type("text/plain");
        assert!(text_ext == ".txt" || text_ext == ".asm");

        assert_eq!(get_extension_from_content_type("invalid/type"), ".bin");
    }
}
