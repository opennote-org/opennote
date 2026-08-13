//! Frozen hand-written Streamable HTTP transport, gated by `legacy-transport`.
//!
//! This module provides a bidirectional HTTP transport with session management,
//! supporting both request/response and streaming patterns for MCP communication.
//! It is written entirely in this crate rather than delegating to rmcp, and it
//! receives no new features: it is retained only for deployments that depend on
//! its wire behaviour or on its flat `on_request` extension shape.
//!
//! Prefer [`streamable_http_server`](crate::transport::streamable_http_server), which delegates
//! every wire-protocol decision to rmcp. This module does not support MCP `2026-07-28`:
//! its sessionless path serves every peer the legacy wire shape. It also writes the
//! `on_request` hook's values directly into the MCP request context, so handlers read
//! them from `RequestContext::extensions` without the extra hop
//! [`on_request_extensions`](crate::transport::on_request_extensions) performs for the delegating
//! transport.
//!
//! ## Architecture
//!
//! The transport uses three HTTP methods on a single endpoint:
//! - **GET**: Resume or open SSE stream to receive server-to-client messages
//! - **POST**: Send JSON-RPC requests (returns SSE stream with responses)
//! - **DELETE**: Close session and cleanup resources
//!
//! ## Features
//!
//! - Full bidirectional communication
//! - Session management with pluggable backends
//! - Support for both streaming and request/response patterns
//! - Efficient message routing
//! - Graceful connection handling
//!
//! ## Session Management
//!
//! The transport supports different session managers:
//! - `LocalSessionManager`: In-memory session storage (default)
//! - Custom implementations via the `SessionManager` trait
//!
//! ## Example
//!
//! ```rust,no_run
//! use rmcp_actix_web::transport::legacy_streamable_http_server::StreamableHttpService;
//! use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
//! use actix_web::{App, HttpServer};
//! use std::sync::Arc;
//!
//! # use rmcp::{ServerHandler, model::ServerInfo};
//! # #[derive(Clone)]
//! # struct MyService;
//! # impl ServerHandler for MyService {
//! #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
//! # }
//! # impl MyService { fn new() -> Self { Self } }
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     // Create service OUTSIDE HttpServer::new() to share across workers
//!     let service = StreamableHttpService::builder()
//!         .service_factory(Arc::new(|| Ok(MyService::new())))
//!         .session_manager(Arc::new(LocalSessionManager::default()))
//!         .stateful_mode(true)
//!         .build();
//!
//!     HttpServer::new(move || {
//!         App::new()
//!             // Clone service for each worker (shares the same LocalSessionManager)
//!             .service(service.clone().scope())
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await
//! }
//! ```

use std::{sync::Arc, time::Duration};

use actix_web::{
    HttpRequest, HttpResponse, Result, Scope,
    error::InternalError,
    http::{
        StatusCode,
        header::{self, CACHE_CONTROL},
    },
    middleware,
    web::{self, Bytes, Data},
};
use futures::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;

/// Type alias for the on_request hook function.
///
/// This hook is called for each incoming request, allowing users to propagate
/// typed extensions from the actix-web `HttpRequest` to rmcp's `RequestContext::extensions`.
pub type OnRequestHook = dyn Fn(&HttpRequest, &mut rmcp::model::Extensions) + Send + Sync + 'static;

use rmcp::{
    RoleServer,
    model::{ClientJsonRpcMessage, ClientRequest},
    serve_server,
    service::serve_directly,
    transport::{
        OneshotTransport, TransportAdapterIdentity,
        common::http_header::{HEADER_LAST_EVENT_ID, HEADER_SESSION_ID},
        streamable_http_server::session::SessionManager,
    },
};

use rmcp::model::GetExtensions;

#[cfg(feature = "authorization-token-passthrough")]
use super::AuthorizationHeader;

// Local constants
const HEADER_X_ACCEL_BUFFERING: &str = "X-Accel-Buffering";
const EVENT_STREAM_MIME_TYPE: &str = "text/event-stream";
const JSON_MIME_TYPE: &str = "application/json";
const MISSING_SESSION_ID_BODY: &str = "Bad Request: Mcp-Session-Id header is required";
const SESSION_NOT_FOUND_BODY: &str = "Session not found";

/// Configuration for the streamable HTTP server transport.
///
/// Contains settings for session management and connection behavior.
#[derive(Debug, Clone)]
pub struct StreamableHttpServerConfig {
    /// Whether to enable stateful session management
    pub stateful_mode: bool,
    /// Optional keep-alive interval for SSE connections
    pub sse_keep_alive: Option<Duration>,
}

impl Default for StreamableHttpServerConfig {
    fn default() -> Self {
        Self {
            stateful_mode: true,
            sse_keep_alive: None,
        }
    }
}

