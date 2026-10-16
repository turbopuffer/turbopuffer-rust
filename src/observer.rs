//! Hooks for observing the requests a client makes, such as to record their timings.
//!
//! Custom code installs a [`RequestObserver`] with `ClientBuilder::request_observer`. It's called synchronously at
//! fixed points of each request, so it must be cheap. A panic in an observer is caught, so it can never break a
//! request. This is an internal API for the crate's custom code, and nothing here is public.

// Custom code decides which parts of this API it uses.
#![allow(dead_code)]

use std::any::Any;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll};
use std::time::Instant;

use bytes::Bytes;
use http_body::{Frame, SizeHint};
use hyper_util::client::legacy::connect::{Connection, HttpInfo};
use reqwest::Method;

use crate::error::Error;

/// Observes a client's requests. It's shared by concurrent requests, so per request state belongs on the
/// [`RequestScope`] it returns.
pub(crate) trait RequestObserver: Send + Sync {
    /// Called once per request, before its first attempt. Returns a scope to observe the request, or `None` to leave
    /// it unobserved.
    fn start(&self, info: &RequestInfo<'_>) -> Option<Box<dyn RequestScope>>;
}

/// Observes one request across its attempts.
pub(crate) trait RequestScope: Send {
    /// Called before each attempt is sent. May change the request's headers.
    fn attempt_begin(&mut self, _attempt: u32, _request: &mut reqwest::Request) {}

    /// Called after each attempt, success or failure, while its response headers can still be read.
    fn attempt_end(&mut self, _result: &AttemptResult<'_>) {}

    /// Called at most once, after the response is parsed or when the request fails for good, including when the
    /// caller drops the request's future.
    fn finish(&mut self, _result: &RequestResult<'_>) {}
}

/// Describes a request as it starts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RequestInfo<'a> {
    pub(crate) method: &'a Method,
    /// The path the request was made with, like `/v1/things/abc`. It keeps the spec path's own query, like
    /// `?stainless_overload=create`, but not query params added from request params.
    pub(crate) url: &'a str,
}

/// The outcome of one attempt of a request.
#[derive(Debug)]
pub(crate) struct AttemptResult<'a> {
    /// The attempt's 0-based index.
    pub(crate) attempt: u32,
    /// The request as it was sent.
    pub(crate) request: &'a reqwest::Request,
    /// The status and headers of the response, if one arrived.
    pub(crate) response: Option<&'a http::Response<()>>,
    /// The error, if the attempt failed without a response.
    pub(crate) error: Option<&'a Error>,
    pub(crate) error_kind: Option<ErrorKind>,
    /// `Some(false)` when the attempt certainly never reached the server, `Some(true)` when a response arrived, and
    /// `None` when unknown.
    pub(crate) reached_server: Option<bool>,
    pub(crate) timings: &'a Timings,
}

/// The final outcome of a request.
#[derive(Debug)]
pub(crate) struct RequestResult<'a> {
    /// The number of retries made, which is also the final attempt's index.
    pub(crate) retries: u32,
    /// The status and headers of the response the request ended with, if any.
    pub(crate) response: Option<&'a http::Response<()>>,
    /// The error the request failed with. For an error response it's the API error, which has no `error_kind`. It's
    /// `None` when the request was canceled, since a dropped future has no error.
    pub(crate) error: Option<&'a Error>,
    pub(crate) error_kind: Option<ErrorKind>,
    /// The value the response body was parsed into, if it was parsed.
    pub(crate) parsed: Option<&'a dyn Any>,
    /// The final attempt's timings.
    pub(crate) timings: &'a Timings,
}

/// Why an attempt or request failed without a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ErrorKind {
    PoolTimeout,
    ConnectTimeout,
    ConnectError,
    WriteTimeout,
    ReadTimeout,
    ReadError,
    /// Any other timeout.
    Timeout,
    /// The caller dropped the request's future.
    Canceled,
    Other,
}

impl ErrorKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PoolTimeout => "pool_timeout",
            Self::ConnectTimeout => "connect_timeout",
            Self::ConnectError => "connect_error",
            Self::WriteTimeout => "write_timeout",
            Self::ReadTimeout => "read_timeout",
            Self::ReadError => "read_error",
            Self::Timeout => "timeout",
            Self::Canceled => "canceled",
            Self::Other => "other",
        }
    }

    pub(crate) fn is_timeout(self) -> bool {
        matches!(
            self,
            Self::PoolTimeout
                | Self::ConnectTimeout
                | Self::WriteTimeout
                | Self::ReadTimeout
                | Self::Timeout
        )
    }
}

