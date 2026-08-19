//! Transport implementations for the Model Context Protocol using actix-web.
//!
//! This module provides HTTP-based transport layers that enable MCP services
//! to communicate with clients over standard web protocols.
//!
//! ## Streamable HTTP
//!
//! The [`streamable_http_server`] module provides a bidirectional transport
//! with session management. This is ideal for:
//! - Full request/response communication patterns
//! - Maintaining client state across requests
//! - Complex interaction patterns
//! - Higher performance for bidirectional communication
//!
//! See [`StreamableHttpService`] for the main implementation.
//!
//! ## Framework-Level Composition
//!
//! The transport supports framework-level composition for mounting at custom paths
//! using a builder pattern:
//!
//! ```rust,no_run
//! use actix_web::{App, HttpServer, web};
//! use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};
//! use std::{sync::Arc, time::Duration};
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
//!     let http_service = StreamableHttpService::builder()
//!         .service_factory(Arc::new(|| Ok(MyService::new())))
//!         .session_manager(Arc::new(LocalSessionManager::default()))
//!         .stateful_mode(true)
//!         .sse_keep_alive(Duration::from_secs(30))
//!         .build();
//!
//!     HttpServer::new(move || {
//!         App::new()
//!             // Mount StreamableHttp service at /api/v1/mcp/ (cloned for each worker)
//!             .service(web::scope("/api/v1/mcp").service(http_service.clone().scope()))
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await
//! }
//! ```
//!
//! ## Configuring the Transport
//!
//! `StreamableHttpServiceBuilder` reaches every knob of rmcp's
//! `StreamableHttpServerConfig`, each documented on its own setter. A knob left unset is
//! not written to rmcp's config, so it keeps whatever default rmcp chose; the two
//! interval knobs additionally offer `disable_sse_keep_alive` and `disable_sse_retry`
//! for the difference between "inherit the default" and "turn it off".
//!
//! Every third-party type those setters name is re-exported here — `SessionManager`,
//! `LocalSessionManager`, `SessionStore` and `CancellationToken` — along with the
//! [`http`] crate whose `request::Parts` handlers read, so a downstream crate need not
//! declare those dependencies itself.
//!
//! ## Propagating Extensions from Middleware
//!
//! Use the `on_request` hook to propagate typed data from actix-web middleware
//! to MCP request handlers. This is useful for passing authentication claims,
//! request metadata, or other context from HTTP middleware to your MCP service.
//! The hook receives the actix-web request and an [`Extensions`] map to write into:
//!
//! ```rust,ignore
//! use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};
//! use actix_web::HttpMessage;
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct JwtClaims { user_id: String }
//!
//! let service = StreamableHttpService::builder()
//!     .service_factory(Arc::new(|| Ok(MyService::new())))
//!     .session_manager(Arc::new(LocalSessionManager::default()))
//!     .on_request_fn(|http_req, ext| {
//!         // Access data populated by actix-web middleware
//!         if let Some(claims) = http_req.extensions().get::<JwtClaims>() {
//!             ext.insert(claims.clone());
//!         }
//!     })
//!     .build();
//! ```
//!
//! rmcp's transport nests the hook's `Extensions` inside `http::request::Parts`
//! before handing them to your MCP service, so read them back through
//! [`on_request_extensions`] rather than `RequestContext::extensions` directly:
//!
//! ```rust,ignore
//! use rmcp_actix_web::transport::on_request_extensions;
//!
//! async fn handle_request(
//!     &self,
//!     request: SomeRequest,
//!     context: RequestContext<RoleServer>,
//! ) -> Result<Response, McpError> {
//!     if let Some(claims) = on_request_extensions(&context.extensions)
//!         .and_then(|extensions| extensions.get::<JwtClaims>())
//!     {
//!         // ...
//!     }
//!     // ...
//! }
//! ```
//!
//! ## Protocol Compatibility
//!
//! The transport implements the [MCP protocol specification][mcp] and is compatible
//! with all MCP clients that support HTTP transports. The wire protocol is
//! identical to the Axum-based transports in the main [RMCP crate][rmcp].
//!
//! [mcp]: https://modelcontextprotocol.io/
//! [rmcp]: https://docs.rs/rmcp/

#[cfg(feature = "transport-streamable-http")]
pub mod streamable_http_server;

/// The hand-written actix-web transport, retained behind the `legacy-transport` feature.
///
/// It does not support MCP `2026-07-28`: its sessionless path serves every peer the
/// legacy wire shape.
#[cfg(feature = "legacy-transport")]
pub mod legacy_streamable_http_server;

