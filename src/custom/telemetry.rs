//! Client telemetry reported to turbopuffer on later requests.
//!
//! When enabled, the client buffers a small timing record for each request and attaches buffered records to a
//! later request in the `X-turbopuffer-Client-Telemetry` header. The server controls how many requests are
//! recorded by advertising a sample rate in the `X-turbopuffer-Client-Telemetry-Sample-Rate` response header.

use std::any::Any;
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, Weak};
use std::time::{Duration, Instant};

use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;

use crate::observer::{
    AttemptResult, ErrorKind, RequestInfo, RequestObserver, RequestResult, RequestScope, Timings,
};
use crate::types::{MultiQueryResult, QueryResult, WriteResult};

const ENV_VAR: &str = "TURBOPUFFER_TELEMETRY";
const HEADER: &str = "x-turbopuffer-client-telemetry";
const SAMPLE_RATE_HEADER: &str = "x-turbopuffer-client-telemetry-sample-rate";

const VERSION: u32 = 1;

// Keeps the header well under the 8 KB per-line limit of common proxies.
const MAX_RECORDS_PER_HEADER: usize = 20;
const MAX_BUFFERED_RECORDS: usize = 1000;

/// Returns the observer that records telemetry for a client, or `None` if telemetry is disabled. `enabled`
/// overrides the environment variable read with `env`.
pub(crate) fn observer(
    enabled: Option<bool>,
    env: impl Fn(&str) -> Option<String>,
    api_key: &str,
    base_url: &str,
) -> Option<Arc<dyn RequestObserver>> {
    if !enabled.unwrap_or_else(|| enabled_by_env(env(ENV_VAR).as_deref())) {
        return None;
    }
    Some(Arc::new(TelemetryObserver {
        buffer: shared_buffer(api_key, base_url),
    }))
}

/// Whether the environment variable's value enables telemetry. Invalid values disable it, silently since the
/// crate doesn't log.
fn enabled_by_env(value: Option<&str>) -> bool {
    let value = value.unwrap_or_default().trim().to_ascii_lowercase();
    matches!(value.as_str(), "1" | "true" | "yes" | "on")
}