/// Monotonic marks of the progress of one attempt. A mark stays `None` when its phase didn't happen or couldn't be
/// observed. Phases may overlap.
///
/// Connections and sending are only traced on the client's own HTTP client, not one passed to
/// `ClientBuilder::http_client`. Sending is traced through the request body, so it isn't for requests without one.
#[derive(Debug, Clone, Default)]
pub(crate) struct Timings {
    /// The attempt began. The first attempt begins when its body is serialized.
    pub(crate) attempt_begin: Option<Instant>,
    /// Encoding the request body, once before the first attempt.
    pub(crate) serialize_begin: Option<Instant>,
    pub(crate) serialize_end: Option<Instant>,
    /// The request was built and handed to the HTTP client.
    pub(crate) request_ready: Option<Instant>,
    /// Opening a new connection: DNS, TCP and TLS. Unset when an existing connection was used.
    pub(crate) connect_begin: Option<Instant>,
    pub(crate) connect_end: Option<Instant>,
    /// The connection started and finished taking the request body, which is written along with the headers.
    pub(crate) send_begin: Option<Instant>,
    pub(crate) send_end: Option<Instant>,
    /// The response headers were received.
    pub(crate) response_headers: Option<Instant>,
    /// The response body was fully read.
    pub(crate) body_end: Option<Instant>,
    /// Parsing the response body, only on the final attempt.
    pub(crate) deserialize_begin: Option<Instant>,
    pub(crate) deserialize_end: Option<Instant>,
    /// Whether the attempt used an existing connection, or `None` when unknown.
    pub(crate) reused: Option<bool>,
    /// Marks added by custom code.
    pub(crate) extra: HashMap<&'static str, Instant>,
}

impl Timings {
    /// Records the current time in `extra` under `name`.
    pub(crate) fn mark(&mut self, name: &'static str) {
        self.extra.insert(name, Instant::now());
    }
}

/// Drives one request's scope through its attempts. It's inert when the request isn't observed, which costs a null
/// pointer and no work.
pub(crate) struct Observation(Option<Box<Active>>);

struct Active {
    /// Taken by `finish`, so it's called at most once.
    scope: Option<Box<dyn RequestScope>>,
    trace_connections: bool,
    attempt: u32,
    timings: Timings,
    /// The attempt in flight, from `attempt_begin` until it ends.
    current: Option<Current>,
    /// The head of the final attempt's response.
    response: Option<http::Response<()>>,
    /// Why the request failed so far, if it failed without a response.
    error_kind: Option<ErrorKind>,
}

struct Current {
    request: reqwest::Request,
    trace: Option<Arc<Trace>>,
    /// Whether the request body traces sending, so a missing mark means the request wasn't sent yet.
    traced_body: bool,
}

impl Observation {
    /// An observation that does nothing.
    pub(crate) fn inert() -> Self {
        Self(None)
    }

    /// Starts observing a request, if `observer` returns a scope for it. `serialized` bounds encoding its body.
    pub(crate) fn start(
        observer: &dyn RequestObserver,
        trace_connections: bool,
        info: &RequestInfo<'_>,
        serialized: Option<(Instant, Instant)>,
    ) -> Self {
        let Some(scope) = guarded(|| observer.start(info)).flatten() else {
            return Self(None);
        };
        let timings = Timings {
            serialize_begin: serialized.map(|(start, _)| start),
            serialize_end: serialized.map(|(_, end)| end),
            ..Timings::default()
        };
        Self(Some(Box::new(Active {
            scope: Some(scope),
            trace_connections,
            attempt: 0,
            timings,
            current: None,
            response: None,
            error_kind: None,
        })))
    }

    pub(crate) fn is_active(&self) -> bool {
        self.0.is_some()
    }

    /// Marks the start of an attempt, before its request is built.
    pub(crate) fn start_attempt(&mut self, attempt: u32) {
        let Some(active) = self.0.as_deref_mut() else {
            return;
        };
        let now = Instant::now();
        if attempt == 0 {
            active.timings.attempt_begin = Some(active.timings.serialize_begin.unwrap_or(now));
        } else {
            active.attempt = attempt;
            active.timings = Timings {
                attempt_begin: Some(now),
                ..Timings::default()
            };
        }
    }

    /// Calls the scope before a request is sent, and traces its connection and body if the client allows it.
    /// `body` is the request's body, and the returned trace must scope sending it with [`traced`].
    pub(crate) fn attempt_begin(
        &mut self,
        request: &mut reqwest::Request,
        body: Option<&Bytes>,
    ) -> Option<Arc<Trace>> {
        let active = self.0.as_deref_mut()?;
        active.timings.request_ready = Some(Instant::now());
        let attempt = active.attempt;
        if let Some(scope) = active.scope.as_mut() {
            guarded(|| scope.attempt_begin(attempt, request));
        }
        let sent = copy_request(request);
        let trace = active.trace_connections.then(Arc::<Trace>::default);
        let mut traced_body = false;
        if let (Some(trace), Some(body)) = (&trace, body.filter(|body| !body.is_empty())) {
            *request.body_mut() = Some(reqwest::Body::wrap(TracedBody {
                remaining: body.clone(),
                started: false,
                trace: trace.clone(),
            }));
            traced_body = true;
        }
        active.current = Some(Current {
            request: sent,
            trace: trace.clone(),
            traced_body,
        });
        trace
    }