/// Streamable HTTP transport service for actix-web integration.
///
/// Provides bidirectional MCP communication over HTTP with session management.
/// This service can be integrated into existing actix-web applications.
/// Uses a builder pattern for configuration.
///
/// # Type Parameters
///
/// * `S` - The MCP service type that handles protocol messages
/// * `M` - The session manager type (defaults to `LocalSessionManager`)
///
/// # Architecture
///
/// The service manages endpoints with multiple HTTP methods:
/// - GET: For streaming event connections
/// - POST: For sending messages and creating sessions
/// - DELETE: For closing sessions
///
/// Each client is identified by a session ID that must be provided in request headers.
///
/// # Example
///
/// ```rust,no_run
/// use rmcp_actix_web::transport::legacy_streamable_http_server::StreamableHttpService;
/// use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
/// use actix_web::{App, HttpServer, web};
/// use std::{sync::Arc, time::Duration};
///
/// # use rmcp::{ServerHandler, model::ServerInfo};
/// # #[derive(Clone)]
/// # struct MyService;
/// # impl ServerHandler for MyService {
/// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
/// # }
/// # impl MyService { fn new() -> Self { Self } }
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     // Create service OUTSIDE HttpServer::new() to share across workers
///     let service = StreamableHttpService::builder()
///         .service_factory(Arc::new(|| Ok(MyService::new())))
///         .session_manager(Arc::new(LocalSessionManager::default()))
///         .stateful_mode(true)
///         .sse_keep_alive(Duration::from_secs(30))
///         .build();
///
///     HttpServer::new(move || {
///         App::new()
///             // Clone service for each worker (shares the same LocalSessionManager)
///             .service(web::scope("/mcp").service(service.clone().scope()))
///     })
///     .bind("127.0.0.1:8080")?
///     .run()
///     .await
/// }
/// ```
#[derive(bon::Builder)]
pub struct StreamableHttpService<
    S,
    M = rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
> {
    /// The service factory function that creates new MCP service instances
    service_factory: Arc<dyn Fn() -> Result<S, std::io::Error> + Send + Sync>,

    /// The session manager for tracking client connections
    session_manager: Arc<M>,

    /// Whether to enable stateful session management
    #[builder(default = true)]
    stateful_mode: bool,

    /// Optional keep-alive interval for SSE connections
    sse_keep_alive: Option<Duration>,

    /// Optional hook called for each request to propagate extensions from HttpRequest to RequestContext.
    ///
    /// This allows middleware-populated data (e.g., JWT claims) to be accessed in MCP handlers.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use actix_web::HttpMessage;
    ///
    /// StreamableHttpService::builder()
    ///     .on_request(Arc::new(|http_req, ext| {
    ///         if let Some(claims) = http_req.extensions().get::<MyClaims>() {
    ///             ext.insert(claims.clone());
    ///         }
    ///     }))
    ///     .build()
    /// ```
    on_request: Option<Arc<OnRequestHook>>,
}

impl<S, M> Clone for StreamableHttpService<S, M> {
    fn clone(&self) -> Self {
        Self {
            service_factory: self.service_factory.clone(),
            session_manager: self.session_manager.clone(),
            stateful_mode: self.stateful_mode,
            sse_keep_alive: self.sse_keep_alive,
            on_request: self.on_request.clone(),
        }
    }
}