/// Returns the buffer shared by clients with the same API key and base URL. Sharing lets records ride on any
/// client's requests, but never to another org or region. A buffer lives as long as a client uses it.
fn shared_buffer(api_key: &str, base_url: &str) -> Arc<Buffer> {
    /// Buffers by API key and base URL.
    type Buffers = HashMap<(String, String), Weak<Buffer>>;
    static BUFFERS: OnceLock<Mutex<Buffers>> = OnceLock::new();
    let mut buffers = lock(BUFFERS.get_or_init(Mutex::default));
    let key = (api_key.to_owned(), base_url.to_owned());
    if let Some(buffer) = buffers.get(&key).and_then(Weak::upgrade) {
        return buffer;
    }
    buffers.retain(|_, buffer| buffer.strong_count() > 0);
    let buffer = Arc::new(Buffer::new());
    buffers.insert(key, Arc::downgrade(&buffer));
    buffer
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Returns the method and path of a request with identifiers like the namespace replaced.
fn route_template(method: &Method, url: &str) -> String {
    // Only the path and query of an absolute URL identify the route.
    let absolute = url
        .contains("://")
        .then(|| reqwest::Url::parse(url).ok())
        .flatten();
    let path = match &absolute {
        Some(absolute) => match absolute.query() {
            Some(query) => format!("{}?{query}", absolute.path()),
            None => absolute.path().to_owned(),
        },
        None => url.to_owned(),
    };
    let path = template_operation(&template_namespace(&path));
    format!("{} {path}", method.as_str().to_ascii_uppercase())
}

/// Replaces the namespace in a path like `/v2/namespaces/{ns}/query`.
fn template_namespace(path: &str) -> String {
    let templated = || {
        let rest = path.strip_prefix("/v")?;
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let name = rest[digits..].strip_prefix("/namespaces/")?;
        let len = name.find(['/', '?']).unwrap_or(name.len());
        if digits == 0 || len == 0 {
            return None;
        }
        let prefix = &path[..path.len() - name.len()];
        Some(format!("{prefix}{{ns}}{}", &name[len..]))
    };
    templated().unwrap_or_else(|| path.to_owned())
}

/// Replaces operation tokens in a path like `/v1/namespaces/{ns}/operations/{token}`.
fn template_operation(path: &str) -> String {
    const SEGMENT: &str = "/operations/";
    let mut templated = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(index) = rest.find(SEGMENT) {
        let (head, tail) = rest.split_at(index + SEGMENT.len());
        templated.push_str(head);
        let len = tail.find(['/', '?']).unwrap_or(tail.len());
        if len > 0 {
            templated.push_str("{token}");
        }
        rest = &tail[len..];
    }
    templated.push_str(rest);
    templated
}

/// The telemetry of one request. Durations are in whole microseconds, and are left out when their phase wasn't
/// observed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
struct Record {
    route: String,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'static str>,
    #[serde(skip_serializing_if = "is_zero")]
    retries: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    serialize_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compress_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pool_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connect_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wait_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body_read_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deserialize_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_total_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reused: Option<bool>,
    /// Set when the record is taken to be sent.
    age_ms: u64,
}

fn is_zero<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl Record {
    /// Fills in the duration of each phase of an attempt ending at `end`.
    fn set_durations(&mut self, timings: &Timings, end: Instant) {
        let Some(attempt_start) = timings.attempt_start else {
            return;
        };
        let micros = |duration: Duration| u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
        let between = |start: Option<Instant>, end: Option<Instant>| {
            Some(micros(end?.saturating_duration_since(start?)))
        };

        self.total_us = Some(micros(end.saturating_duration_since(attempt_start)));
        self.serialize_us = between(timings.serialize_start, timings.serialize_end);

        // Set by transports that compress request bodies. Zero means the body wasn't compressed.
        let compress_start = timings.extra.get("compress_start").copied();
        let compress_end = timings.extra.get("compress_end").copied();
        self.compress_us = between(compress_start, compress_end).filter(|us| *us > 0);

        // Waiting for a free connection, until connecting a new one or sending on a reused one.
        let pool_start = compress_end.or(timings.request_ready);
        let connection_ready = timings.connect_start.or(timings.send_start);
        self.pool_us = between(pool_start, connection_ready);

        self.connect_us = between(timings.connect_start, timings.connect_end);
        self.send_us = between(timings.send_start, timings.send_end);
        // Covers the network round trip and the server.
        self.wait_us = between(timings.send_end, timings.response_headers);
        self.body_read_us = between(timings.response_headers, timings.body_end);
        self.deserialize_us = between(timings.deserialize_start, timings.deserialize_end);
    }
}

/// `performance.server_total_ms` of a parsed response, from the response types that have it.
fn server_total_ms(parsed: &dyn Any) -> Option<u64> {
    if let Some(result) = parsed.downcast_ref::<QueryResult>() {
        return Some(result.performance.server_total_ms);
    }
    if let Some(result) = parsed.downcast_ref::<MultiQueryResult>() {
        return Some(result.performance.server_total_ms);
    }
    if let Some(result) = parsed.downcast_ref::<WriteResult>() {
        return result
            .performance
            .as_ref()
            .map(|performance| performance.server_total_ms);
    }
    None
}

/// The error reported for a request that failed without an HTTP status.
fn error_name(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Canceled => "canceled",
        kind if kind.is_timeout() => "timeout",
        _ => "connect",
    }
}

#[derive(Debug)]
struct Entry {
    recorded_at: Instant,
    record: Record,
}

/// Records taken from a buffer, with the header value carrying them.
#[derive(Debug)]
struct Batch {
    entries: Vec<Entry>,
    dropped: u64,
    header: Option<HeaderValue>,
}

#[derive(Serialize)]
struct Envelope {
    version: u32,
    records: Vec<Record>,
    #[serde(skip_serializing_if = "is_zero")]
    dropped: u64,
}

impl Batch {
    fn new(entries: Vec<Entry>, dropped: u64) -> Self {
        let now = Instant::now();
        let records = entries
            .iter()
            .map(|entry| {
                let age = now.saturating_duration_since(entry.recorded_at);
                Record {
                    age_ms: (age.as_secs_f64() * 1000.0).round() as u64,
                    ..entry.record.clone()
                }
            })
            .collect();
        let envelope = Envelope {
            version: VERSION,
            records,
            dropped,
        };
        // Records only hold ASCII, so a header that can't be encoded would be a bug.
        let header = serde_json::to_string(&envelope)
            .ok()
            .and_then(|header| HeaderValue::try_from(header).ok());
        Self {
            entries,
            dropped,
            header,
        }
    }
}