    /// Ends an attempt that got a response, before its body is read.
    pub(crate) fn attempt_response(&mut self, response: &reqwest::Response) {
        let Some(active) = self.0.as_deref_mut() else {
            return;
        };
        active.timings.response_headers = Some(Instant::now());
        let current = active.current.take();
        if let Some(trace) = current.as_ref().and_then(|current| current.trace.as_ref()) {
            trace.settle(&mut active.timings, connection_addrs(response.extensions()));
        }
        let mut head = http::Response::new(());
        *head.status_mut() = response.status();
        *head.version_mut() = response.version();
        *head.headers_mut() = response.headers().clone();
        active.response = Some(head);
        active.error_kind = None;
        if let Some(current) = current {
            active.attempt_end(&current.request, None, None, Some(true));
        }
    }

    /// Ends an attempt that failed without a response, converting its error.
    pub(crate) fn attempt_failed(&mut self, err: reqwest::Error) -> Error {
        let Some(active) = self.0.as_deref_mut() else {
            return Error::from_reqwest(err);
        };
        let current = active.current.take();
        if let Some(trace) = current.as_ref().and_then(|current| current.trace.as_ref()) {
            trace.settle(&mut active.timings, None);
        }
        let traced_body = current.as_ref().is_some_and(|current| current.traced_body);
        let kind = error_kind(&err, phase(&active.timings, traced_body));
        let error = Error::from_reqwest(err);
        active.response = None;
        active.error_kind = Some(kind);
        // A request that couldn't be built never began its attempt.
        if let Some(current) = current {
            let reached_server = matches!(
                kind,
                ErrorKind::PoolTimeout | ErrorKind::ConnectTimeout | ErrorKind::ConnectError
            )
            .then_some(false);
            active.attempt_end(&current.request, Some(&error), Some(kind), reached_server);
        }
        error
    }

    /// Marks the response body as read, unless custom code that read it already did.
    pub(crate) fn body_end(&mut self) {
        if let Some(active) = self.0.as_deref_mut() {
            active.timings.body_end.get_or_insert_with(Instant::now);
        }
    }

    /// Records that reading the response body failed, converting its error.
    pub(crate) fn body_failed(&mut self, err: reqwest::Error) -> Error {
        if let Some(active) = self.0.as_deref_mut() {
            active.error_kind = Some(if err.is_timeout() {
                ErrorKind::ReadTimeout
            } else {
                ErrorKind::ReadError
            });
            // The request has no usable response.
            active.response = None;
        }
        Error::from_reqwest(err)
    }

    pub(crate) fn deserialize_begin(&mut self) {
        if let Some(active) = self.0.as_deref_mut() {
            active.timings.deserialize_begin = Some(Instant::now());
        }
    }

    /// Finishes the request with the value its response was parsed into, or the error it failed with.
    pub(crate) fn finish(&mut self, parsed: Option<&dyn Any>, error: Option<&Error>) {
        let Some(active) = self.0.as_deref_mut() else {
            return;
        };
        let error_kind = match error {
            // An error response, which the response describes.
            None | Some(Error::Api(_)) => None,
            // The body couldn't be parsed.
            Some(_) if active.timings.deserialize_begin.is_some() => Some(ErrorKind::Other),
            Some(_) => Some(active.error_kind.unwrap_or(ErrorKind::Other)),
        };
        active.finish(parsed, error, error_kind);
    }
}

impl Active {
    fn attempt_end(
        &mut self,
        request: &reqwest::Request,
        error: Option<&Error>,
        error_kind: Option<ErrorKind>,
        reached_server: Option<bool>,
    ) {
        let Some(scope) = self.scope.as_mut() else {
            return;
        };
        let result = AttemptResult {
            attempt: self.attempt,
            request,
            response: self.response.as_ref(),
            error,
            error_kind,
            reached_server,
            timings: &self.timings,
        };
        guarded(|| scope.attempt_end(&result));
    }

