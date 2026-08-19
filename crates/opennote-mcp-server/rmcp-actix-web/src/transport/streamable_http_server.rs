//! Streamable HTTP transport implementation.
//!
//! This module owns actix-web integration only: `Scope` composition and the builder
//! API. Every wire-protocol decision — session lifecycle, protocol-version negotiation,
//! SEP-2567 sessionless dispatch, `_meta`/header consistency, DNS-rebinding defence and
//! request-body limits — is made by
//! [`rmcp::transport::streamable_http_server::StreamableHttpService::handle`].

use std::{sync::Arc, time::Duration};

use actix_web::{
    Error as ActixError, HttpRequest, HttpResponse, Result, Scope,
    error::{ErrorBadRequest, PayloadError},
    middleware, web,
};
use bytes::Bytes;
use futures::StreamExt;
use http_body::Frame;
use http_body_util::{BodyDataStream, StreamBody};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig as RmcpConfig, StreamableHttpService as RmcpService,
    session::{SessionManager, SessionStore},
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;

/// Bound on the number of body chunks buffered between actix's `!Send` payload
/// and the `Send` body handed to rmcp. Backpressure stops the forwarder when full.
const BODY_CHANNEL_CAPACITY: usize = 8;

/// The request body type handed to rmcp's transport.
///
/// Streaming rather than buffered so rmcp's own `max_request_body_bytes` limit is
/// enforced while the body arrives, as it is for its axum front end.
type BridgedRequestBody = StreamBody<ReceiverStream<Result<Frame<Bytes>, PayloadError>>>;

/// Converts an actix-web (`http` 0.2) method into an rmcp (`http` 1.x) method.
fn convert_method(method: &actix_web::http::Method) -> Result<http::Method, ActixError> {
    http::Method::from_bytes(method.as_str().as_bytes())
        .map_err(|_| ErrorBadRequest("Bad Request: unsupported HTTP method"))
}

/// Converts an actix-web (`http` 0.2) URI into an rmcp (`http` 1.x) URI.
fn convert_uri(uri: &actix_web::http::Uri) -> Result<http::Uri, ActixError> {
    // `try_from(String)` hands the existing allocation to `Bytes`, where `parse()`
    // would copy the whole URI into a second buffer.
    http::Uri::try_from(uri.to_string())
        .map_err(|_| ErrorBadRequest("Bad Request: unsupported request URI"))
}

/// Converts an actix-web (`http` 0.2) version into an rmcp (`http` 1.x) version.
fn convert_version(version: actix_web::http::Version) -> http::Version {
    match version {
        actix_web::http::Version::HTTP_09 => http::Version::HTTP_09,
        actix_web::http::Version::HTTP_10 => http::Version::HTTP_10,
        actix_web::http::Version::HTTP_2 => http::Version::HTTP_2,
        actix_web::http::Version::HTTP_3 => http::Version::HTTP_3,
        _ => http::Version::HTTP_11,
    }
}

/// Converts an actix-web (`http` 0.2) header map into an rmcp (`http` 1.x) header map.
///
/// Repeated header values are preserved in order. A name or value the target crate
/// rejects is skipped rather than failing the request; every header rmcp inspects is
/// ASCII by specification.
///
/// Unlike [`convert_response_headers`], the skip is reachable here: `http` 1.x rejects
/// a quotation mark in a header name where `http` 0.2 accepts one, so a name actix
/// admits can have no `http` 1.x equivalent.
fn convert_request_headers(headers: &actix_web::http::header::HeaderMap) -> http::HeaderMap {
    let mut converted = http::HeaderMap::with_capacity(headers.len());
    for (name, value) in headers.iter() {
        let Ok(name) = http::header::HeaderName::from_bytes(name.as_str().as_bytes()) else {
            continue;
        };
        let Ok(value) = http::header::HeaderValue::from_bytes(value.as_bytes()) else {
            continue;
        };
        converted.append(name, value);
    }
    converted
}

