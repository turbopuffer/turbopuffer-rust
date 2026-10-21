//! Request execution: building HTTP requests, retrying failures, and decoding responses.

use std::any::Any;
use std::borrow::Cow;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::time::{Duration, Instant};

use bytes::Bytes;
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue,
    USER_AGENT,
};
use reqwest::{Method, Response, StatusCode, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::observer::{Observation, RequestInfo};

/// The longest `Retry-After` the client will honor. Longer values fall back to exponential backoff.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30 * 60);

/// Whether clients using compression also gzip request bodies (larger than `MIN_GZIP_SIZE`). They don't, since many
/// APIs don't accept gzipped bodies. APIs that do can change this with custom code.
const GZIP_REQUEST_BODIES: bool = false;

/// Request bodies larger than this are gzipped, if they're gzipped at all. Smaller ones aren't worth the CPU.
const MIN_GZIP_SIZE: usize = 1024;

/// Settings that can be overridden for a single request.
#[derive(Debug, Clone, Default)]
pub(crate) struct RequestOptions {
    pub(crate) timeout: Option<Duration>,
    pub(crate) max_retries: Option<u32>,
    pub(crate) headers: Vec<(String, String)>,
}

/// A fully described API request that can be sent (and re-sent when retrying).
#[derive(Debug, Clone)]
pub(crate) struct ApiRequest {
    method: Method,
    path: String,
    /// The spec path's `stainless_overload`, which lets request observers tell overloads of a path apart. It isn't
    /// sent.
    overload: Option<&'static str>,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Option<Bytes>,
    /// When the body was serialized, for request observers.
    serialized: Option<(Instant, Instant)>,
    options: RequestOptions,
}

impl ApiRequest {
    pub(crate) fn new(method: Method, path: String) -> Self {
        Self {
            method,
            path,
            overload: None,
            query: Vec::new(),
            headers: Vec::new(),
            body: None,
            serialized: None,
            options: RequestOptions::default(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn json<T: Serialize + ?Sized>(mut self, body: &T) -> Result<Self> {
        let start = Instant::now();
        self.body = Some(serde_json::to_vec(body).map_err(Error::Encode)?.into());
        self.serialized = Some((start, Instant::now()));
        Ok(self)
    }

    #[allow(dead_code)]
    pub(crate) fn overload(mut self, overload: &'static str) -> Self {
        self.overload = Some(overload);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn query(mut self, query: Vec<(String, String)>) -> Self {
        self.query = query;
        self
    }

    #[allow(dead_code)]
    pub(crate) fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub(crate) fn options(mut self, options: RequestOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets a query parameter, replacing any existing values for it.
    #[allow(dead_code)]
    pub(crate) fn set_query_param(&mut self, name: &str, value: String) {
        self.query.retain(|(key, _)| key != name);
        self.query.push((name.to_owned(), value));
    }

    /// Forgets when the body was serialized, for a copy of the request that's sent later.
    #[allow(dead_code)]
    pub(crate) fn forget_serialization(&mut self) {
        self.serialized = None;
    }

    /// The URL request observers see: the path with its overload, without query params.
    fn observed_url(&self) -> Cow<'_, str> {
        match self.overload {
            Some(overload) => {
                let separator = if self.path.contains('?') { '&' } else { '?' };
                Cow::Owned(format!(
                    "{}{separator}stainless_overload={overload}",
                    self.path
                ))
            }
            None => Cow::Borrowed(&self.path),
        }
    }
}

impl Client {
    /// Sends a request and decodes its JSON response body.
    pub(crate) async fn execute<T: DeserializeOwned + 'static>(
        &self,
        request: ApiRequest,
    ) -> Result<T> {
        let mut observation = self.observe(&request);
        let result = self
            .send_and_parse(&request, &mut observation, decode::<T>)
            .await;
        match &result {
            Ok(value) => observation.finish(Some(value as &dyn Any), None),
            Err(err) => observation.finish(None, Some(err)),
        }
        result
    }

    /// Sends a request and parses its response body with `parse`.
    #[allow(dead_code)]
    pub(crate) async fn execute_with<T>(
        &self,
        request: ApiRequest,
        parse: impl FnOnce(&[u8]) -> Result<T>,
    ) -> Result<T> {
        let mut observation = self.observe(&request);
        let result = self.send_and_parse(&request, &mut observation, parse).await;
        observation.finish(None, result.as_ref().err());
        result
    }

    /// Sends a request and returns its raw response body.
    #[allow(dead_code)]
    pub(crate) async fn execute_bytes(&self, request: ApiRequest) -> Result<Bytes> {
        let mut observation = self.observe(&request);
        let result = self.send_and_read(&request, &mut observation).await;
        match &result {
            Ok(bytes) => observation.finish(Some(bytes as &dyn Any), None),
            Err(err) => observation.finish(None, Some(err)),
        }
        result
    }

    /// Sends a request, ignoring any response body.
    #[allow(dead_code)]
    pub(crate) async fn execute_empty(&self, request: ApiRequest) -> Result<()> {
        let mut observation = self.observe(&request);
        let result = self
            .send_with_retries(&request, &mut observation)
            .await
            .map(drop);
        match &result {
            Ok(()) => observation.finish(Some(&() as &dyn Any), None),
            Err(err) => observation.finish(None, Some(err)),
        }
        result
    }

    /// Starts observing a request, if the client has an observer.
    pub(crate) fn observe(&self, request: &ApiRequest) -> Observation {
        let Some(observer) = &self.inner.observer else {
            return Observation::inert();
        };
        let url = request.observed_url();
        let info = RequestInfo {
            method: &request.method,
            url: &url,
        };
        Observation::start(
            observer.as_ref(),
            self.inner.trace_connections,
            &info,
            request.serialized,
        )
    }

    async fn send_and_parse<T>(
        &self,
        request: &ApiRequest,
        observation: &mut Observation,
        parse: impl FnOnce(&[u8]) -> Result<T>,
    ) -> Result<T> {
        let bytes = self.send_and_read(request, observation).await?;
        observation.deserialize_begin();
        parse(&bytes)
    }

    async fn send_and_read(
        &self,
        request: &ApiRequest,
        observation: &mut Observation,
    ) -> Result<Bytes> {
        let response = self.send_with_retries(request, observation).await?;
        match response.bytes().await {
            Ok(bytes) => {
                observation.body_end();
                Ok(bytes)
            }
            Err(err) => Err(observation.body_failed(err)),
        }
    }

    async fn send_with_retries(
        &self,
        request: &ApiRequest,
        observation: &mut Observation,
    ) -> Result<Response> {
        let inner = &self.inner;
        let max_retries = request.options.max_retries.unwrap_or(inner.max_retries);
        let timeout = request.options.timeout.unwrap_or(inner.timeout);
        let url = self.url(request);
        // Compressed once, rather than for each attempt.
        let body = self.encode_body(request);

        let mut attempt = 0;
        loop {
            observation.start_attempt(attempt);
            let result = match self
                .request_builder(request, body.as_ref(), url.clone(), attempt, timeout)
                .build()
            {
                Ok(mut http_request) => {
                    match observation.attempt_begin(&mut http_request, request.body.as_ref()) {
                        Some(trace) => {
                            crate::observer::traced(trace, inner.http.execute(http_request)).await
                        }
                        None => inner.http.execute(http_request).await,
                    }
                }
                Err(err) => Err(err),
            };
            let retry_after = match result {
                Ok(response) => {
                    observation.attempt_response(&response);
                    if response.status().is_success() {
                        return Ok(response);
                    }
                    if attempt >= max_retries
                        || !should_retry(response.status(), response.headers())
                    {
                        let err = ApiError::from_response(response).await;
                        observation.body_end();
                        return Err(err.into());
                    }
                    retry_after(response.headers())
                }
                Err(err) => {
                    let retryable = is_retryable_reqwest_error(&err);
                    let err = observation.attempt_failed(err);
                    if attempt >= max_retries || !retryable {
                        return Err(err);
                    }
                    None
                }
            };
            let delay = retry_after.unwrap_or_else(|| self.backoff(attempt));
            attempt += 1;
            tokio::time::sleep(delay).await;
        }
    }

    fn url(&self, request: &ApiRequest) -> Url {
        let mut url = self.inner.base.clone();
        let path = format!("{}{}", url.path().trim_end_matches('/'), request.path);
        url.set_path(&path);
        if !request.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&request.query);
        }
        url
    }

    /// A request's body as it's sent, and whether it's gzipped: it is when the API accepts gzipped bodies,
    /// compression is on, and the body is large enough to be worth it.
    fn encode_body(&self, request: &ApiRequest) -> Option<(Bytes, bool)> {
        let body = request.body.as_ref()?;
        if !GZIP_REQUEST_BODIES || !self.inner.compression || body.len() <= MIN_GZIP_SIZE {
            return Some((body.clone(), false));
        }
        // Fast compression, like the other SDKs: most of the size reduction for a fraction of the CPU.
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::with_capacity(body.len() / 4),
            flate2::Compression::fast(),
        );
        encoder
            .write_all(body)
            .expect("gzipping into memory can't fail");
        Some((
            encoder
                .finish()
                .expect("gzipping into memory can't fail")
                .into(),
            true,
        ))
    }

    fn request_builder(
        &self,
        request: &ApiRequest,
        body: Option<&(Bytes, bool)>,
        url: Url,
        attempt: u32,
        timeout: Duration,
    ) -> reqwest::RequestBuilder {
        let inner = &self.inner;
        let mut builder = inner
            .http
            .request(request.method.clone(), url)
            .timeout(timeout)
            .headers(inner.headers.clone());
        if inner.stainless_headers {
            builder = builder
                .header("x-stainless-retry-count", attempt.to_string())
                .header("x-stainless-timeout", timeout.as_secs().to_string());
        }
        // Request headers replace the client's defaults rather than adding to them, e.g. to change \`accept\`.
        let mut overrides = HeaderMap::new();
        for (name, value) in request.headers.iter().chain(&request.options.headers) {
            match (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                (Ok(name), Ok(value)) => {
                    overrides.append(name, value);
                }
                // Invalid headers are reported as an error when the request is sent.
                _ => builder = builder.header(name.as_str(), value.as_str()),
            }
        }
        builder = builder.headers(overrides);
        if let Some((body, gzipped)) = body {
            builder = builder.header(CONTENT_TYPE, "application/json");
            if *gzipped {
                builder = builder.header(CONTENT_ENCODING, "gzip");
            }
            builder = builder.body(body.clone());
        }
        builder
    }

    /// Exponential backoff with jitter.
    fn backoff(&self, attempt: u32) -> Duration {
        let inner = &self.inner;
        let delay = inner
            .initial_retry_delay
            .saturating_mul(2u32.saturating_pow(attempt))
            .min(inner.max_retry_delay);
        // Subtract up to 25% jitter so concurrent clients don't retry in lockstep.
        let jitter = (random_fraction() * 0.25).min(1.0);
        delay.mul_f64(1.0 - jitter)
    }
}

/// Headers sent with every request.
pub(crate) fn default_headers(stainless_headers: bool, compression: bool) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    // Without compression, ask for uncompressed responses: reqwest asks for gzip otherwise.
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static(if compression { "gzip" } else { "identity" }),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(concat!("Turbopuffer/Rust ", env!("CARGO_PKG_VERSION"))),
    );
    if stainless_headers {
        headers.insert("x-stainless-lang", HeaderValue::from_static("rust"));
        headers.insert(
            "x-stainless-package-version",
            HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        );
        headers.insert("x-stainless-os", HeaderValue::from_static(os()));
        headers.insert("x-stainless-arch", HeaderValue::from_static(arch()));
        headers.insert("x-stainless-runtime", HeaderValue::from_static("rust"));
    }
    headers
}