// Convenience methods for StreamableHttpServiceBuilder
impl<S, M, State: streamable_http_service_builder::State> StreamableHttpServiceBuilder<S, M, State>
where
    State::OnRequest: streamable_http_service_builder::IsUnset,
{
    /// Sets the on_request hook using a closure.
    ///
    /// This is a convenience method that automatically wraps the closure in an `Arc`,
    /// making it easier to use without manual Arc wrapping.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use actix_web::HttpMessage;
    ///
    /// StreamableHttpService::builder()
    ///     .on_request_fn(|http_req, ext| {
    ///         if let Some(claims) = http_req.extensions().get::<MyClaims>() {
    ///             ext.insert(claims.clone());
    ///         }
    ///     })
    ///     .build()
    /// ```
    pub fn on_request_fn(
        self,
        hook: impl Fn(&HttpRequest, &mut rmcp::model::Extensions) + Send + Sync + 'static,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetOnRequest<State>>
    {
        self.on_request(Arc::new(hook))
    }
}

/// Internal data structure used by handlers to store service configuration
/// with Arc-wrapped session manager for thread safety.
#[derive(Clone)]
struct AppData<S, M> {
    /// The service factory function that creates new MCP service instances
    service_factory: Arc<dyn Fn() -> Result<S, std::io::Error> + Send + Sync>,
    /// The session manager wrapped in Arc for thread safety
    session_manager: Arc<M>,
    /// Whether the service operates in stateful mode
    stateful_mode: bool,
    /// Optional keep-alive interval for SSE connections
    sse_keep_alive: Option<Duration>,
    /// Optional hook for propagating extensions from HttpRequest to RequestContext
    on_request: Option<Arc<OnRequestHook>>,
}

impl<S, M> AppData<S, M> {
    fn get_service(&self) -> Result<S, std::io::Error> {
        (self.service_factory)()
    }
}

// SSE Stream Helper Functions
//
// These functions provide reusable SSE keep-alive functionality to avoid code duplication.

/// Serialize a `ServerSseMessage` as a single SSE event.
///
/// Priming events ([SEP-1699](https://github.com/modelcontextprotocol/modelcontextprotocol/issues/1699))
/// carry no JSON-RPC payload (`message == None`) and MUST be emitted with an empty `data` field
/// (`data:\n\n`), not the JSON literal `null`.
fn format_sse_event(
    event_id: Option<&str>,
    message: Option<&rmcp::model::ServerJsonRpcMessage>,
) -> Bytes {
    let mut output = String::new();
    if let Some(id) = event_id {
        output.push_str(&format!("id: {id}\n"));
    }
    match message {
        Some(message) => {
            let data = serde_json::to_string(message).unwrap_or_else(|_| "{}".to_string());
            output.push_str(&format!("data: {data}\n\n"));
        }
        None => output.push_str("data:\n\n"),
    }
    Bytes::from(output)
}

/// Wraps any SSE-formatted stream with keep-alive ping support.
///
/// Adds periodic `:ping\n\n` messages during silent periods to prevent connection timeouts.
/// The wrapper automatically stops when the underlying stream ends, allowing POST responses
/// to close properly per MCP spec.
///
/// # Arguments
///
/// * `stream` - A stream of SSE-formatted bytes (already formatted as `data: ...\n\n`)
/// * `keep_alive` - Optional keep-alive interval. If `Some`, sends `:ping\n\n` at this interval
///   during silent periods. If `None`, no pings are sent.
///
/// # Returns
///
/// A stream that multiplexes the input stream with keep-alive pings, ending when the input ends.
fn wrap_with_sse_keepalive<S>(
    stream: S,
    keep_alive: Option<Duration>,
) -> impl Stream<Item = Result<Bytes, actix_web::Error>>
where
    S: Stream<Item = Result<Bytes, actix_web::Error>> + Send + 'static,
{
    async_stream::stream! {
        let mut stream = Box::pin(stream);
        let mut keep_alive_timer = keep_alive.map(|duration| tokio::time::interval(duration));

        // Consume the immediate first tick if keep-alive is enabled
        if let Some(ref mut timer) = keep_alive_timer {
            timer.tick().await;
        }

        loop {
            tokio::select! {
                result = stream.next() => {
                    match result {
                        Some(msg) => yield msg,
                        None => break, // Stream ended, stop sending pings
                    }
                }
                _ = async {
                    match keep_alive_timer.as_mut() {
                        Some(timer) => {
                            timer.tick().await;
                        }
                        None => {
                            std::future::pending::<()>().await;
                        }
                    }
                } => {
                    yield Ok(Bytes::from(":ping\n\n"));
                }
            }
        }
    }
}

impl<S, M> StreamableHttpService<S, M>
where
    S: Clone + rmcp::ServerHandler + Send + 'static,
    M: SessionManager + 'static,
{
    /// Creates a new scope configured with this service for framework-level composition.
    ///
    /// This method provides framework-level composition aligned with RMCP patterns,
    /// similar to how `SseService::scope()` works. This allows mounting the
    /// streamable HTTP service at custom paths using actix-web's routing.
    ///
    /// The method consumes `self`, so you can call it directly on the service instance.
    /// If you need to use the service multiple times, wrap it in an `Arc` and clone it.
    ///
    /// This method is equivalent to `scope_with_path("")`.
    ///
    /// # Returns
    ///
    /// Returns an actix-web `Scope` configured with the streamable HTTP routes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::transport::legacy_streamable_http_server::StreamableHttpService;
    /// use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    /// use actix_web::{App, HttpServer, web};
    /// use std::sync::Arc;
    ///
    /// # use rmcp::{ServerHandler, model::ServerInfo};
    /// # #[derive(Clone)]
    /// # struct MyService;
    /// # impl ServerHandler for MyService {
    /// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
    /// # }
    /// # impl MyService { fn new() -> Self { Self } }
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     // Create service OUTSIDE HttpServer::new() to share across workers
    ///     let service = StreamableHttpService::builder()
    ///         .service_factory(Arc::new(|| Ok(MyService::new())))
    ///         .session_manager(Arc::new(LocalSessionManager::default()))
    ///         .build();
    ///
    ///     HttpServer::new(move || {
    ///         App::new()
    ///             // Clone service for each worker (shares the same LocalSessionManager)
    ///             .service(web::scope("/api/v1/mcp").service(service.clone().scope()))
    ///     })
    ///     .bind("127.0.0.1:8080")?
    ///     .run();
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn scope(
        self,
    ) -> Scope<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        self.scope_with_path("")
    }

    /// Creates a new scope configured with this service for framework-level composition.
    ///
    /// This method provides framework-level composition aligned with RMCP patterns,
    /// similar to how `SseService::scope()` works. This allows mounting the
    /// streamable HTTP service at custom paths using actix-web's routing.
    ///
    /// The method consumes `self`, so you can call it directly on the service instance.
    /// If you need to use the service multiple times, wrap it in an `Arc` and clone it.
    ///
    /// This method is similar to `scope` except that it allows specifying a custom path.
    ///
    /// # Returns
    ///
    /// Returns an actix-web `Scope` configured with the streamable HTTP routes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rmcp_actix_web::transport::AuthorizationHeader;
    /// use rmcp_actix_web::transport::legacy_streamable_http_server::StreamableHttpService;
    /// use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    /// use actix_web::{App, HttpServer, web};
    /// use std::sync::Arc;
    ///
    /// # use rmcp::{ServerHandler, model::ServerInfo};
    /// # #[derive(Clone)]
    /// # struct MyService;
    /// # impl ServerHandler for MyService {
    /// #     fn get_info(&self) -> ServerInfo { ServerInfo::default() }
    /// # }
    /// # impl MyService { fn new() -> Self { Self } }
    /// #[actix_web::main]
    /// async fn main() -> std::io::Result<()> {
    ///     // Create service OUTSIDE HttpServer::new() to share across workers
    ///     let service = StreamableHttpService::builder()
    ///         .service_factory(Arc::new(|| Ok(MyService::new())))
    ///         .session_manager(Arc::new(LocalSessionManager::default()))
    ///         .build();
    ///
    ///     HttpServer::new(move || {
    ///         App::new()
    ///             // Clone service for each worker (shares the same LocalSessionManager)
    ///             .service(service.clone().scope_with_path("/api/v1/mcp"))
    ///     })
    ///     .bind("127.0.0.1:8080")?
    ///     .run();
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn scope_with_path(
        self,
        path: &str,
    ) -> Scope<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        let app_data = AppData {
            service_factory: self.service_factory,
            session_manager: self.session_manager,
            stateful_mode: self.stateful_mode,
            sse_keep_alive: self.sse_keep_alive,
            on_request: self.on_request,
        };

        // Each method needs both paths. Under a non-empty prefix the empty path is
        // the one that matches: `/mcp` needs no trimming, and `/mcp/` has already
        // been trimmed by `NormalizePath::trim`. Under an empty prefix — a service
        // mounted at the application root — the empty path yields a pattern no
        // request can match, since request paths always begin with a slash; there,
        // "/" is the one that matches. Neither path shadows the other.
        web::scope(path)
            .app_data(Data::new(app_data))
            .wrap(middleware::NormalizePath::trim())
            .route("", web::get().to(Self::handle_get))
            .route("", web::post().to(Self::handle_post))
            .route("", web::delete().to(Self::handle_delete))
            .route("/", web::get().to(Self::handle_get))
            .route("/", web::post().to(Self::handle_post))
            .route("/", web::delete().to(Self::handle_delete))
    }

    async fn handle_get(req: HttpRequest, service: Data<AppData<S, M>>) -> Result<HttpResponse> {
        // Check accept header
        let accept = req
            .headers()
            .get(header::ACCEPT)
            .and_then(|h| h.to_str().ok());

        if !accept.is_some_and(|header| header.contains(EVENT_STREAM_MIME_TYPE)) {
            return Ok(HttpResponse::NotAcceptable()
                .body("Not Acceptable: Client must accept text/event-stream"));
        }

        // Check session id
        let session_id = req
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned().into());

        let Some(session_id) = session_id else {
            return Ok(HttpResponse::BadRequest().body(MISSING_SESSION_ID_BODY));
        };

        tracing::debug!(%session_id, "GET request for SSE stream");

        // Check if session exists
        let has_session = service
            .session_manager
            .has_session(&session_id)
            .await
            .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        if !has_session {
            tracing::warn!(%session_id, "Session not found");
            return Ok(HttpResponse::NotFound().body(SESSION_NOT_FOUND_BODY));
        }

        // Check if last event id is provided
        let last_event_id = req
            .headers()
            .get(HEADER_LAST_EVENT_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        // Get the appropriate stream
        let sse_stream: std::pin::Pin<Box<dyn Stream<Item = _> + Send>> =
            if let Some(last_event_id) = last_event_id {
                tracing::debug!(%session_id, %last_event_id, "Resuming stream from last event");
                Box::pin(
                    service
                        .session_manager
                        .resume(&session_id, last_event_id)
                        .await
                        .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?,
                )
            } else {
                tracing::debug!(%session_id, "Creating standalone stream");
                Box::pin(
                    service
                        .session_manager
                        .create_standalone_stream(&session_id)
                        .await
                        .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?,
                )
            };

        // Convert to SSE format and add keep-alive
        let formatted_stream = sse_stream.map(|msg| {
            Ok::<_, actix_web::Error>(format_sse_event(
                msg.event_id.as_deref(),
                msg.message.as_deref(),
            ))
        });
        let sse_stream = wrap_with_sse_keepalive(formatted_stream, service.sse_keep_alive);

        Ok(HttpResponse::Ok()
            .content_type(EVENT_STREAM_MIME_TYPE)
            .append_header((CACHE_CONTROL, "no-cache"))
            .append_header((HEADER_X_ACCEL_BUFFERING, "no"))
            .streaming(sse_stream))
    }

    async fn handle_post(
        req: HttpRequest,
        body: Bytes,
        service: Data<AppData<S, M>>,
    ) -> Result<HttpResponse> {
        // Check accept header
        let accept = req
            .headers()
            .get(header::ACCEPT)
            .and_then(|h| h.to_str().ok());

        if !accept.is_some_and(|header| {
            header.contains(JSON_MIME_TYPE) && header.contains(EVENT_STREAM_MIME_TYPE)
        }) {
            return Ok(HttpResponse::NotAcceptable().body(
                "Not Acceptable: Client must accept both application/json and text/event-stream",
            ));
        }

        // Check content type
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok());

        if !content_type.is_some_and(|header| header.starts_with(JSON_MIME_TYPE)) {
            return Ok(HttpResponse::UnsupportedMediaType()
                .body("Unsupported Media Type: Content-Type must be application/json"));
        }

        // Deserialize the message
        let mut message: ClientJsonRpcMessage = serde_json::from_slice(&body)
            .map_err(|e| InternalError::new(e, StatusCode::BAD_REQUEST))?;

        tracing::debug!(?message, "POST request with message");

        if service.stateful_mode {
            // Check session id
            let session_id = req
                .headers()
                .get(HEADER_SESSION_ID)
                .and_then(|v| v.to_str().ok())
                .filter(|s| !s.is_empty());

            if let Some(session_id) = session_id {
                let session_id = session_id.to_owned().into();
                tracing::debug!(%session_id, "POST request with existing session");

                let has_session = service
                    .session_manager
                    .has_session(&session_id)
                    .await
                    .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

                if !has_session {
                    tracing::warn!(%session_id, "Session not found");
                    return Ok(HttpResponse::NotFound().body(SESSION_NOT_FOUND_BODY));
                }

                // Note: In actix-web we can't inject request parts like in tower,
                // but session_id is already available through headers

                match message {
                    #[allow(unused_mut)]
                    ClientJsonRpcMessage::Request(mut request_msg) => {
                        // Call on_request hook to propagate extensions from HttpRequest
                        if let Some(ref hook) = service.on_request {
                            hook(&req, request_msg.request.extensions_mut());
                        }

                        // Extract and inject Authorization header for existing sessions.
                        //
                        // SECURITY: This transport forwards Authorization headers to MCP services.
                        //
                        // MCP-COMPLIANT USAGE: MCP services MUST validate these tokens as intended for themselves
                        // and MUST NOT forward them to upstream APIs (per MCP specification).
                        //
                        // NON-COMPLIANT USAGE: Some implementations (e.g., rmcp-openapi-server) use these tokens
                        // for upstream API authentication. This violates MCP specifications but may be necessary
                        // for certain proxy architectures. Use with caution and ensure proper token audience validation.
                        // See SECURITY.md for details.
                        //
                        // Supports OAuth 2.1 token rotation patterns by forwarding each request's
                        // Authorization independently. This enables:
                        // - Token rotation within sessions (security best practice)
                        // - Token refresh when access tokens expire
                        // - Scope changes for different operations within the same session
                        //
                        // The proxy does NOT cache or reuse tokens from session initialization.
                        // Each request must provide its own valid Authorization header.
                        #[cfg(feature = "authorization-token-passthrough")]
                        if let Some(auth_value) = req.headers().get(header::AUTHORIZATION) {
                            match auth_value.to_str() {
                                Ok(auth_str)
                                    if auth_str.starts_with("Bearer ") && auth_str.len() > 7 =>
                                {
                                    tracing::debug!(
                                        "Forwarding Authorization header to MCP service for existing session. \
                                         Note: MCP services must not pass this token to upstream APIs per MCP spec. \
                                         See SECURITY.md for details."
                                    );
                                    request_msg
                                        .request
                                        .extensions_mut()
                                        .insert(AuthorizationHeader(auth_str.to_string()));
                                }
                                Ok(auth_str) if auth_str == "Bearer" || auth_str == "Bearer " => {
                                    tracing::debug!(
                                        "Malformed Bearer token in existing session: missing token value"
                                    );
                                }
                                Ok(auth_str) if !auth_str.starts_with("Bearer ") => {
                                    let auth_type =
                                        auth_str.split_whitespace().next().unwrap_or("unknown");
                                    tracing::warn!(
                                        "Non-Bearer authorization header ignored for existing session: {}",
                                        auth_type
                                    );
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Invalid Authorization header encoding in existing session: {}",
                                        e
                                    );
                                }
                                _ => {}
                            }
                        }

                        #[cfg(not(feature = "authorization-token-passthrough"))]
                        if req.headers().get(header::AUTHORIZATION).is_some() {
                            tracing::warn!(
                                "Authorization header present but not forwarded. \
                                 Enable 'authorization-token-passthrough' feature to forward tokens to MCP services. \
                                 Note: Token passthrough violates MCP specifications. See SECURITY.md for details."
                            );
                        }

                        let stream = service
                            .session_manager
                            .create_stream(&session_id, ClientJsonRpcMessage::Request(request_msg))
                            .await
                            .map_err(|e| {
                                InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR)
                            })?;

                        // Convert to SSE format with keep-alive
                        // Keep-alive prevents timeouts during long tool execution with no progress updates
                        // Stream closes automatically after final response (keep-alive stops when stream ends)
                        let formatted_stream = stream.map(|msg| {
                            Ok::<_, actix_web::Error>(format_sse_event(
                                msg.event_id.as_deref(),
                                msg.message.as_deref(),
                            ))
                        });
                        let sse_stream =
                            wrap_with_sse_keepalive(formatted_stream, service.sse_keep_alive);

                        Ok(HttpResponse::Ok()
                            .content_type(EVENT_STREAM_MIME_TYPE)
                            .append_header((CACHE_CONTROL, "no-cache"))
                            .append_header((HEADER_X_ACCEL_BUFFERING, "no"))
                            .streaming(sse_stream))
                    }
                    ClientJsonRpcMessage::Notification(_)
                    | ClientJsonRpcMessage::Response(_)
                    | ClientJsonRpcMessage::Error(_) => {
                        // Handle notification
                        service
                            .session_manager
                            .accept_message(&session_id, message)
                            .await
                            .map_err(|e| {
                                InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR)
                            })?;

                        Ok(HttpResponse::Accepted().finish())
                    }
                }
            } else {
                // No session id in stateful mode. A non-initialize request without
                // a session id is a 400 Bad Request per MCP 2025-03-26 Streamable
                // HTTP Session Management. The check happens before create_session
                // so a rejected request never leaves a stranded session behind.
                let is_initialize_request = matches!(
                    &message,
                    ClientJsonRpcMessage::Request(request_msg)
                        if matches!(request_msg.request, ClientRequest::InitializeRequest(_))
                );

                if !is_initialize_request {
                    tracing::warn!("Mcp-Session-Id missing for non-initialize request");
                    return Ok(HttpResponse::BadRequest().body(MISSING_SESSION_ID_BODY));
                }

                tracing::debug!("POST request without session, creating new session");

                let (session_id, transport) = service
                    .session_manager
                    .create_session()
                    .await
                    .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

                tracing::info!(%session_id, "Created new session");

                if let ClientJsonRpcMessage::Request(request_msg) = &mut message {
                    // Call on_request hook to propagate extensions from HttpRequest
                    if let Some(ref hook) = service.on_request {
                        hook(&req, request_msg.request.extensions_mut());
                    }

                    // Extract and inject Authorization header if present
                    //
                    // SECURITY: This transport forwards Authorization headers to MCP services.
                    //
                    // MCP-COMPLIANT USAGE: MCP services MUST validate these tokens as intended for themselves
                    // and MUST NOT forward them to upstream APIs (per MCP specification).
                    //
                    // NON-COMPLIANT USAGE: Some implementations (e.g., rmcp-openapi-server) use these tokens
                    // for upstream API authentication. This violates MCP specifications but may be necessary
                    // for certain proxy architectures. Use with caution and ensure proper token audience validation.
                    // See SECURITY.md for details.
                    #[cfg(feature = "authorization-token-passthrough")]
                    if let Some(auth_value) = req.headers().get(header::AUTHORIZATION) {
                        match auth_value.to_str() {
                            Ok(auth_str)
                                if auth_str.starts_with("Bearer ") && auth_str.len() > 7 =>
                            {
                                tracing::debug!(
                                    "Forwarding Authorization header to MCP service for new session. \
                                     Note: MCP services must not pass this token to upstream APIs per MCP spec. \
                                     See SECURITY.md for details."
                                );
                                request_msg
                                    .request
                                    .extensions_mut()
                                    .insert(AuthorizationHeader(auth_str.to_string()));
                            }
                            Ok(auth_str) if auth_str == "Bearer" || auth_str == "Bearer " => {
                                tracing::debug!(
                                    "Malformed Bearer token in new session: missing token value"
                                );
                            }
                            Ok(auth_str) if !auth_str.starts_with("Bearer ") => {
                                let auth_type =
                                    auth_str.split_whitespace().next().unwrap_or("unknown");
                                tracing::warn!(
                                    "Non-Bearer authorization header ignored for new session: {}",
                                    auth_type
                                );
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Invalid Authorization header encoding in new session: {}",
                                    e
                                );
                            }
                            _ => {}
                        }
                    }

                    #[cfg(not(feature = "authorization-token-passthrough"))]
                    if req.headers().get(header::AUTHORIZATION).is_some() {
                        tracing::warn!(
                            "Authorization header present but not forwarded for new session. \
                             Enable 'authorization-token-passthrough' feature to forward tokens to MCP services. \
                             Note: Token passthrough violates MCP specifications. See SECURITY.md for details."
                        );
                    }
                }

                let service_instance = service
                    .get_service()
                    .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

                // Spawn a task to serve the session
                tokio::spawn({
                    let session_manager = service.session_manager.clone();
                    let session_id = session_id.clone();
                    async move {
                        let service = serve_server::<S, M::Transport, _, TransportAdapterIdentity>(
                            service_instance,
                            transport,
                        )
                        .await;
                        match service {
                            Ok(service) => {
                                let _ = service.waiting().await;
                            }
                            Err(e) => {
                                tracing::error!("Failed to create service: {e}");
                            }
                        }
                        let _ = session_manager
                            .close_session(&session_id)
                            .await
                            .inspect_err(|e| {
                                tracing::error!("Failed to close session {session_id}: {e}");
                            });
                    }
                });

                // Get initialize response
                let response = service
                    .session_manager
                    .initialize_session(&session_id, message)
                    .await
                    .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

                tracing::debug!(?response, "Initialization complete, creating SSE stream");

                // Return SSE stream with initialization response (no keep-alive)
                // Per MCP spec: "After the JSON-RPC response has been sent, the server SHOULD close the SSE stream"
                // Initialization completes with a single response, so no keep-alive needed
                let sse_stream = async_stream::stream! {
                    yield Ok::<_, actix_web::Error>(Bytes::from(format!(
                        "data: {}\n\n",
                        serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
                    )));
                };
                tracing::debug!("Created initialization response stream (closes after response)");

                tracing::info!(
                    ?session_id,
                    "Returning SSE streaming response for initialization"
                );
                Ok(HttpResponse::Ok()
                    .content_type(EVENT_STREAM_MIME_TYPE)
                    .append_header((CACHE_CONTROL, "no-cache"))
                    .append_header((HEADER_X_ACCEL_BUFFERING, "no"))
                    .append_header((HEADER_SESSION_ID, session_id.as_ref()))
                    .streaming(sse_stream))
            }
        } else {
            // Stateless mode: MCP 2025-03-26 Streamable HTTP Session Management
            // scopes its session-id rules to "servers that require a session ID",
            // which a stateless deployment does not. Any Mcp-Session-Id value is
            // accepted, logged for observability, and otherwise ignored. The
            // Python and TypeScript reference SDKs make the same interpretation.
            tracing::debug!("POST request in stateless mode");
            if req
                .headers()
                .get(HEADER_SESSION_ID)
                .and_then(|v| v.to_str().ok())
                .filter(|s| !s.is_empty())
                .is_some()
            {
                tracing::debug!("Mcp-Session-Id header ignored in stateless mode");
            }

            match message {
                #[allow(unused_mut)]
                ClientJsonRpcMessage::Request(mut request) => {
                    tracing::debug!(?request, "Processing request in stateless mode");

                    // Call on_request hook to propagate extensions from HttpRequest
                    if let Some(ref hook) = service.on_request {
                        hook(&req, request.request.extensions_mut());
                    }

                    // Extract and inject Authorization header if present
                    //
                    // SECURITY: This transport forwards Authorization headers to MCP services.
                    //
                    // MCP-COMPLIANT USAGE: MCP services MUST validate these tokens as intended for themselves
                    // and MUST NOT forward them to upstream APIs (per MCP specification).
                    //
                    // NON-COMPLIANT USAGE: Some implementations (e.g., rmcp-openapi-server) use these tokens
                    // for upstream API authentication. This violates MCP specifications but may be necessary
                    // for certain proxy architectures. Use with caution and ensure proper token audience validation.
                    // See SECURITY.md for details.
                    #[cfg(feature = "authorization-token-passthrough")]
                    if let Some(auth_value) = req.headers().get(header::AUTHORIZATION) {
                        match auth_value.to_str() {
                            Ok(auth_str)
                                if auth_str.starts_with("Bearer ") && auth_str.len() > 7 =>
                            {
                                tracing::debug!(
                                    "Forwarding Authorization header to MCP service in stateless mode. \
                                     Note: MCP services must not pass this token to upstream APIs per MCP spec. \
                                     See SECURITY.md for details."
                                );
                                request
                                    .request
                                    .extensions_mut()
                                    .insert(AuthorizationHeader(auth_str.to_string()));
                            }
                            Ok(auth_str) if auth_str == "Bearer" || auth_str == "Bearer " => {
                                tracing::debug!(
                                    "Malformed Bearer token in stateless mode: missing token value"
                                );
                            }
                            Ok(auth_str) if !auth_str.starts_with("Bearer ") => {
                                let auth_type =
                                    auth_str.split_whitespace().next().unwrap_or("unknown");
                                tracing::warn!(
                                    "Non-Bearer authorization header ignored in stateless mode: {}",
                                    auth_type
                                );
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Invalid Authorization header encoding in stateless mode: {}",
                                    e
                                );
                            }
                            _ => {}
                        }
                    }

                    #[cfg(not(feature = "authorization-token-passthrough"))]
                    if req.headers().get(header::AUTHORIZATION).is_some() {
                        tracing::warn!(
                            "Authorization header present but not forwarded in stateless mode. \
                             Enable 'authorization-token-passthrough' feature to forward tokens to MCP services. \
                             Note: Token passthrough violates MCP specifications. See SECURITY.md for details."
                        );
                    }

                    // In stateless mode, handle the request directly
                    let service_instance = service
                        .get_service()
                        .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

                    let (transport, receiver) =
                        OneshotTransport::<RoleServer>::new(ClientJsonRpcMessage::Request(request));
                    let service_handle = serve_directly(service_instance, transport, None);

                    tokio::spawn(async move {
                        // Let the service process the request
                        let _ = service_handle.waiting().await;
                    });

                    // Convert receiver stream to SSE format with keep-alive
                    // Keep-alive prevents timeouts during long tool execution with no progress updates
                    // Stream closes automatically after final response (keep-alive stops when stream ends)
                    let formatted_stream = ReceiverStream::new(receiver).map(|message| {
                        tracing::info!(?message);
                        let data =
                            serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
                        Ok::<_, actix_web::Error>(Bytes::from(format!("data: {data}\n\n")))
                    });
                    let sse_stream =
                        wrap_with_sse_keepalive(formatted_stream, service.sse_keep_alive);

                    Ok(HttpResponse::Ok()
                        .content_type(EVENT_STREAM_MIME_TYPE)
                        .append_header((CACHE_CONTROL, "no-cache"))
                        .append_header((HEADER_X_ACCEL_BUFFERING, "no"))
                        .streaming(sse_stream))
                }
                _ => Ok(HttpResponse::UnprocessableEntity().body("Unexpected message type")),
            }
        }
    }

    async fn handle_delete(req: HttpRequest, service: Data<AppData<S, M>>) -> Result<HttpResponse> {
        // Check session id
        let session_id = req
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned().into());

        let Some(session_id) = session_id else {
            return Ok(HttpResponse::BadRequest().body(MISSING_SESSION_ID_BODY));
        };

        tracing::debug!(%session_id, "DELETE request to close session");

        let has_session = service
            .session_manager
            .has_session(&session_id)
            .await
            .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        if !has_session {
            tracing::warn!(%session_id, "Session not found");
            return Ok(HttpResponse::NotFound().body(SESSION_NOT_FOUND_BODY));
        }

        // Close session
        service
            .session_manager
            .close_session(&session_id)
            .await
            .map_err(|e| InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        tracing::info!(%session_id, "Session closed");

        Ok(HttpResponse::NoContent().finish())
    }
}