/// Buffers per-request telemetry records until a later request can carry them.
#[derive(Debug)]
struct Buffer {
    state: Mutex<BufferState>,
    /// The bits of the `f64` share of requests to record, which is 1 until the server advertises one.
    sample_rate: AtomicU64,
    /// Indirection so tests can control sampling.
    random: fn() -> f64,
}

#[derive(Debug, Default)]
struct BufferState {
    entries: VecDeque<Entry>,
    dropped: u64,
}

impl Buffer {
    fn new() -> Self {
        Self {
            state: Mutex::default(),
            sample_rate: AtomicU64::new(1f64.to_bits()),
            random: random_fraction,
        }
    }

    fn sample_rate(&self) -> f64 {
        f64::from_bits(self.sample_rate.load(Ordering::Relaxed))
    }

    /// Adopts the sample rate advertised by the server, ignoring invalid values.
    fn observe_response(&self, headers: &HeaderMap) {
        let rate = headers
            .get(SAMPLE_RATE_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<f64>().ok());
        // Also rejects NaN, which isn't in any range.
        if let Some(rate) = rate.filter(|rate| (0.0..=1.0).contains(rate)) {
            self.sample_rate.store(rate.to_bits(), Ordering::Relaxed);
        }
    }

    /// Samples and buffers a record, dropping the oldest when full.
    fn record(&self, mut record: Record) {
        let rate = self.sample_rate();
        if rate <= 0.0 || (rate < 1.0 && (self.random)() >= rate) {
            return;
        }
        if rate < 1.0 {
            record.sample_rate = Some(rate);
        }
        let mut state = lock(&self.state);
        if state.entries.len() >= MAX_BUFFERED_RECORDS {
            state.entries.pop_front();
            state.dropped += 1;
        }
        state.entries.push_back(Entry {
            recorded_at: Instant::now(),
            record,
        });
    }

    /// Removes up to `MAX_RECORDS_PER_HEADER` of the oldest records.
    fn take(&self) -> Option<Batch> {
        let mut state = lock(&self.state);
        if state.entries.is_empty() {
            return None;
        }
        let count = state.entries.len().min(MAX_RECORDS_PER_HEADER);
        let entries: Vec<Entry> = state.entries.drain(..count).collect();
        let dropped = std::mem::take(&mut state.dropped);
        drop(state);
        Some(Batch::new(entries, dropped))
    }

    /// Returns a batch that never reached the server to the front of the buffer.
    fn restore(&self, batch: Batch) {
        let mut state = lock(&self.state);
        state.dropped += batch.dropped;
        for entry in batch.entries.into_iter().rev() {
            // The batch holds the oldest records, so they're the ones to drop when full.
            if state.entries.len() >= MAX_BUFFERED_RECORDS {
                state.dropped += 1;
                continue;
            }
            state.entries.push_front(entry);
        }
    }
}

/// A random number in `[0, 1)`, good enough for sampling without pulling in an RNG crate.
fn random_fraction() -> f64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default(),
    );
    (hasher.finish() >> 11) as f64 / (1u64 << 53) as f64
}

/// Records telemetry for a client's requests.
#[derive(Debug)]
struct TelemetryObserver {
    buffer: Arc<Buffer>,
}

impl RequestObserver for TelemetryObserver {
    fn start(&self, info: &RequestInfo<'_>) -> Option<Box<dyn RequestScope>> {
        Some(Box::new(TelemetryScope {
            buffer: self.buffer.clone(),
            route: route_template(info.method, info.url),
            batch: None,
        }))
    }
}

struct TelemetryScope {
    buffer: Arc<Buffer>,
    route: String,
    /// Kept for later attempts until one of them may have reached the server.
    batch: Option<Batch>,
}

impl RequestScope for TelemetryScope {
    fn attempt_begin(&mut self, attempt: u32, request: &mut reqwest::Request) {
        // Taken only once a request is about to be sent, so one that fails before then can't lose it.
        if attempt == 0 {
            self.batch = self.buffer.take();
        }
        if let Some(header) = self.batch.as_ref().and_then(|batch| batch.header.clone()) {
            request.headers_mut().insert(HEADER, header);
        }
    }

    fn attempt_end(&mut self, result: &AttemptResult<'_>) {
        if result.reached_server != Some(false) {
            self.batch = None;
        }
        if let Some(response) = result.response {
            self.buffer.observe_response(response.headers());
        }
    }