    fn finish(
        &mut self,
        parsed: Option<&dyn Any>,
        error: Option<&Error>,
        error_kind: Option<ErrorKind>,
    ) {
        let Some(mut scope) = self.scope.take() else {
            return;
        };
        if self.timings.deserialize_begin.is_some() {
            self.timings.deserialize_end = Some(Instant::now());
        }
        let result = RequestResult {
            retries: self.attempt,
            response: self.response.as_ref(),
            error,
            error_kind,
            parsed,
            timings: &self.timings,
        };
        guarded(|| scope.finish(&result));
    }
}

impl Drop for Observation {
    /// Finishes a request whose future was dropped before it finished, e.g. by a timeout or `select!` around it.
    fn drop(&mut self) {
        let Some(active) = self.0.as_deref_mut() else {
            return;
        };
        // A request unwinding from a panic wasn't canceled, and observer code shouldn't run while panicking.
        if active.scope.is_none() || std::thread::panicking() {
            return;
        }
        if let Some(current) = active.current.take() {
            if let Some(trace) = &current.trace {
                trace.settle(&mut active.timings, None);
            }
            active.response = None;
            active.attempt_end(&current.request, None, Some(ErrorKind::Canceled), None);
        }
        active.response = None;
        active.finish(None, None, Some(ErrorKind::Canceled));
    }
}

/// Calls observer code, which must never break a request. A panic is reported by the panic hook as usual and
/// otherwise ignored, since the crate doesn't log.
fn guarded<R>(f: impl FnOnce() -> R) -> Option<R> {
    catch_unwind(AssertUnwindSafe(f)).ok()
}

/// A copy of a request for observers, which shares its body.
fn copy_request(request: &reqwest::Request) -> reqwest::Request {
    request.try_clone().unwrap_or_else(|| {
        let mut copy = reqwest::Request::new(request.method().clone(), request.url().clone());
        *copy.headers_mut() = request.headers().clone();
        *copy.version_mut() = request.version();
        copy
    })
}

/// How far an attempt got before it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Unknown,
    Pool,
    Connect,
    Write,
    Read,
}

fn phase(timings: &Timings, traced_body: bool) -> Phase {
    if timings.send_end.is_some() {
        Phase::Read
    } else if timings.send_begin.is_some() {
        Phase::Write
    } else if !traced_body {
        Phase::Unknown
    } else if timings.connect_begin.is_some() && timings.connect_end.is_none() {
        Phase::Connect
    } else {
        // The body would have been taken once the request had a connection.
        Phase::Pool
    }
}

fn error_kind(err: &reqwest::Error, phase: Phase) -> ErrorKind {
    let timeout = err.is_timeout();
    if err.is_connect() {
        return if timeout {
            ErrorKind::ConnectTimeout
        } else {
            ErrorKind::ConnectError
        };
    }
    if err.is_body() || err.is_decode() {
        return if timeout {
            ErrorKind::ReadTimeout
        } else {
            ErrorKind::ReadError
        };
    }
    match (phase, timeout) {
        (Phase::Pool, true) => ErrorKind::PoolTimeout,
        (Phase::Connect, true) => ErrorKind::ConnectTimeout,
        (Phase::Write, true) => ErrorKind::WriteTimeout,
        (Phase::Read, true) => ErrorKind::ReadTimeout,
        (Phase::Unknown, true) => ErrorKind::Timeout,
        (Phase::Read, false) => ErrorKind::ReadError,
        (_, false) => ErrorKind::Other,
    }
}

tokio::task_local! {
    /// The trace of the attempt whose request is being polled, which the connector records new connections in.
    static ATTEMPT: Arc<Trace>;
}

/// Sends a request with `trace` as the attempt that connections opened while polling it are attributed to.
pub(crate) async fn traced<F: Future>(trace: Arc<Trace>, send: F) -> F::Output {
    ATTEMPT.scope(trace, send).await
}

/// What the client's connector and the request body record about an attempt. Connections the attempt opened may
/// finish after it got another one, so the connection it used is matched by its addresses once a response arrives.
#[derive(Debug, Default)]
pub(crate) struct Trace(Mutex<TraceState>);

#[derive(Debug, Default)]
struct TraceState {
    connects: Vec<Connect>,
    send_begin: Option<Instant>,
    send_end: Option<Instant>,
    /// Set once the attempt is over, after which late marks are ignored.
    done: bool,
}

#[derive(Debug)]
struct Connect {
    start: Instant,
    end: Option<Instant>,
    /// The connection's local and remote addresses.
    addrs: Option<(SocketAddr, SocketAddr)>,
}