#[cfg(test)]
mod tests {
    use rmcp::model::{
        EmptyResult, JsonRpcResponse, JsonRpcVersion2_0, RequestId, ServerJsonRpcMessage,
        ServerResult,
    };

    use super::format_sse_event;

    fn dummy_message() -> ServerJsonRpcMessage {
        ServerJsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: JsonRpcVersion2_0,
            id: RequestId::Number(1),
            result: ServerResult::EmptyResult(EmptyResult {}),
        })
    }

    /// Regression test for the SEP-1699 priming-event serialization bug.
    ///
    /// Prior to the fix, `serde_json::to_string(&msg.message)` was applied to
    /// `Option<Arc<ServerJsonRpcMessage>>` directly, producing the literal
    /// `null` on the wire when the message was `None`. SEP-1699 mandates an
    /// empty `data` field instead.
    #[test]
    fn priming_event_emits_empty_data_not_null() {
        let bytes = format_sse_event(Some("0/0"), None);
        let wire = std::str::from_utf8(&bytes).expect("utf-8");

        assert_eq!(wire, "id: 0/0\ndata:\n\n");
        assert!(
            !wire.contains("data: null"),
            "priming event must not serialize the message as JSON null, got: {wire:?}"
        );
    }

    #[test]
    fn message_event_serializes_payload_as_json() {
        let message = dummy_message();
        let bytes = format_sse_event(Some("1/0"), Some(&message));
        let wire = std::str::from_utf8(&bytes).expect("utf-8");

        assert_eq!(
            wire,
            "id: 1/0\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n"
        );
    }

    #[test]
    fn message_event_without_event_id_omits_id_line() {
        let message = dummy_message();
        let bytes = format_sse_event(None, Some(&message));
        let wire = std::str::from_utf8(&bytes).expect("utf-8");

        assert_eq!(
            wire,
            "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n"
        );
    }

    /// A service mounted at the application root must reach its handler.
    ///
    /// An empty scope prefix leaves the empty-path routes matching a pattern no
    /// request can produce, so without the "/" routes every request answers 404.
    #[actix_web::test]
    async fn root_mounted_scope_reaches_the_handler() {
        use actix_web::{App, test};
        use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
        use std::sync::Arc;

        let service = super::StreamableHttpService::builder()
            .service_factory(Arc::new(|| Ok(TestHandler)))
            .session_manager(Arc::new(LocalSessionManager::default()))
            .build();
        let app = test::init_service(App::new().service(service.scope())).await;

        // An `initialize` POST is the unambiguous probe: the transport answers it
        // with a session id, whereas a session-scoped request would answer 404 for a
        // missing session, which is indistinguishable from a missing route.
        let request = test::TestRequest::post()
            .uri("/")
            .insert_header(("Content-Type", "application/json"))
            .insert_header(("Accept", "application/json, text/event-stream"))
            .set_payload(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
            )
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(
            response.status(),
            actix_web::http::StatusCode::OK,
            "a root-mounted scope must route to the transport"
        );
        assert!(
            response.headers().contains_key("mcp-session-id"),
            "the transport, not the router, must have produced this response"
        );
    }

    #[derive(Clone)]
    struct TestHandler;

    impl rmcp::ServerHandler for TestHandler {}
}