/// Converts an rmcp (`http` 1.x) header map into an actix-web (`http` 0.2) header map.
///
/// The caller appends the result onto a response builder one pair at a time, which is
/// the only header API `HttpResponseBuilder` offers — it accepts no prepared map.
/// Returning the map rather than writing into the builder keeps this conversion
/// symmetric with [`convert_request_headers`] and testable without one.
///
/// A name or value the target crate rejects is skipped rather than failing the whole
/// response. No such input is reachable in this direction: the two crate versions
/// validate header values identically, and for names `http` 1.x is the stricter of
/// the two, so anything already held in an `http` 1.x map converts. The conversions
/// are fallible regardless, and skipping keeps a future divergence from panicking on
/// a response path.
///
/// The reverse direction is not symmetric — see [`convert_request_headers`].
fn convert_response_headers(headers: &http::HeaderMap) -> actix_web::http::header::HeaderMap {
    let mut converted = actix_web::http::header::HeaderMap::with_capacity(headers.len());
    for (name, value) in headers.iter() {
        let Ok(name) = actix_web::http::header::HeaderName::from_bytes(name.as_str().as_bytes())
        else {
            continue;
        };
        let Ok(value) = actix_web::http::header::HeaderValue::from_bytes(value.as_bytes()) else {
            continue;
        };
        converted.append(name, value);
    }
    converted
}

/// Adapts actix-web's `!Send` payload into a `Send` streaming body.
///
/// A bounded channel carries chunks from a task spawned on actix's local runtime to
/// the body rmcp polls, which must be `Send` to satisfy `handle`'s bound.
fn bridge_payload(mut payload: web::Payload) -> BridgedRequestBody {
    let (sender, receiver) = tokio::sync::mpsc::channel(BODY_CHANNEL_CAPACITY);
    actix_web::rt::spawn(async move {
        while let Some(chunk) = payload.next().await {
            if sender.send(chunk.map(Frame::data)).await.is_err() {
                break;
            }
        }
    });
    StreamBody::new(ReceiverStream::new(receiver))
}

/// Applies this crate's `Authorization` policy to the bridged request.
///
/// MCP servers must not forward client tokens to upstream APIs, so the header is
/// removed from what handlers can observe unless `authorization-token-passthrough`
/// is enabled. When it is, the value is additionally surfaced as
/// [`AuthorizationHeader`](super::AuthorizationHeader) in the request extensions.
fn apply_authorization_policy(
    headers: &mut http::HeaderMap,
    extensions: &mut rmcp::model::Extensions,
) {
    let Some(value) = headers.remove(http::header::AUTHORIZATION) else {
        return;
    };
    #[cfg(feature = "authorization-token-passthrough")]
    {
        let Ok(text) = value.to_str() else {
            tracing::debug!("Ignoring non-UTF-8 Authorization header");
            return;
        };
        if let Some(token) = text.strip_prefix("Bearer ")
            && !token.is_empty()
        {
            tracing::debug!(
                "Forwarding Authorization header to MCP service. MCP services must not \
                 pass this token to upstream APIs per MCP spec. See SECURITY.md."
            );
            let forwarded = super::AuthorizationHeader(text.to_string());
            headers.insert(http::header::AUTHORIZATION, value);
            extensions.insert(forwarded);
        } else {
            tracing::debug!("Ignoring malformed Authorization header");
        }
    }
    #[cfg(not(feature = "authorization-token-passthrough"))]
    {
        let _ = (value, extensions);
        tracing::debug!(
            "Stripped Authorization header; enable authorization-token-passthrough to forward it"
        );
    }
}

/// Type alias for the `on_request` hook function.
///
/// The hook is called for each incoming request and may write typed values that
/// handlers later read. rmcp carries those values to handlers inside the
/// `http::request::Parts` it places in the MCP request context.
pub type OnRequestHook = dyn Fn(&HttpRequest, &mut rmcp::model::Extensions) + Send + Sync + 'static;

/// Streamable HTTP transport service for actix-web integration.
#[derive(bon::Builder)]
pub struct StreamableHttpService<
    S,
    M = rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