pub(crate) fn should_retry(status: StatusCode, headers: &HeaderMap) -> bool {
    match headers.get("x-should-retry").and_then(|v| v.to_str().ok()) {
        Some("true") => return true,
        Some("false") => return false,
        _ => {}
    }
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::CONFLICT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

/// Whether a request that failed without a response is worth retrying.
pub(crate) fn is_retryable_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

/// Parses `retry-after-ms` or `retry-after` (seconds or an HTTP date) from a response.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
    };

    let delay = if let Some(ms) = header("retry-after-ms").and_then(|v| v.parse::<f64>().ok()) {
        Duration::try_from_secs_f64(ms / 1000.0).ok()?
    } else {
        let value = header("retry-after")?;
        match value.parse::<f64>() {
            Ok(secs) => Duration::try_from_secs_f64(secs).ok()?,
            Err(_) => {
                let date = jiff::fmt::rfc2822::DateTimeParser::new()
                    .parse_timestamp(value)
                    .ok()?;
                jiff::Timestamp::now()
                    .duration_until(date)
                    .try_into()
                    .ok()?
            }
        }
    };
    (delay <= MAX_RETRY_AFTER).then_some(delay)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|source| Error::Decode {
        source,
        body: String::from_utf8_lossy(bytes).into_owned(),
    })
}