#[cfg(feature = "transport-streamable-http")]
pub use streamable_http_server::{
    OnRequestHook, StreamableHttpService, StreamableHttpServiceBuilder,
};

/// Re-export of rmcp's Extensions type for use with on_request hook.
///
/// Not inlined: it belongs to rmcp, and rendering it here would present another
/// crate's API as this one's.
#[doc(no_inline)]
pub use rmcp::model::Extensions;

/// Re-export of the `http` crate, so downstream crates can name
/// `http::request::Parts` when reading raw request headers out of
/// [`rmcp::service::RequestContext::extensions`] without declaring their own
/// `http` dependency and relying on Cargo to unify the versions.
pub use http;

/// Re-export of rmcp's session-store trait, named by the builder's `session_store`.
///
/// Gated with the builder that names it: rmcp puts this trait behind the same
/// transport feature. Not inlined, for the reason given on [`Extensions`].
#[cfg(feature = "transport-streamable-http")]
#[doc(no_inline)]
pub use rmcp::transport::streamable_http_server::session::SessionStore;

/// Re-export of rmcp's session-manager trait, the `M` type parameter of
/// [`StreamableHttpService`] and the bound its `session_manager` satisfies.
///
/// Gated with the builder that names it, for the reason given on [`SessionStore`].
/// Not inlined, for the reason given on [`Extensions`].
#[cfg(feature = "transport-streamable-http")]
#[doc(no_inline)]
pub use rmcp::transport::streamable_http_server::session::SessionManager;

/// Re-export of rmcp's in-memory session manager, the default `M` of
/// [`StreamableHttpService`] and the one every example passes to `session_manager`.
///
/// Gated with the builder that names it, for the reason given on [`SessionStore`].
/// Not inlined, for the reason given on [`Extensions`].
#[cfg(feature = "transport-streamable-http")]
#[doc(no_inline)]
pub use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

/// Re-export of the cancellation token named by the builder's `cancellation_token`.
///
/// Gated with the builder that names it: nothing outside the transport feature
/// mentions this type. Not inlined, for the reason given on [`Extensions`].
#[cfg(feature = "transport-streamable-http")]
#[doc(no_inline)]
pub use tokio_util::sync::CancellationToken;

/// Retrieves the extensions written by the `on_request` hook from a handler's request context.
///
/// rmcp's transport inserts the whole [`http::request::Parts`] into the MCP message
/// extensions, so values written by the hook are reached two hops in:
/// `RequestContext::extensions` → [`http::request::Parts::extensions`] → the hook's
/// [`Extensions`]. This function performs both hops.
///
/// # Example
///
/// ```rust,ignore
/// use rmcp_actix_web::transport::on_request_extensions;
///
/// async fn handle_request(
///     &self,
///     request: SomeRequest,
///     context: RequestContext<RoleServer>,
/// ) -> Result<Response, McpError> {
///     if let Some(claims) = on_request_extensions(&context.extensions)
///         .and_then(|extensions| extensions.get::<MyClaims>())
///     {
///         // ...
///     }
///     // ...
/// }
/// ```
pub fn on_request_extensions(extensions: &Extensions) -> Option<&Extensions> {
    extensions
        .get::<http::request::Parts>()
        .and_then(|parts| parts.extensions.get::<Extensions>())
}

/// Authorization header value for MCP proxy scenarios.
///
/// This type is used to pass Authorization headers from HTTP requests
/// to MCP services via RequestContext extensions. This enables MCP services
/// to act as proxies, forwarding authentication tokens to backend APIs.
///
/// It is only inserted when the `authorization-token-passthrough` feature is
/// enabled; otherwise the transport strips the header before handlers see it.
/// Reach it through [`on_request_extensions`], which performs the two hops
/// rmcp's transport nests it behind.
///
/// # Example
///
/// ```rust,ignore
/// // In an MCP service handler:
/// use rmcp_actix_web::transport::{AuthorizationHeader, on_request_extensions};
///
/// async fn handle_request(
///     &self,
///     request: SomeRequest,
///     context: RequestContext<RoleServer>,
/// ) -> Result<Response, McpError> {
///     // Extract the Authorization header if present
///     if let Some(auth) = on_request_extensions(&context.extensions)
///         .and_then(|extensions| extensions.get::<AuthorizationHeader>())
///     {
///         // Use auth.0 to access the header value (e.g., "Bearer token123")
///         let token = &auth.0;
///         // Forward to backend API...
///     }
///     // ...
/// }
/// ```
#[derive(Clone, Debug)]
pub struct AuthorizationHeader(pub String);