> {
    /// The service factory function that creates new MCP service instances.
    service_factory: Arc<dyn Fn() -> Result<S, std::io::Error> + Send + Sync>,

    /// The session manager for tracking client connections.
    session_manager: Arc<M>,

    /// Whether to keep sessions alive for peers negotiating a legacy protocol version.
    ///
    /// Leaving this unset inherits rmcp's own default.
    ///
    /// # Stateless routing
    ///
    /// rmcp routes a request statelessly when this is `false`, and also when the request
    /// itself negotiates protocol version `2026-07-28` — SEP-2567 removed sessions from
    /// that version, so such peers are served statelessly whatever this is set to.
    /// Setting this to `false` is therefore what guarantees *every* request takes the
    /// stateless path, not what makes the stateless path exist.
    ///
    /// Setting it to `false` also narrows the accepted methods: `DELETE` answers
    /// `405 Method Not Allowed`, and so does `GET` unless the session manager supplies
    /// an event store for the transport to replay from.
    /// [`LocalSessionManager`](super::LocalSessionManager) supplies none by default, so a
    /// stateless deployment using it as-is serves `POST` only;
    /// `LocalSessionManager::default().with_event_store(store)` opts back into `GET`.
    stateful_mode: Option<bool>,

    // The three states of both interval knobs are encoded in `Option<Option<Duration>>`:
    // outer `None` inherits rmcp's default, `Some(None)` turns the interval off, and
    // `Some(Some(_))` sets it. The generated setters are private because they would
    // expose that encoding; the public ones below are hand-written and carry the docs.
    #[builder(setters(vis = "", name = sse_keep_alive_value))]
    sse_keep_alive: Option<Option<Duration>>,

    #[builder(setters(vis = "", name = sse_retry_value))]
    sse_retry: Option<Option<Duration>>,

    /// Whether to prefer `application/json` over `text/event-stream` for simple
    /// request-response tools.
    ///
    /// Consulted for every statelessly routed request, as described under
    /// [`stateful_mode`](StreamableHttpServiceBuilder::stateful_mode).
    /// Peers negotiating `2026-07-28` are served statelessly even with
    /// `stateful_mode` left at its default, so this knob reaches them too. If the
    /// handler emits a notification or request before the final response, rmcp falls
    /// back to `text/event-stream` so no message is lost. Leaving this unset inherits
    /// rmcp's own default.
    json_response: Option<bool>,

    /// Maximum accepted `POST` body size, in bytes.
    ///
    /// Enforced by rmcp while the body streams in, independently of `Content-Length`,
    /// chunked transfer encoding, and HTTP version. Oversized payloads are answered
    /// `413 Payload Too Large`. Leaving this unset inherits rmcp's own default limit.
    max_request_body_bytes: Option<usize>,

    /// Whether stateless JSON-RPC request `POST`s must carry per-request protocol
    /// signals before handler dispatch.
    ///
    /// When enabled, non-initialize requests must carry `MCP-Protocol-Version`, and
    /// ordinary non-discovery requests must also carry
    /// `_meta.io.modelcontextprotocol/protocolVersion`. Leaving this unset inherits
    /// rmcp's own default.
    ///
    /// The validation runs only on statelessly routed requests, as described under
    /// [`stateful_mode`](StreamableHttpServiceBuilder::stateful_mode).
    /// Legacy clients do not attach per-request protocol metadata, so a server enabling
    /// this should normally override
    /// [`ServerHandler::supported_protocol_versions`](rmcp::ServerHandler::supported_protocol_versions)
    /// to advertise only `2026-07-28` and later.
    stateless_protocol_metadata_required: Option<bool>,

    /// Hostnames or `host:port` authorities accepted in the inbound `Host` header.
    ///
    /// Leaving this unset inherits rmcp's own
    /// [loopback-only default][rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig::allowed_hosts],
    /// which prevents DNS-rebinding attacks against locally running servers. Deployments
    /// reachable under any other hostname must set their own list, otherwise every
    /// request is rejected with `403 Forbidden`. An empty list disables the check,
    /// accepting requests carrying any `Host` header: it does not fall back to the
    /// loopback default. Building this list from configuration that may resolve to
    /// empty silently disables DNS-rebinding protection, so treat an empty result as
    /// a configuration error rather than passing it through.
    allowed_hosts: Option<Vec<String>>,

    /// Browser origins accepted in the inbound `Origin` header.
    ///
    /// Leaving this unset inherits rmcp's own default, which performs no `Origin`
    /// validation. When set to a non-empty list, entries must include a scheme;
    /// requests without an `Origin` header still pass.
    allowed_origins: Option<Vec<String>>,

    /// External session store used for cross-instance session recovery.
    ///
    /// When set, the client's `initialize` parameters are persisted after a successful
    /// handshake and deleted when the session closes, so a request arriving at an
    /// instance with no in-memory session can transparently restore it. Leaving this
    /// unset means sessions live only in the process that created them.
    session_store: Option<Arc<dyn SessionStore>>,

    /// Token that terminates all active sessions when cancelled.
    ///
    /// Set this to tie the transport's lifetime to a coordinated shutdown. Leaving it
    /// unset gives rmcp its own token, which nothing outside the transport can cancel.
    cancellation_token: Option<CancellationToken>,

    /// Optional hook called for each request to propagate extensions from the
    /// actix-web request to the MCP request context.
    on_request: Option<Arc<OnRequestHook>>,
}