impl Trace {
    fn state(&self) -> MutexGuard<'_, TraceState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn connect_begin(&self) -> Option<usize> {
        let mut state = self.state();
        if state.done {
            return None;
        }
        state.connects.push(Connect {
            start: Instant::now(),
            end: None,
            addrs: None,
        });
        Some(state.connects.len() - 1)
    }

    fn connect_end(&self, index: usize, addrs: Option<(SocketAddr, SocketAddr)>) {
        let now = Instant::now();
        let mut state = self.state();
        if state.done {
            return;
        }
        if let Some(connect) = state.connects.get_mut(index) {
            connect.end = Some(now);
            connect.addrs = addrs;
        }
    }

    fn send_begin(&self) {
        let now = Instant::now();
        let mut state = self.state();
        if !state.done {
            state.send_begin.get_or_insert(now);
        }
    }

    fn send_end(&self) {
        let now = Instant::now();
        let mut state = self.state();
        if !state.done {
            state.send_end = Some(now);
        }
    }

    /// Ends the trace, copying its marks into `timings`. `used` is the addresses of the connection a response
    /// arrived on, if known.
    fn settle(&self, timings: &mut Timings, used: Option<(SocketAddr, SocketAddr)>) {
        let mut state = self.state();
        state.done = true;
        timings.send_begin = state.send_begin;
        timings.send_end = state.send_end;
        let connect = match used {
            Some(used) => {
                let opened = state
                    .connects
                    .iter()
                    .find(|connect| connect.addrs == Some(used));
                timings.reused = Some(opened.is_none());
                opened
            }
            // Without a response the connection is unknown, so this shows how far opening one got.
            None => state.connects.last(),
        };
        if let Some(connect) = connect {
            timings.connect_begin = Some(connect.start);
            timings.connect_end = connect.end;
        }
    }
}

fn connection_addrs(extensions: &http::Extensions) -> Option<(SocketAddr, SocketAddr)> {
    extensions
        .get::<HttpInfo>()
        .map(|info| (info.local_addr(), info.remote_addr()))
}

/// A request body that marks when the connection starts and finishes taking it. It's handed over in chunks, so
/// the connection only takes the last one once it has written most of the others.
struct TracedBody {
    remaining: Bytes,
    started: bool,
    trace: Arc<Trace>,
}

const SEND_CHUNK_SIZE: usize = 16 * 1024;

impl http_body::Body for TracedBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        let this = &mut *self;
        if this.remaining.is_empty() {
            return Poll::Ready(None);
        }
        if !this.started {
            this.started = true;
            this.trace.send_begin();
        }
        let chunk = this
            .remaining
            .split_to(this.remaining.len().min(SEND_CHUNK_SIZE));
        if this.remaining.is_empty() {
            this.trace.send_end();
        }
        Poll::Ready(Some(Ok(Frame::data(chunk))))
    }

    fn is_end_stream(&self) -> bool {
        self.remaining.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.remaining.len() as u64)
    }
}

/// Traces the connections the client opens for observed attempts. Install it with
/// `reqwest::ClientBuilder::connector_layer`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConnectLayer;