    fn finish(&mut self, result: &RequestResult<'_>) {
        // A batch that never reached the server goes back for a later request.
        if let Some(batch) = self.batch.take() {
            self.buffer.restore(batch);
        }
        let mut record = Record {
            route: std::mem::take(&mut self.route),
            status: result
                .response
                .map_or(0, |response| response.status().as_u16()),
            error: result.error_kind.map(error_name),
            retries: result.retries,
            reused: result.timings.reused,
            server_total_us: result.parsed.and_then(server_total_ms).map(|ms| ms * 1000),
            ..Record::default()
        };
        let end = result.timings.deserialize_end.unwrap_or_else(Instant::now);
        record.set_durations(result.timings, end);
        self.buffer.record(record);
    }
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, TcpListener};
    use std::time::Duration;

    use futures::TryStreamExt;
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use super::*;
    use crate::{Client, Error};

    const API_KEY: &str = "tpuf_test";
    const QUERY_ROUTE: &str = "POST /v2/namespaces/{ns}/query";

    fn query_body() -> Value {
        json!({
            "rows": [],
            "billing": { "billable_logical_bytes_queried": 0, "billable_logical_bytes_returned": 0 },
            "performance": {
                "approx_namespace_size": 1,
                "cache_hit_ratio": 1.0,
                "cache_temperature": "hot",
                "exhaustive_search_count": 0,
                "query_execution_ms": 3,
                "server_total_ms": 7,
            },
        })
    }

    fn client(server: &str) -> Client {
        Client::builder()
            .api_key(API_KEY)
            .base_url(server)
            .telemetry(true)
            .build()
            .unwrap()
    }

    /// A client with telemetry that retries without delay.
    fn retrying_client(server: &str) -> Client {
        let observer = observer(Some(true), |_| None, API_KEY, server).unwrap();
        Client::for_tests_observed(server, observer)
    }

    async fn query(client: &Client) -> crate::Result<crate::types::QueryResult> {
        client.namespace("my-ns").query().top_k(1).send().await
    }

    fn buffer(server: &str) -> Arc<Buffer> {
        shared_buffer(API_KEY, server)
    }

    /// The records in a request's telemetry header.
    fn sent_records(request: &Request) -> Option<Vec<Value>> {
        let header = request.headers.get(HEADER)?.to_str().unwrap();
        let envelope: Value = serde_json::from_str(header).unwrap();
        assert_eq!(envelope["version"], 1);
        Some(envelope["records"].as_array().unwrap().clone())
    }

    async fn received(server: &MockServer) -> Vec<Option<Vec<Value>>> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(sent_records)
            .collect()
    }

    fn take_records(buffer: &Buffer) -> Vec<Value> {
        let batch = buffer.take().unwrap();
        let envelope: Value =
            serde_json::from_str(batch.header.unwrap().to_str().unwrap()).unwrap();
        envelope["records"].as_array().unwrap().clone()
    }

    async fn query_server(template: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/namespaces/my-ns/query"))
            .respond_with(template)
            .mount(&server)
            .await;
        server
    }

    fn record(status: u16) -> Record {
        Record {
            route: QUERY_ROUTE.to_owned(),
            status,
            total_us: Some(1),
            ..Record::default()
        }
    }

    #[test]
    fn reads_the_environment_variable() {
        for value in ["1", "true", "TRUE", " yes ", "on"] {
            assert!(enabled_by_env(Some(value)), "{value}");
        }
        for value in ["", "0", "false", "no", "off", "maybe"] {
            assert!(!enabled_by_env(Some(value)), "{value}");
        }
        assert!(!enabled_by_env(None));
    }

    #[test]
    fn explicit_settings_beat_the_environment() {
        let env =
            |value: &'static str| move |name: &str| (name == ENV_VAR).then(|| value.to_owned());
        assert!(observer(None, env("1"), API_KEY, "https://a").is_some());
        assert!(observer(None, env("0"), API_KEY, "https://a").is_none());
        assert!(observer(Some(false), env("1"), API_KEY, "https://a").is_none());
        assert!(observer(Some(true), env("0"), API_KEY, "https://a").is_some());
        assert!(observer(None, |_| None, API_KEY, "https://a").is_none());
    }

    #[test]
    fn templates_routes() {
        let cases = [
            (Method::POST, "/v2/namespaces/my-ns/query", QUERY_ROUTE),
            (
                Method::POST,
                "/v2/namespaces/my-ns/query?stainless_overload=multiQuery",
                "POST /v2/namespaces/{ns}/query?stainless_overload=multiQuery",
            ),
            (
                Method::POST,
                "/v2/namespaces/my%2Fns",
                "POST /v2/namespaces/{ns}",
            ),
            (
                Method::POST,
                "/v2/namespaces/my-ns?stainless_overload=copyFrom",
                "POST /v2/namespaces/{ns}?stainless_overload=copyFrom",
            ),
            (Method::GET, "/v1/namespaces", "GET /v1/namespaces"),
            (Method::GET, "/v1/namespaces/", "GET /v1/namespaces/"),
            (
                Method::GET,
                "/v1/namespaces/my-ns/operations/abc123?stainless_overload=pollCopyFrom",
                "GET /v1/namespaces/{ns}/operations/{token}?stainless_overload=pollCopyFrom",
            ),
            (
                Method::GET,
                "https://gcp-us-central1.turbopuffer.com/v1/namespaces/my-ns/metadata",
                "GET /v1/namespaces/{ns}/metadata",
            ),
            (
                Method::GET,
                "/other/operations//operations/x",
                "GET /other/operations//operations/{token}",
            ),
        ];
        for (method, url, expected) in cases {
            assert_eq!(route_template(&method, url), expected, "{url}");
        }
    }

    fn timings(marks: &[(&'static str, u64)]) -> (Timings, Instant) {
        let base = Instant::now();
        let at = |micros: u64| Some(base + Duration::from_micros(micros));
        let mut timings = Timings::default();
        for (name, micros) in marks {
            match *name {
                "attempt_start" => timings.attempt_start = at(*micros),
                "request_ready" => timings.request_ready = at(*micros),
                "connect_start" => timings.connect_start = at(*micros),
                "connect_end" => timings.connect_end = at(*micros),
                "send_start" => timings.send_start = at(*micros),
                "send_end" => timings.send_end = at(*micros),
                "response_headers" => timings.response_headers = at(*micros),
                "compress_start" | "compress_end" => {
                    timings.extra.insert(name, at(*micros).unwrap());
                }
                other => panic!("unknown mark {other}"),
            }
        }
        (timings, base)
    }

    #[test]
    fn computes_durations() {
        let (new_connection, base) = timings(&[
            ("attempt_start", 0),
            ("request_ready", 50),
            ("compress_start", 60),
            ("compress_end", 100),
            ("connect_start", 150),
            ("connect_end", 400),
            ("send_start", 410),
            ("send_end", 500),
            ("response_headers", 900),
        ]);
        let mut record = Record::default();
        record.set_durations(&new_connection, base + Duration::from_micros(1000));
        assert_eq!(
            (
                record.total_us,
                record.compress_us,
                record.pool_us,
                record.connect_us,
                record.send_us,
                record.wait_us
            ),
            (
                Some(1000),
                Some(40),
                Some(50),
                Some(250),
                Some(90),
                Some(400)
            )
        );

        let (mut reused, base) = timings(&[
            ("attempt_start", 0),
            ("request_ready", 100),
            ("send_start", 130),
            ("send_end", 160),
            ("response_headers", 300),
        ]);
        let mut record = Record::default();
        record.set_durations(&reused, base + Duration::from_micros(400));
        assert_eq!(
            (record.connect_us, record.pool_us, record.compress_us),
            (None, Some(30), None)
        );

        // Sending can finish after the response headers arrive.
        reused.send_end = Some(base + Duration::from_micros(350));
        record.set_durations(&reused, base + Duration::from_micros(400));
        assert_eq!(record.wait_us, Some(0));

        // Untraced requests, e.g. through a user-supplied HTTP client.
        let (untraced, base) = timings(&[("attempt_start", 0)]);
        let mut record = Record::default();
        record.set_durations(&untraced, base + Duration::from_micros(400));
        assert_eq!(
            record,
            Record {
                total_us: Some(400),
                ..Record::default()
            }
        );

        // Durations are whole microseconds, and a zero compression duration means the body wasn't compressed.
        let mut timings = Timings::default();
        let base = Instant::now();
        timings.attempt_start = Some(base);
        timings.deserialize_start = Some(base);
        timings.deserialize_end = Some(base + Duration::from_nanos(999));
        timings.extra.insert("compress_start", base);
        timings.extra.insert("compress_end", base);
        let mut record = Record::default();
        record.set_durations(&timings, base + Duration::from_nanos(1_234_567));
        assert_eq!(
            (record.total_us, record.deserialize_us, record.compress_us),
            (Some(1234), Some(0), None)
        );
    }

    #[test]
    fn encodes_headers() {
        let buffer = Buffer::new();
        buffer.record(Record {
            retries: 2,
            server_total_us: Some(7000),
            reused: Some(true),
            ..record(200)
        });
        let batch = buffer.take().unwrap();
        let header = batch.header.unwrap();
        let header = header.to_str().unwrap();
        // Compact, with unset fields and a zero dropped count left out.
        assert!(header.starts_with(r#"{"version":1,"records":[{"route":"POST /v2/namespaces/{ns}/query","status":200,"retries":2,"total_us":1,"server_total_us":7000,"reused":true,"age_ms":"#), "{header}");
        assert!(header.ends_with("}]}"), "{header}");
        assert!(buffer.take().is_none());
    }

    #[test]
    fn full_buffers_drop_the_oldest_records() {
        let buffer = Buffer::new();
        for status in 0..(MAX_BUFFERED_RECORDS + 5) as u16 {
            buffer.record(record(status));
        }
        let batch = buffer.take().unwrap();
        let envelope: Value =
            serde_json::from_str(batch.header.as_ref().unwrap().to_str().unwrap()).unwrap();
        assert_eq!(envelope["dropped"], 5);
        let records = envelope["records"].as_array().unwrap();
        assert_eq!(records.len(), MAX_RECORDS_PER_HEADER);
        assert_eq!(records[0]["status"], 5);
        // The dropped count is only sent once.
        let envelope: Value =
            serde_json::from_str(buffer.take().unwrap().header.unwrap().to_str().unwrap()).unwrap();
        assert!(envelope.get("dropped").is_none());
    }

    #[test]
    fn takes_records_when_the_first_attempt_begins() {
        let buffer = Arc::new(Buffer::new());
        buffer.record(record(200));
        let observer = TelemetryObserver {
            buffer: buffer.clone(),
        };
        let url = "/v2/namespaces/my-ns/query";
        let mut scope = observer
            .start(&RequestInfo {
                method: &Method::POST,
                url,
            })
            .unwrap();

        // A request that fails before it's sent never takes them.
        let held = buffer.take().unwrap();
        buffer.restore(held);

        let mut request = reqwest::Request::new(
            Method::POST,
            format!("http://localhost{url}").parse().unwrap(),
        );
        scope.attempt_begin(0, &mut request);
        assert!(request.headers().contains_key(HEADER));
        assert!(buffer.take().is_none());
    }

    #[test]
    fn restores_batches_in_order() {
        let buffer = Buffer::new();
        for status in 0..3 {
            buffer.record(record(status));
        }
        let batch = buffer.take().unwrap();
        buffer.record(record(3));
        buffer.restore(batch);
        let statuses: Vec<Value> = take_records(&buffer)
            .iter()
            .map(|r| r["status"].clone())
            .collect();
        assert_eq!(statuses, [0, 1, 2, 3]);

        // Restoring into a full buffer drops the batch's records, which are the oldest.
        for status in 0..2 {
            buffer.record(record(status));
        }
        let batch = buffer.take().unwrap();
        for _ in 0..MAX_BUFFERED_RECORDS - 1 {
            buffer.record(record(9));
        }
        buffer.restore(batch);
        let envelope: Value =
            serde_json::from_str(buffer.take().unwrap().header.unwrap().to_str().unwrap()).unwrap();
        assert_eq!(envelope["dropped"], 1);
        assert_eq!(envelope["records"][0]["status"], 1);
    }

    #[test]
    fn adopts_valid_sample_rates() {
        let buffer = Buffer::new();
        let observe = |value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(SAMPLE_RATE_HEADER, value.parse().unwrap());
            buffer.observe_response(&headers);
            buffer.sample_rate()
        };
        assert_eq!(observe("0.5"), 0.5);
        for invalid in ["", "abc", "NaN", "-0.1", "1.5", "inf"] {
            assert_eq!(observe(invalid), 0.5, "{invalid}");
        }
        assert_eq!(observe("0"), 0.0);
        assert_eq!(observe(" 1 "), 1.0);
    }

    #[test]
    fn samples_records() {
        let mut buffer = Buffer::new();
        buffer.random = || 0.3;
        buffer
            .sample_rate
            .store(0.5f64.to_bits(), Ordering::Relaxed);
        buffer.record(record(200));
        assert_eq!(take_records(&buffer)[0]["sample_rate"], 0.5);

        buffer
            .sample_rate
            .store(0.25f64.to_bits(), Ordering::Relaxed);
        buffer.record(record(200));
        assert!(buffer.take().is_none());

        buffer.random = || 0.0;
        buffer.sample_rate.store(0f64.to_bits(), Ordering::Relaxed);
        buffer.record(record(200));
        assert!(buffer.take().is_none());

        // A full rate isn't reported.
        buffer.sample_rate.store(1f64.to_bits(), Ordering::Relaxed);
        buffer.record(record(200));
        assert!(take_records(&buffer)[0].get("sample_rate").is_none());
    }

    #[test]
    fn shares_buffers_by_api_key_and_base_url() {
        let a = shared_buffer("key-a", "https://shared.example.com");
        assert!(Arc::ptr_eq(
            &a,
            &shared_buffer("key-a", "https://shared.example.com")
        ));
        assert!(!Arc::ptr_eq(
            &a,
            &shared_buffer("key-b", "https://shared.example.com")
        ));
        assert!(!Arc::ptr_eq(
            &a,
            &shared_buffer("key-a", "https://other.example.com")
        ));

        // A buffer no client uses is dropped.
        let weak = Arc::downgrade(&a);
        drop(a);
        assert!(weak.upgrade().is_none());
    }

    #[tokio::test]
    async fn clients_share_buffers() {
        let server = query_server(ResponseTemplate::new(200).set_body_json(query_body())).await;
        let first = client(&server.uri());
        let copy = first.clone();
        let independent = client(&server.uri());

        query(&first).await.unwrap();
        query(&copy).await.unwrap();
        query(&independent).await.unwrap();

        let sent = received(&server).await;
        assert_eq!(sent[0], None);
        assert_eq!(sent[1].as_ref().unwrap().len(), 1);
        assert_eq!(sent[2].as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn records_ride_on_the_next_request() {
        let server = query_server(ResponseTemplate::new(200).set_body_json(query_body())).await;
        let client = client(&server.uri());

        for _ in 0..3 {
            query(&client).await.unwrap();
        }

        let sent = received(&server).await;
        assert_eq!(sent[0], None);
        let first = &sent[1].as_ref().unwrap()[0];
        let second = &sent[2].as_ref().unwrap()[0];
        assert_eq!(first["route"], QUERY_ROUTE);
        assert_eq!(first["status"], 200);
        assert_eq!(first["server_total_us"], 7000);
        for key in ["error", "retries", "sample_rate", "compress_us"] {
            assert!(first.get(key).is_none(), "{key}");
        }
        // A new connection, then a reused one.
        assert_eq!(first["reused"], false);
        assert_eq!(second["reused"], true);
        assert!(first["connect_us"].is_u64() && second.get("connect_us").is_none());
        for record in [first, second] {
            for key in [
                "total_us",
                "serialize_us",
                "pool_us",
                "send_us",
                "wait_us",
                "body_read_us",
                "deserialize_us",
                "age_ms",
            ] {
                assert!(record[key].is_u64(), "{key} in {record}");
            }
        }
    }

    #[tokio::test]
    async fn disabled_clients_send_nothing() {
        let server = query_server(ResponseTemplate::new(200).set_body_json(query_body())).await;
        // Explicitly, so the test doesn't depend on the environment.
        let client = Client::builder()
            .api_key(API_KEY)
            .base_url(server.uri())
            .telemetry(false)
            .build()
            .unwrap();
        assert!(client.inner.observer.is_none());
        query(&client).await.unwrap();
        query(&client).await.unwrap();
        assert!(received(&server).await.iter().all(Option::is_none));
    }

    #[tokio::test]
    async fn records_failures() {
        let server = MockServer::start().await;
        Mock::given(path("/v2/namespaces/missing/metadata"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({ "error": "not found" })))
            .mount(&server)
            .await;
        Mock::given(path("/v2/namespaces/my-ns/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "unexpected": true })))
            .mount(&server)
            .await;
        let client = client(&server.uri());

        let buffer = buffer(&server.uri());

        client
            .namespace("missing")
            .metadata()
            .send()
            .await
            .unwrap_err();
        let record = &take_records(&buffer)[0];
        assert_eq!(record["route"], "GET /v2/namespaces/{ns}/metadata");
        assert_eq!(record["status"], 404);
        assert!(record.get("error").is_none());

        // A response that couldn't be parsed.
        let err = query(&client).await.unwrap_err();
        assert!(matches!(err, Error::Decode { .. }));
        let record = &take_records(&buffer)[0];
        assert_eq!(
            (&record["status"], &record["error"]),
            (&json!(200), &json!("connect"))
        );
    }

    #[tokio::test]
    async fn records_timeouts_and_cancellations() {
        let server =
            query_server(ResponseTemplate::new(200).set_delay(Duration::from_secs(5))).await;
        let client = client(&server.uri());
        let buffer = buffer(&server.uri());

        let err = client
            .namespace("my-ns")
            .query()
            .timeout(Duration::from_millis(50))
            .max_retries(0)
            .send()
            .await
            .unwrap_err();
        assert!(err.is_timeout());
        let record = &take_records(&buffer)[0];
        assert_eq!(
            (&record["status"], &record["error"]),
            (&json!(0), &json!("timeout"))
        );

        tokio::time::timeout(Duration::from_millis(50), query(&client))
            .await
            .unwrap_err();
        let record = &take_records(&buffer)[0];
        assert_eq!(
            (&record["status"], &record["error"]),
            (&json!(0), &json!("canceled"))
        );
    }

    /// Starts a server on `addr`, or a new address if it's `None`.
    async fn start_on(addr: Option<SocketAddr>) -> MockServer {
        let listener =
            TcpListener::bind(addr.unwrap_or_else(|| "127.0.0.1:0".parse().unwrap())).unwrap();
        let server = MockServer::builder().listener(listener).start().await;
        Mock::given(path("/v2/namespaces/my-ns/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(query_body()))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn resends_batches_after_connection_errors() {
        let server = start_on(None).await;
        let addr = *server.address();
        let uri = server.uri();
        // Keeps the shared buffer alive.
        let first = client(&uri);
        query(&first).await.unwrap();
        drop(server);

        // New clients share the buffer, without connections to reuse. The batch never reached the server, so it's
        // kept along with this request's record.
        let failing = client(&uri);
        let err = failing
            .namespace("my-ns")
            .query()
            .max_retries(0)
            .send()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Connection(_)), "{err:?}");

        let server = start_on(Some(addr)).await;
        query(&client(&uri)).await.unwrap();
        let sent = received(&server).await;
        let records = sent[0].as_ref().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["status"], 200);
        assert_eq!(
            (&records[1]["status"], &records[1]["error"]),
            (&json!(0), &json!("connect"))
        );
    }

    #[tokio::test]
    async fn does_not_resend_batches_that_reached_the_server() {
        let server = MockServer::start().await;
        Mock::given(path("/v2/namespaces/my-ns/query"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(path("/v2/namespaces/my-ns/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(query_body()))
            .with_priority(2)
            .mount(&server)
            .await;
        let client = retrying_client(&server.uri());
        buffer(&server.uri()).record(record(200));

        query(&client).await.unwrap();

        let sent = received(&server).await;
        assert_eq!(sent[0].as_ref().unwrap().len(), 1);
        assert_eq!(sent[1], None);
        let records = take_records(&buffer(&server.uri()));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["retries"], 1);
    }

    #[tokio::test]
    async fn follows_the_server_sample_rate() {
        let server = query_server(
            ResponseTemplate::new(200)
                .insert_header(SAMPLE_RATE_HEADER, "0")
                .set_body_json(query_body()),
        )
        .await;
        let client = client(&server.uri());

        query(&client).await.unwrap();
        query(&client).await.unwrap();

        assert!(received(&server).await.iter().all(Option::is_none));
        assert!(buffer(&server.uri()).take().is_none());
    }

    #[tokio::test]
    async fn records_routes_without_query_params() {
        let server = MockServer::start().await;
        Mock::given(path("/v1/namespaces"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "namespaces": [{ "id": "a" }] })),
            )
            .mount(&server)
            .await;
        Mock::given(path("/v2/namespaces/my-ns/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "billing": query_body()["billing"],
                "performance": query_body()["performance"],
            })))
            .mount(&server)
            .await;
        let client = client(&server.uri());
        let buffer = buffer(&server.uri());

        let namespaces: Vec<_> = client
            .namespaces()
            .prefix("a")
            .into_stream()
            .try_collect()
            .await
            .unwrap();
        assert_eq!(namespaces.len(), 1);
        assert_eq!(take_records(&buffer)[0]["route"], "GET /v1/namespaces");

        client
            .namespace("my-ns")
            .multi_query([crate::types::Query::new().top_k(1)])
            .send()
            .await
            .unwrap();
        let record = &take_records(&buffer)[0];
        assert_eq!(
            record["route"],
            "POST /v2/namespaces/{ns}/query?stainless_overload=multiQuery"
        );
        assert_eq!(record["server_total_us"], 7000);
    }
}