impl<S, M> Clone for StreamableHttpService<S, M> {
    fn clone(&self) -> Self {
        Self {
            service_factory: self.service_factory.clone(),
            session_manager: self.session_manager.clone(),
            stateful_mode: self.stateful_mode,
            sse_keep_alive: self.sse_keep_alive,
            sse_retry: self.sse_retry,
            json_response: self.json_response,
            max_request_body_bytes: self.max_request_body_bytes,
            stateless_protocol_metadata_required: self.stateless_protocol_metadata_required,
            allowed_hosts: self.allowed_hosts.clone(),
            allowed_origins: self.allowed_origins.clone(),
            session_store: self.session_store.clone(),
            cancellation_token: self.cancellation_token.clone(),
            on_request: self.on_request.clone(),
        }
    }
}

impl<S, M, State: streamable_http_service_builder::State> StreamableHttpServiceBuilder<S, M, State>
where
    State::OnRequest: streamable_http_service_builder::IsUnset,
{
    /// Sets the `on_request` hook using a closure, wrapping it in an `Arc`.
    pub fn on_request_fn(
        self,
        hook: impl Fn(&HttpRequest, &mut rmcp::model::Extensions) + Send + Sync + 'static,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetOnRequest<State>>
    {
        self.on_request(Arc::new(hook))
    }
}

impl<S, M, State: streamable_http_service_builder::State> StreamableHttpServiceBuilder<S, M, State>
where
    State::SseKeepAlive: streamable_http_service_builder::IsUnset,
{
    /// Sets the keep-alive interval for SSE connections.
    ///
    /// Leaving this unset inherits rmcp's own default. Call
    /// [`disable_sse_keep_alive`](Self::disable_sse_keep_alive) to turn keep-alive off
    /// instead of inheriting it. Set an explicit interval when intermediaries close
    /// idle connections faster than rmcp's default.
    pub fn sse_keep_alive(
        self,
        interval: Duration,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetSseKeepAlive<State>>
    {
        self.sse_keep_alive_value(Some(interval))
    }

    /// Sets the keep-alive interval from a value that is optional at runtime.
    ///
    /// `Some(interval)` is equivalent to
    /// [`sse_keep_alive(interval)`](Self::sse_keep_alive). `None` leaves the knob unset,
    /// so rmcp's own default applies exactly as if neither setter had been called — it
    /// does *not* turn keep-alive off, which is
    /// [`disable_sse_keep_alive`](Self::disable_sse_keep_alive). Use this for an interval
    /// read from configuration, where "absent" means "whatever rmcp chose".
    pub fn maybe_sse_keep_alive(
        self,
        interval: Option<Duration>,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetSseKeepAlive<State>>
    {
        self.maybe_sse_keep_alive_value(interval.map(Some))
    }

    /// Disables SSE keep-alive entirely, rather than inheriting rmcp's default interval.
    pub fn disable_sse_keep_alive(
        self,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetSseKeepAlive<State>>
    {
        self.sse_keep_alive_value(None)
    }
}

impl<S, M, State: streamable_http_service_builder::State> StreamableHttpServiceBuilder<S, M, State>
where
    State::SseRetry: streamable_http_service_builder::IsUnset,
{
    /// Sets the retry interval advertised to clients in SSE priming events.
    ///
    /// Leaving this unset inherits rmcp's own default. Call
    /// [`disable_sse_retry`](Self::disable_sse_retry) to omit the retry hint entirely
    /// instead of inheriting it.
    pub fn sse_retry(
        self,
        interval: Duration,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetSseRetry<State>>
    {
        self.sse_retry_value(Some(interval))
    }

    /// Sets the retry interval from a value that is optional at runtime.
    ///
    /// `Some(interval)` is equivalent to [`sse_retry(interval)`](Self::sse_retry). `None`
    /// leaves the knob unset, so rmcp's own default applies exactly as if neither setter
    /// had been called — it does *not* omit the retry hint, which is
    /// [`disable_sse_retry`](Self::disable_sse_retry). Use this for an interval read from
    /// configuration, where "absent" means "whatever rmcp chose".
    pub fn maybe_sse_retry(
        self,
        interval: Option<Duration>,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetSseRetry<State>>
    {
        self.maybe_sse_retry_value(interval.map(Some))
    }

    /// Omits the SSE retry hint entirely, rather than inheriting rmcp's default interval.
    pub fn disable_sse_retry(
        self,
    ) -> StreamableHttpServiceBuilder<S, M, streamable_http_service_builder::SetSseRetry<State>>
    {
        self.sse_retry_value(None)
    }
}

/// Per-scope state: the upstream service plus this crate's hook.
struct AppData<S, M> {
    inner: RmcpService<S, M>,
    on_request: Option<Arc<OnRequestHook>>,
}

impl<S, M> StreamableHttpService<S, M>
where
    S: rmcp::ServerHandler + Send + 'static,
    M: SessionManager + 'static,
{
    /// Assembles rmcp's transport config from the fields the caller actually set.
    ///
    /// A field left unset is not written, so it keeps whatever default rmcp
    /// chose rather than one hardcoded here.
    // This function mirrors `RmcpConfig` field by field: when rmcp gains a config
    // field, this crate gains a builder field and this function gains an arm for it.
    // `RmcpConfig` is `#[non_exhaustive]`, so nothing but this comment flags the
    // omission — an rmcp upgrade still compiles with the new knob unreachable.
    fn build_rmcp_config(&self) -> RmcpConfig {
        let mut config = RmcpConfig::default();
        if let Some(sse_keep_alive) = self.sse_keep_alive {
            config = config.with_sse_keep_alive(sse_keep_alive);
        }
        if let Some(sse_retry) = self.sse_retry {
            config = config.with_sse_retry(sse_retry);
        }
        if let Some(json_response) = self.json_response {
            config = config.with_json_response(json_response);
        }
        if let Some(max_request_body_bytes) = self.max_request_body_bytes {
            config = config.with_max_request_body_bytes(max_request_body_bytes);
        }
        if let Some(stateless_protocol_metadata_required) =
            self.stateless_protocol_metadata_required
        {
            config = config
                .with_stateless_protocol_metadata_required(stateless_protocol_metadata_required);
        }
        if let Some(stateful_mode) = self.stateful_mode {
            config = config.with_legacy_session_mode(stateful_mode);
        }
        if let Some(allowed_hosts) = self.allowed_hosts.clone() {
            config = config.with_allowed_hosts(allowed_hosts);
        }
        if let Some(allowed_origins) = self.allowed_origins.clone() {
            config = config.with_allowed_origins(allowed_origins);
        }
        if let Some(cancellation_token) = self.cancellation_token.clone() {
            config = config.with_cancellation_token(cancellation_token);
        }
        // `session_store` is the one knob rmcp gives no fluent setter, so it is written
        // by field assignment, which `#[non_exhaustive]` permits on an existing value.
        if let Some(session_store) = self.session_store.clone() {
            config.session_store = Some(session_store);
        }
        config
    }

    /// Creates a scope configured with this service, mounted at the caller's path.
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

    /// Creates a scope configured with this service, mounted at `path`.
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
        let config = self.build_rmcp_config();
        let service_factory = self.service_factory;
        let inner = RmcpService::new(move || (service_factory)(), self.session_manager, config);
        let app_data = AppData {
            inner,
            on_request: self.on_request,
        };

        // Both routes are needed. Under a non-empty prefix the empty path is the one
        // that matches: `/mcp` needs no trimming, and `/mcp/` has already been
        // trimmed by `NormalizePath::trim`. Under an empty prefix — a service
        // mounted at the application root — the empty path yields a pattern no
        // request can match, since request paths always begin with a slash; there,
        // "/" is the one that matches. Neither route shadows the other, and the pair
        // keeps serving a trailing slash even without the trimming middleware.
        web::scope(path)
            .app_data(web::Data::new(app_data))
            .wrap(middleware::NormalizePath::trim())
            .route("", web::route().to(Self::handle))
            .route("/", web::route().to(Self::handle))
    }

    /// Bridges one actix-web request through rmcp's transport and back.
    async fn handle(
        request: HttpRequest,
        payload: web::Payload,
        data: web::Data<AppData<S, M>>,
    ) -> Result<HttpResponse> {
        let mut extensions = rmcp::model::Extensions::new();
        if let Some(hook) = data.on_request.as_ref() {
            hook(&request, &mut extensions);
        }
        let mut headers = convert_request_headers(request.headers());
        apply_authorization_policy(&mut headers, &mut extensions);

        let mut builder = http::Request::builder()
            .method(convert_method(request.method())?)
            .uri(convert_uri(request.uri())?)
            .version(convert_version(request.version()));
        if let Some(builder_headers) = builder.headers_mut() {
            *builder_headers = headers;
        }
        let mut bridged = builder
            .body(bridge_payload(payload))
            .map_err(|_| ErrorBadRequest("Bad Request: malformed request"))?;
        bridged.extensions_mut().insert(extensions);

        let response = data.inner.handle(bridged).await;
        let (parts, body) = response.into_parts();

        let status = actix_web::http::StatusCode::from_u16(parts.status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
        let mut response_builder = HttpResponse::build(status);
        for (name, value) in convert_response_headers(&parts.headers) {
            response_builder.append_header((name, value));
        }

        Ok(response_builder.streaming(BodyDataStream::new(body)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::{
        ServerHandler,
        model::ServerInfo,
        transport::streamable_http_server::session::{
            SessionState, SessionStoreError, local::LocalSessionManager,
        },
    };

    #[derive(Clone)]
    struct TestService;

    impl ServerHandler for TestService {
        fn get_info(&self) -> ServerInfo {
            ServerInfo::default()
        }
    }

    /// A session store that persists nothing and never recovers a session.
    struct NeverSessionStore;

    #[async_trait::async_trait]
    impl SessionStore for NeverSessionStore {
        async fn load(&self, _session_id: &str) -> Result<Option<SessionState>, SessionStoreError> {
            Ok(None)
        }

        async fn store(
            &self,
            _session_id: &str,
            _state: &SessionState,
        ) -> Result<(), SessionStoreError> {
            Ok(())
        }

        async fn delete(&self, _session_id: &str) -> Result<(), SessionStoreError> {
            Ok(())
        }
    }

    // A macro, not a function: after two setters the bon builder's State type
    // parameter is no longer its default, so a function would have to spell out
    // bon's generated state types in its return position, as the public setters
    // above do. The macro says the same thing in one line.
    macro_rules! builder {
        () => {
            StreamableHttpService::builder()
                .service_factory(Arc::new(|| Ok(TestService)))
                .session_manager(Arc::new(LocalSessionManager::default()))
        };
    }

    #[test]
    fn set_sse_keep_alive_overrides_the_rmcp_default() {
        let config = builder!()
            .sse_keep_alive(Duration::from_secs(42))
            .build()
            .build_rmcp_config();

        assert_eq!(config.sse_keep_alive, Some(Duration::from_secs(42)));
    }

    #[test]
    fn disabled_sse_keep_alive_sends_none() {
        let config = builder!()
            .disable_sse_keep_alive()
            .build()
            .build_rmcp_config();

        assert_eq!(config.sse_keep_alive, None);
    }

    #[test]
    fn maybe_sse_keep_alive_with_a_value_overrides_the_rmcp_default() {
        let config = builder!()
            .maybe_sse_keep_alive(Some(Duration::from_secs(42)))
            .build()
            .build_rmcp_config();

        assert_eq!(config.sse_keep_alive, Some(Duration::from_secs(42)));
    }

    #[test]
    fn maybe_sse_keep_alive_with_none_inherits_the_rmcp_default() {
        let config = builder!()
            .maybe_sse_keep_alive(None)
            .build()
            .build_rmcp_config();

        assert_eq!(config.sse_keep_alive, RmcpConfig::default().sse_keep_alive);
    }

    #[test]
    fn maybe_sse_retry_with_a_value_overrides_the_rmcp_default() {
        let config = builder!()
            .maybe_sse_retry(Some(Duration::from_secs(7)))
            .build()
            .build_rmcp_config();

        assert_eq!(config.sse_retry, Some(Duration::from_secs(7)));
    }

    #[test]
    fn maybe_sse_retry_with_none_inherits_the_rmcp_default() {
        let config = builder!().maybe_sse_retry(None).build().build_rmcp_config();

        assert_eq!(config.sse_retry, RmcpConfig::default().sse_retry);
    }

    #[test]
    fn set_allowed_origins_overrides_the_rmcp_default() {
        let config = builder!()
            .allowed_origins(vec!["https://example.com".to_string()])
            .build()
            .build_rmcp_config();

        assert_eq!(config.allowed_origins, vec!["https://example.com"]);
    }

    #[test]
    fn set_stateful_mode_overrides_the_rmcp_default() {
        let config = builder!().stateful_mode(false).build().build_rmcp_config();

        assert!(!config.legacy_session_mode);
    }

    /// Every knob left unset keeps rmcp's default. This is the single home for that
    /// assertion, so a knob added later has one obvious place to be covered.
    #[test]
    fn unset_knobs_inherit_the_rmcp_defaults() {
        let config = builder!().build().build_rmcp_config();
        let defaults = RmcpConfig::default();

        assert_eq!(config.sse_keep_alive, defaults.sse_keep_alive);
        assert_eq!(config.sse_retry, defaults.sse_retry);
        assert_eq!(config.json_response, defaults.json_response);
        assert_eq!(
            config.max_request_body_bytes,
            defaults.max_request_body_bytes
        );
        assert_eq!(
            config.stateless_protocol_metadata_required,
            defaults.stateless_protocol_metadata_required
        );
        assert_eq!(config.legacy_session_mode, defaults.legacy_session_mode);
        assert_eq!(config.allowed_hosts, defaults.allowed_hosts);
        assert_eq!(config.allowed_origins, defaults.allowed_origins);
        assert!(config.session_store.is_none());

        // `CancellationToken` has no `PartialEq`, so the unset case is checked by
        // behaviour: the config must hold a token of its own. Cancelling the token of a
        // second unset build, or rmcp's own default token, must leave it uncancelled,
        // which fails if the unset arm wrote a token shared beyond this build.
        let second = builder!().build().build_rmcp_config();
        second.cancellation_token.cancel();
        defaults.cancellation_token.cancel();
        assert!(!config.cancellation_token.is_cancelled());
    }

    #[test]
    fn set_knobs_override_the_rmcp_defaults() {
        let config = builder!()
            .sse_retry(Duration::from_secs(7))
            .json_response(true)
            .max_request_body_bytes(1024)
            .stateless_protocol_metadata_required(true)
            .build()
            .build_rmcp_config();

        assert_eq!(config.sse_retry, Some(Duration::from_secs(7)));
        assert!(config.json_response);
        assert_eq!(config.max_request_body_bytes, 1024);
        assert!(config.stateless_protocol_metadata_required);
    }

    #[test]
    fn disabled_sse_retry_sends_none() {
        let config = builder!().disable_sse_retry().build().build_rmcp_config();

        assert_eq!(config.sse_retry, None);
    }

    #[test]
    fn a_set_cancellation_token_reaches_rmcps_config() {
        let token = CancellationToken::new();
        let config = builder!()
            .cancellation_token(token.clone())
            .build()
            .build_rmcp_config();

        token.cancel();

        assert!(config.cancellation_token.is_cancelled());
    }

    #[test]
    fn a_set_session_store_reaches_rmcps_config() {
        let config = builder!()
            .session_store(Arc::new(NeverSessionStore))
            .build()
            .build_rmcp_config();

        assert!(config.session_store.is_some());
    }

    #[test]
    fn converts_method_across_http_versions() {
        let converted = convert_method(&actix_web::http::Method::GET).expect("valid method");
        assert_eq!(converted, http::Method::GET);

        let converted = convert_method(&actix_web::http::Method::POST).expect("valid method");
        assert_eq!(converted, http::Method::POST);

        let converted = convert_method(&actix_web::http::Method::DELETE).expect("valid method");
        assert_eq!(converted, http::Method::DELETE);
    }

    #[test]
    fn converts_uri_preserving_path_and_query() {
        let uri: actix_web::http::Uri = "/api/v1/mcp?x=1".parse().expect("valid uri");
        let converted = convert_uri(&uri).expect("valid uri");
        assert_eq!(converted.path(), "/api/v1/mcp");
        assert_eq!(converted.query(), Some("x=1"));
    }

    #[test]
    fn converts_version_variants() {
        assert_eq!(
            convert_version(actix_web::http::Version::HTTP_11),
            http::Version::HTTP_11
        );
        assert_eq!(
            convert_version(actix_web::http::Version::HTTP_2),
            http::Version::HTTP_2
        );
    }

    #[test]
    fn converts_request_headers_including_repeated_values() {
        let mut headers = actix_web::http::header::HeaderMap::new();
        headers.append(
            actix_web::http::header::ACCEPT,
            actix_web::http::header::HeaderValue::from_static("application/json"),
        );
        headers.append(
            actix_web::http::header::ACCEPT,
            actix_web::http::header::HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            actix_web::http::header::HOST,
            actix_web::http::header::HeaderValue::from_static("127.0.0.1:8080"),
        );

        let converted = convert_request_headers(&headers);

        let accepts: Vec<&str> = converted
            .get_all(http::header::ACCEPT)
            .iter()
            .map(|value| value.to_str().expect("utf-8"))
            .collect();
        assert_eq!(accepts, vec!["application/json", "text/event-stream"]);
        assert_eq!(
            converted.get(http::header::HOST).expect("host"),
            "127.0.0.1:8080"
        );
    }

    #[test]
    fn converts_response_headers_including_repeated_values() {
        let mut headers = http::HeaderMap::new();
        headers.append(
            http::header::SET_COOKIE,
            http::HeaderValue::from_static("first=1"),
        );
        headers.append(
            http::header::SET_COOKIE,
            http::HeaderValue::from_static("second=2"),
        );
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/event-stream"),
        );

        let converted = convert_response_headers(&headers);

        let cookies: Vec<&str> = converted
            .get_all(actix_web::http::header::SET_COOKIE)
            .map(|value| value.to_str().expect("utf-8"))
            .collect();
        assert_eq!(cookies, vec!["first=1", "second=2"]);
        assert_eq!(
            converted
                .get(actix_web::http::header::CONTENT_TYPE)
                .expect("content type"),
            "text/event-stream"
        );
    }

    /// A request header name actix admits but rmcp cannot represent is skipped, and
    /// the rest of the map still converts.
    ///
    /// `http` 0.2 accepts a quotation mark in a header name where `http` 1.x does not,
    /// so this is the one input on which the two crates genuinely disagree.
    #[test]
    fn skips_request_header_names_rmcp_cannot_represent() {
        let unrepresentable = actix_web::http::header::HeaderName::from_bytes(b"x\"y")
            .expect("actix-web's http 0.2 admits a quotation mark in a header name");
        let mut headers = actix_web::http::header::HeaderMap::new();
        headers.insert(
            unrepresentable,
            actix_web::http::header::HeaderValue::from_static("dropped"),
        );
        headers.insert(
            actix_web::http::header::HOST,
            actix_web::http::header::HeaderValue::from_static("127.0.0.1:8080"),
        );

        let converted = convert_request_headers(&headers);

        assert_eq!(
            converted.len(),
            1,
            "the unrepresentable name must be skipped, not carried over or renamed"
        );
        assert_eq!(
            converted.get(http::header::HOST).expect("host"),
            "127.0.0.1:8080",
            "skipping one header must not discard the rest of the map"
        );
    }
}