impl<S> tower_layer::Layer<S> for ConnectLayer {
    type Service = TracedConnector<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracedConnector(inner)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TracedConnector<S>(S);

impl<S, Target> tower_service::Service<Target> for TracedConnector<S>
where
    S: tower_service::Service<Target>,
    S::Response: Connection + Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, target: Target) -> Self::Future {
        // The pool starts connecting while it polls the request that needs the connection.
        let traced = ATTEMPT
            .try_with(|trace| trace.connect_begin().map(|index| (trace.clone(), index)))
            .ok()
            .flatten();
        let connecting = self.0.call(target);
        let Some((trace, index)) = traced else {
            return Box::pin(connecting);
        };
        Box::pin(async move {
            let result = connecting.await;
            if let Ok(connection) = &result {
                let mut extensions = http::Extensions::new();
                connection.connected().get_extras(&mut extensions);
                trace.connect_end(index, connection_addrs(&extensions));
            }
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use serde_json::{Value, json};
    use wiremock::matchers::{header, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::client::Client;
    use crate::transport::{ApiRequest, RequestOptions};

    /// What a recording observer saw.
    #[derive(Debug, Clone, PartialEq)]
    enum Event {
        Start {
            method: String,
            url: String,
        },
        AttemptBegin(u32),
        AttemptEnd {
            attempt: u32,
            status: Option<u16>,
            error: bool,
            kind: Option<ErrorKind>,
            reached_server: Option<bool>,
        },
        Finish {
            retries: u32,
            status: Option<u16>,
            error: bool,
            kind: Option<ErrorKind>,
            parsed: bool,
        },
    }

    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<Event>>,
        timings: Mutex<Vec<Timings>>,
        finished: Mutex<Vec<Timings>>,
    }

    impl Recorder {
        fn events(&self) -> Vec<Event> {
            self.events.lock().unwrap().clone()
        }

        fn attempts(&self) -> Vec<Timings> {
            self.timings.lock().unwrap().clone()
        }

        fn finished(&self) -> Timings {
            let finished = self.finished.lock().unwrap();
            assert_eq!(finished.len(), 1, "finished more than once");
            finished[0].clone()
        }
    }

    /// Records every call, and sets a header in `attempt_begin`.
    #[derive(Clone, Default)]
    struct RecordingObserver(Arc<Recorder>);

    impl RequestObserver for RecordingObserver {
        fn start(&self, info: &RequestInfo<'_>) -> Option<Box<dyn RequestScope>> {
            self.0.events.lock().unwrap().push(Event::Start {
                method: info.method.to_string(),
                url: info.url.to_owned(),
            });
            Some(Box::new(self.clone()))
        }
    }

    impl RequestScope for RecordingObserver {
        fn attempt_begin(&mut self, attempt: u32, request: &mut reqwest::Request) {
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::AttemptBegin(attempt));
            request
                .headers_mut()
                .insert("x-observed", attempt.to_string().parse().unwrap());
        }

        fn attempt_end(&mut self, result: &AttemptResult<'_>) {
            assert_eq!(
                result.request.headers()["x-observed"],
                result.attempt.to_string()
            );
            self.0.events.lock().unwrap().push(Event::AttemptEnd {
                attempt: result.attempt,
                status: result.response.map(|response| response.status().as_u16()),
                error: result.error.is_some(),
                kind: result.error_kind,
                reached_server: result.reached_server,
            });
            self.0.timings.lock().unwrap().push(result.timings.clone());
        }

        fn finish(&mut self, result: &RequestResult<'_>) {
            self.0.events.lock().unwrap().push(Event::Finish {
                retries: result.retries,
                status: result.response.map(|response| response.status().as_u16()),
                error: result.error.is_some(),
                kind: result.error_kind,
                parsed: result.parsed.is_some(),
            });
            self.0.finished.lock().unwrap().push(result.timings.clone());
        }
    }

    fn observed(server: &str) -> (Client, Arc<Recorder>) {
        let observer = RecordingObserver::default();
        let recorder = observer.0.clone();
        (
            Client::for_tests_observed(server, Arc::new(observer)),
            recorder,
        )
    }

    fn get(path: &str) -> ApiRequest {
        ApiRequest::new(Method::GET, path.to_owned())
    }

    fn post(path: &str) -> ApiRequest {
        ApiRequest::new(Method::POST, path.to_owned())
            .json(&json!({ "name": "thing" }))
            .unwrap()
    }

    fn with_options(
        request: ApiRequest,
        timeout: Option<Duration>,
        max_retries: u32,
    ) -> ApiRequest {
        request.options(RequestOptions {
            timeout,
            max_retries: Some(max_retries),
            ..Default::default()
        })
    }

    async fn server_with(template: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(template)
            .mount(&server)
            .await;
        server
    }

    fn start(method: &str, url: &str) -> Event {
        Event::Start {
            method: method.to_owned(),
            url: url.to_owned(),
        }
    }

    fn response_end(attempt: u32, status: u16) -> Event {
        Event::AttemptEnd {
            attempt,
            status: Some(status),
            error: false,
            kind: None,
            reached_server: Some(true),
        }
    }

    fn ordered(marks: &[Option<Instant>]) {
        let marks: Vec<Instant> = marks
            .iter()
            .map(|mark| mark.expect("missing mark"))
            .collect();
        assert!(marks.windows(2).all(|pair| pair[0] <= pair[1]), "{marks:?}");
    }

    #[tokio::test]
    async fn observes_a_request() {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .and(header("x-observed", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .expect(1)
            .mount(&server)
            .await;
        let (client, recorder) = observed(&server.uri());

        let request = post("/thing").query(vec![("limit".to_owned(), "10".to_owned())]);
        let response: Value = client.execute(request).await.unwrap();
        assert_eq!(response, json!({ "ok": true }));

        assert_eq!(
            recorder.events(),
            [
                // Query params added from request params aren't part of the URL.
                start("POST", "/thing"),
                Event::AttemptBegin(0),
                response_end(0, 200),
                Event::Finish {
                    retries: 0,
                    status: Some(200),
                    error: false,
                    kind: None,
                    parsed: true,
                },
            ]
        );
        let t = recorder.finished();
        ordered(&[
            t.attempt_begin,
            t.serialize_begin,
            t.serialize_end,
            t.request_ready,
            t.connect_begin,
            t.connect_end,
            t.send_begin,
            t.send_end,
            t.response_headers,
            t.body_end,
            t.deserialize_begin,
            t.deserialize_end,
        ]);
        assert_eq!(t.reused, Some(false));
    }

    #[tokio::test]
    async fn reports_reused_connections() {
        let server = server_with(ResponseTemplate::new(200).set_body_json(json!({}))).await;
        let (client, recorder) = observed(&server.uri());

        client.execute::<Value>(get("/thing")).await.unwrap();
        client.execute::<Value>(get("/thing")).await.unwrap();

        let attempts = recorder.attempts();
        assert_eq!(attempts[0].reused, Some(false));
        assert!(attempts[0].connect_begin.is_some() && attempts[0].connect_end.is_some());
        assert_eq!(attempts[1].reused, Some(true));
        assert!(attempts[1].connect_begin.is_none() && attempts[1].connect_end.is_none());
        // Sending is traced through the request body, and these have none.
        assert!(attempts[1].send_begin.is_none());
    }

    #[tokio::test]
    async fn adds_the_overload_to_the_url() {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;
        let (client, recorder) = observed(&server.uri());

        client
            .execute::<Value>(post("/thing").overload("other"))
            .await
            .unwrap();

        assert_eq!(
            recorder.events()[0],
            start("POST", "/thing?stainless_overload=other")
        );
        // It isn't sent.
        let received = &server.received_requests().await.unwrap()[0];
        assert_eq!(received.url.query(), None);
    }

    #[tokio::test]
    async fn observes_retries() {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .with_priority(2)
            .mount(&server)
            .await;
        let (client, recorder) = observed(&server.uri());

        client.execute::<Value>(post("/thing")).await.unwrap();

        assert_eq!(
            recorder.events()[1..],
            [
                Event::AttemptBegin(0),
                response_end(0, 503),
                Event::AttemptBegin(1),
                response_end(1, 200),
                Event::Finish {
                    retries: 1,
                    status: Some(200),
                    error: false,
                    kind: None,
                    parsed: true,
                },
            ]
        );
        let headers: Vec<String> = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| request.headers["x-observed"].to_str().unwrap().to_owned())
            .collect();
        assert_eq!(headers, ["0", "1"]);
        // Serializing happened before the first attempt.
        let attempts = recorder.attempts();
        assert!(attempts[0].serialize_begin.is_some());
        assert!(attempts[1].serialize_begin.is_none());
    }

    #[tokio::test]
    async fn observes_error_responses() {
        let server = server_with(ResponseTemplate::new(404).set_body_json(json!({}))).await;
        let (client, recorder) = observed(&server.uri());

        let err = client.execute::<Value>(get("/thing")).await.unwrap_err();
        assert!(matches!(err, Error::Api(_)));

        assert_eq!(
            recorder.events()[1..],
            [
                Event::AttemptBegin(0),
                response_end(0, 404),
                Event::Finish {
                    retries: 0,
                    status: Some(404),
                    error: true,
                    kind: None,
                    parsed: false,
                },
            ]
        );
        assert!(recorder.finished().body_end.is_some());
    }

    #[tokio::test]
    async fn observes_connection_errors() {
        // A port that's free, so connecting to it fails.
        let addr = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let (client, recorder) = observed(&format!("http://{addr}"));

        client
            .execute::<Value>(with_options(get("/thing"), None, 1))
            .await
            .unwrap_err();

        let failed = |attempt| Event::AttemptEnd {
            attempt,
            status: None,
            error: true,
            kind: Some(ErrorKind::ConnectError),
            reached_server: Some(false),
        };
        assert_eq!(
            recorder.events()[1..],
            [
                Event::AttemptBegin(0),
                failed(0),
                Event::AttemptBegin(1),
                failed(1),
                Event::Finish {
                    retries: 1,
                    status: None,
                    error: true,
                    kind: Some(ErrorKind::ConnectError),
                    parsed: false,
                },
            ]
        );
        let t = recorder.finished();
        assert!(t.connect_begin.is_some() && t.connect_end.is_none());
    }

    #[tokio::test]
    async fn observes_timeouts() {
        let server =
            server_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5))).await;
        let (client, recorder) = observed(&server.uri());
        let timeout = Some(Duration::from_millis(50));

        // The request was sent, since its body was taken.
        let err = client
            .execute::<Value>(with_options(post("/thing"), timeout, 0))
            .await
            .unwrap_err();
        assert!(err.is_timeout());
        // Without a body, how far the request got is unknown.
        client
            .execute::<Value>(with_options(get("/thing"), timeout, 0))
            .await
            .unwrap_err();

        let kinds: Vec<_> = recorder
            .events()
            .into_iter()
            .filter_map(|event| match event {
                Event::AttemptEnd {
                    kind,
                    reached_server,
                    ..
                } => Some((kind, reached_server)),
                Event::Finish { kind, .. } => Some((kind, None)),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            [
                (Some(ErrorKind::ReadTimeout), None),
                (Some(ErrorKind::ReadTimeout), None),
                (Some(ErrorKind::Timeout), None),
                (Some(ErrorKind::Timeout), None),
            ]
        );
    }

    #[tokio::test]
    async fn observes_canceled_requests() {
        let server =
            server_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5))).await;
        let (client, recorder) = observed(&server.uri());

        let request = client.execute::<Value>(get("/thing"));
        tokio::time::timeout(Duration::from_millis(50), request)
            .await
            .unwrap_err();

        assert_eq!(
            recorder.events()[1..],
            [
                Event::AttemptBegin(0),
                Event::AttemptEnd {
                    attempt: 0,
                    status: None,
                    error: false,
                    kind: Some(ErrorKind::Canceled),
                    reached_server: None,
                },
                Event::Finish {
                    retries: 0,
                    status: None,
                    error: false,
                    kind: Some(ErrorKind::Canceled),
                    parsed: false,
                },
            ]
        );
    }

    #[tokio::test]
    async fn finishes_once_when_parsing_fails() {
        let server = server_with(ResponseTemplate::new(200).set_body_json(json!({}))).await;
        let (client, recorder) = observed(&server.uri());

        let err = client
            .execute::<Vec<String>>(get("/thing"))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Decode { .. }));

        let events = recorder.events();
        assert_eq!(
            events.last(),
            Some(&Event::Finish {
                retries: 0,
                status: Some(200),
                error: true,
                kind: Some(ErrorKind::Other),
                parsed: false,
            })
        );
        // Dropping the finished observation doesn't finish it again.
        recorder.finished();
    }

    struct PanickingObserver(Arc<AtomicUsize>);

    impl RequestObserver for PanickingObserver {
        fn start(&self, _info: &RequestInfo<'_>) -> Option<Box<dyn RequestScope>> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("start");
            }
            Some(Box::new(PanickingScope))
        }
    }

    struct PanickingScope;

    impl RequestScope for PanickingScope {
        fn attempt_begin(&mut self, _attempt: u32, _request: &mut reqwest::Request) {
            panic!("attempt_begin");
        }

        fn attempt_end(&mut self, _result: &AttemptResult<'_>) {
            panic!("attempt_end");
        }

        fn finish(&mut self, _result: &RequestResult<'_>) {
            panic!("finish");
        }
    }

    #[tokio::test]
    async fn ignores_observer_panics() {
        let server =
            server_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true }))).await;
        let starts = Arc::new(AtomicUsize::new(0));
        let client =
            Client::for_tests_observed(&server.uri(), Arc::new(PanickingObserver(starts.clone())));

        for _ in 0..2 {
            let response: Value = client.execute(post("/thing")).await.unwrap();
            assert_eq!(response, json!({ "ok": true }));
        }
        assert_eq!(starts.load(Ordering::SeqCst), 2);
    }

    struct Unobserved(Arc<AtomicUsize>);

    impl RequestObserver for Unobserved {
        fn start(&self, _info: &RequestInfo<'_>) -> Option<Box<dyn RequestScope>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            None
        }
    }

    #[tokio::test]
    async fn leaves_requests_unobserved_without_a_scope() {
        let server = server_with(ResponseTemplate::new(200).set_body_json(json!({}))).await;
        let starts = Arc::new(AtomicUsize::new(0));
        let client =
            Client::for_tests_observed(&server.uri(), Arc::new(Unobserved(starts.clone())));

        client.execute::<Value>(post("/thing")).await.unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert!(!client.observe(&post("/thing")).is_active());
        // A client without an observer observes nothing.
        let client = Client::for_tests(&server.uri());
        assert!(!client.observe(&post("/thing")).is_active());
    }

    #[test]
    fn classifies_timeouts_by_phase() {
        let mut timings = Timings::default();
        assert_eq!(phase(&timings, false), Phase::Unknown);
        assert_eq!(phase(&timings, true), Phase::Pool);
        timings.connect_begin = Some(Instant::now());
        assert_eq!(phase(&timings, true), Phase::Connect);
        timings.connect_end = Some(Instant::now());
        assert_eq!(phase(&timings, true), Phase::Pool);
        timings.send_begin = Some(Instant::now());
        assert_eq!(phase(&timings, false), Phase::Write);
        timings.send_end = Some(Instant::now());
        assert_eq!(phase(&timings, false), Phase::Read);
    }

    #[test]
    fn names_error_kinds() {
        assert_eq!(ErrorKind::PoolTimeout.as_str(), "pool_timeout");
        assert!(ErrorKind::PoolTimeout.is_timeout());
        assert!(!ErrorKind::Canceled.is_timeout());
        assert!(!ErrorKind::ConnectError.is_timeout());
    }
}