/// A random number in `[0, 1)`, good enough for jitter without pulling in an RNG crate.
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

/// Percent-encodes a value for use as a single path segment.
#[allow(dead_code)]
pub(crate) fn encode_path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "MacOS",
        "ios" => "iOS",
        "linux" => "Linux",
        "windows" => "Windows",
        "android" => "Android",
        "freebsd" => "FreeBSD",
        "openbsd" => "OpenBSD",
        other => other,
    }
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x32",
        "x86_64" => "x64",
        "arm" => "arm",
        "aarch64" => "arm64",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_path_segments() {
        assert_eq!(encode_path_segment("simple-name_1.0~"), "simple-name_1.0~");
        assert_eq!(encode_path_segment("a/b c"), "a%2Fb%20c");
        assert_eq!(encode_path_segment("ü"), "%C3%BC");
    }

    #[test]
    fn appends_request_paths_to_the_base_url() {
        let url = |base: &str| {
            let request = ApiRequest::new(Method::GET, "/things/a%20b".to_owned())
                .query(vec![("limit".to_owned(), "10".to_owned())]);
            Client::for_tests(base).url(&request).to_string()
        };
        assert_eq!(
            url("https://api.example.com"),
            "https://api.example.com/things/a%20b?limit=10"
        );
        assert_eq!(
            url("https://api.example.com/"),
            "https://api.example.com/things/a%20b?limit=10"
        );
        assert_eq!(
            url("https://example.com/api/"),
            "https://example.com/api/things/a%20b?limit=10"
        );
    }

    #[test]
    fn parses_retry_after() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("2"));
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(2)));

        headers.insert("retry-after-ms", HeaderValue::from_static("150"));
        assert_eq!(retry_after(&headers), Some(Duration::from_millis(150)));

        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("999999"));
        assert_eq!(retry_after(&headers), None);

        // HTTP dates are relative to now, and ones in the past don't apply.
        let date = jiff::Timestamp::now() + jiff::SignedDuration::from_secs(30);
        let date = jiff::fmt::rfc2822::DateTimePrinter::new()
            .timestamp_to_rfc9110_string(&date)
            .unwrap();
        headers.insert("retry-after", HeaderValue::from_str(&date).unwrap());
        let delay = retry_after(&headers).unwrap();
        assert!(
            delay > Duration::from_secs(28) && delay <= Duration::from_secs(30),
            "{delay:?}"
        );

        headers.insert(
            "retry-after",
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        assert_eq!(retry_after(&headers), None);
    }

    #[test]
    fn retries_on_expected_statuses() {
        let headers = HeaderMap::new();
        assert!(should_retry(StatusCode::TOO_MANY_REQUESTS, &headers));
        assert!(should_retry(StatusCode::BAD_GATEWAY, &headers));
        assert!(should_retry(StatusCode::CONFLICT, &headers));
        assert!(!should_retry(StatusCode::BAD_REQUEST, &headers));
        assert!(!should_retry(StatusCode::NOT_FOUND, &headers));

        let mut headers = HeaderMap::new();
        headers.insert("x-should-retry", HeaderValue::from_static("true"));
        assert!(should_retry(StatusCode::BAD_REQUEST, &headers));
    }

    #[test]
    fn random_fraction_is_in_range() {
        for _ in 0..100 {
            let value = random_fraction();
            assert!((0.0..1.0).contains(&value));
        }
    }

    mod requests {
        use std::time::Duration;

        use serde_json::{Value, json};
        use wiremock::matchers::{body_json, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use super::super::ApiRequest;
        use crate::client::Client;
        use crate::error::{ApiErrorKind, Error};

        async fn server_with(status: u16, body: Value) -> MockServer {
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(status).set_body_json(body))
                .mount(&server)
                .await;
            server
        }

        #[tokio::test]
        async fn sends_requests_and_decodes_responses() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/things/a%20b"))
                .and(query_param("limit", "10"))
                .and(body_json(json!({ "name": "thing" })))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
                .expect(1)
                .mount(&server)
                .await;

            let client = Client::for_tests(&server.uri());
            let request = ApiRequest::new(
                reqwest::Method::POST,
                format!("/things/{}", super::encode_path_segment("a b")),
            )
            .query(vec![("limit".to_owned(), "10".to_owned())])
            .json(&json!({ "name": "thing" }))
            .unwrap();
            let response: Value = client.execute(request).await.unwrap();
            assert_eq!(response, json!({ "ok": true }));

            let received = &server.received_requests().await.unwrap()[0];
            let header = |name: &str| {
                received
                    .headers
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned)
            };
            assert_eq!(header("content-type").as_deref(), Some("application/json"));
            assert_eq!(header("accept").as_deref(), Some("application/json"));
            assert_eq!(header("x-stainless-lang").as_deref(), Some("rust"));
            assert_eq!(header("x-stainless-retry-count").as_deref(), Some("0"));
            assert!(header("user-agent").is_some_and(|ua| ua.contains("/Rust ")));
        }

        #[tokio::test]
        async fn doesnt_compress_when_off() {
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
                .mount(&server)
                .await;

            let client = Client::for_tests(&server.uri());
            let large = json!({ "text": "a".repeat(2 * crate::transport::MIN_GZIP_SIZE) });
            let request = ApiRequest::new(reqwest::Method::POST, "/thing".to_owned())
                .json(&large)
                .unwrap();
            let _: Value = client.execute(request).await.unwrap();

            let received = &server.received_requests().await.unwrap()[0];
            let header = |name: &str| received.headers.get(name).and_then(|v| v.to_str().ok());
            assert_eq!(header("accept-encoding"), Some("identity"));
            assert_eq!(header("content-encoding"), None);
            assert_eq!(
                serde_json::from_slice::<Value>(&received.body).unwrap(),
                large
            );
        }

        #[tokio::test]
        async fn compresses_when_enabled() {
            // `{"ok":true}`, gzipped.
            const GZIPPED: &[u8] = &[
                0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xab, 0x56, 0xca, 0xcf,
                0x56, 0xb2, 0x2a, 0x29, 0x2a, 0x4d, 0xad, 0x05, 0x00, 0x90, 0x5f, 0xd4, 0xa7, 0x0b,
                0x00, 0x00, 0x00,
            ];
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-encoding", "gzip")
                        .set_body_raw(GZIPPED, "application/json"),
                )
                .mount(&server)
                .await;

            let client = Client::for_tests_with(&server.uri(), true);
            let large = json!({ "text": "a".repeat(2 * crate::transport::MIN_GZIP_SIZE) });
            let small = json!({ "text": "a" });
            for body in [&large, &small] {
                let request = ApiRequest::new(reqwest::Method::POST, "/thing".to_owned())
                    .json(body)
                    .unwrap();
                let response: Value = client.execute(request).await.unwrap();
                assert_eq!(response, json!({ "ok": true }));
            }

            let received = server.received_requests().await.unwrap();
            let header =
                |i: usize, name: &str| received[i].headers.get(name).and_then(|v| v.to_str().ok());
            assert_eq!(header(0, "accept-encoding"), Some("gzip"));
            // Large bodies are gzipped if the API accepts them, and small ones aren't worth it.
            if crate::transport::GZIP_REQUEST_BODIES {
                assert_eq!(header(0, "content-encoding"), Some("gzip"));
                let mut decompressed = Vec::new();
                std::io::Read::read_to_end(
                    &mut flate2::read::GzDecoder::new(&received[0].body[..]),
                    &mut decompressed,
                )
                .unwrap();
                assert_eq!(
                    serde_json::from_slice::<Value>(&decompressed).unwrap(),
                    large
                );
            } else {
                assert_eq!(header(0, "content-encoding"), None);
                assert_eq!(
                    serde_json::from_slice::<Value>(&received[0].body).unwrap(),
                    large
                );
            }
            assert_eq!(header(1, "content-encoding"), None);
            assert_eq!(
                serde_json::from_slice::<Value>(&received[1].body).unwrap(),
                small
            );
        }

        #[tokio::test]
        async fn request_headers_replace_defaults() {
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
                .mount(&server)
                .await;

            let client = Client::for_tests(&server.uri());
            let request = ApiRequest::new(reqwest::Method::GET, "/thing".to_owned())
                .header("accept", "text/plain");
            client.execute::<Value>(request).await.unwrap();

            let received = &server.received_requests().await.unwrap()[0];
            let accept: Vec<_> = received.headers.get_all("accept").iter().collect();
            assert_eq!(accept, ["text/plain"]);
        }

        #[tokio::test]
        async fn retries_server_errors() {
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(500))
                .up_to_n_times(2)
                .with_priority(1)
                .mount(&server)
                .await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
                .with_priority(2)
                .mount(&server)
                .await;

            let client = Client::for_tests(&server.uri());
            let response: Value = client
                .execute(ApiRequest::new(reqwest::Method::GET, "/thing".to_owned()))
                .await
                .unwrap();
            assert_eq!(response, json!({ "ok": true }));

            let counts: Vec<String> = server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .map(|r| {
                    r.headers["x-stainless-retry-count"]
                        .to_str()
                        .unwrap()
                        .to_owned()
                })
                .collect();
            assert_eq!(counts, ["0", "1", "2"]);
        }

        #[tokio::test]
        async fn gives_up_after_max_retries() {
            let server = server_with(503, json!({})).await;
            let client = Client::for_tests(&server.uri());
            let options = super::super::RequestOptions {
                max_retries: Some(1),
                ..Default::default()
            };
            let err = client
                .execute::<Value>(
                    ApiRequest::new(reqwest::Method::GET, "/thing".to_owned()).options(options),
                )
                .await
                .unwrap_err();
            assert_eq!(err.status(), Some(reqwest::StatusCode::SERVICE_UNAVAILABLE));
            assert_eq!(server.received_requests().await.unwrap().len(), 2);
        }

        #[tokio::test]
        async fn does_not_retry_client_errors() {
            let server = server_with(404, json!({ "status": "error" })).await;
            let client = Client::for_tests(&server.uri());
            let err = client
                .execute::<Value>(ApiRequest::new(reqwest::Method::GET, "/thing".to_owned()))
                .await
                .unwrap_err();
            let Error::Api(api) = &err else {
                panic!("expected an API error, got {err:?}");
            };
            assert_eq!(api.kind(), ApiErrorKind::NotFound);
            assert_eq!(api.body, Some(json!({ "status": "error" })));
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }

        #[tokio::test]
        async fn honors_retry_after() {
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(429).insert_header("retry-after-ms", "50"))
                .up_to_n_times(1)
                .with_priority(1)
                .mount(&server)
                .await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
                .with_priority(2)
                .mount(&server)
                .await;

            let client = Client::for_tests(&server.uri());
            let start = std::time::Instant::now();
            client
                .execute::<Value>(ApiRequest::new(reqwest::Method::GET, "/thing".to_owned()))
                .await
                .unwrap();
            assert!(start.elapsed() >= Duration::from_millis(50));
        }

        #[tokio::test]
        async fn times_out() {
            let server = MockServer::start().await;
            Mock::given(path("/thing"))
                .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
                .mount(&server)
                .await;

            let client = Client::for_tests(&server.uri());
            let options = super::super::RequestOptions {
                timeout: Some(Duration::from_millis(50)),
                max_retries: Some(0),
                ..Default::default()
            };
            let err = client
                .execute::<Value>(
                    ApiRequest::new(reqwest::Method::GET, "/thing".to_owned()).options(options),
                )
                .await
                .unwrap_err();
            assert!(err.is_timeout(), "expected a timeout, got {err:?}");
        }

        #[tokio::test]
        async fn reports_undecodable_responses() {
            let server = server_with(200, json!({ "unexpected": true })).await;
            let client = Client::for_tests(&server.uri());
            let err = client
                .execute::<Vec<String>>(ApiRequest::new(reqwest::Method::GET, "/thing".to_owned()))
                .await
                .unwrap_err();
            let Error::Decode { body, .. } = &err else {
                panic!("expected a decode error, got {err:?}");
            };
            assert_eq!(body, r#"{"unexpected":true}"#);
        }
    }
}
