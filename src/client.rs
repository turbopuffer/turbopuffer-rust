//! The API client and its configuration.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};

use crate::error::{Error, Result};
use crate::observer::RequestObserver;

/// The `production` environment's base URL.
const PRODUCTION_URL: &str = "https://{region}.turbopuffer.com";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_RETRIES: u32 = 4;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(250);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(8);

/// A client for the turbopuffer API.
///
/// Create one with [`Client::builder`] or [`Client::from_env`]. Clients hold a connection pool, so create one
/// and reuse it: cloning is cheap, and clones share the pool.
#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) headers: HeaderMap,
    pub(crate) stainless_headers: bool,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) initial_retry_delay: Duration,
    pub(crate) max_retry_delay: Duration,
    /// Whether to ask for gzipped responses and gzip large request bodies.
    pub(crate) compression: bool,
    pub(crate) observer: Option<Arc<dyn RequestObserver>>,
    /// Whether the client's connector traces connections for the observer.
    pub(crate) trace_connections: bool,
}

impl Client {
    /// Returns a builder for configuring a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Creates a client configured from environment variables:
    ///
    /// - `TURBOPUFFER_API_KEY`: API key used for authentication
    /// - `TURBOPUFFER_REGION`: The turbopuffer region to use
    /// - `TURBOPUFFER_BASE_URL`: overrides the base URL
    ///
    /// Equivalent to `Client::builder().build()`.
    pub fn from_env() -> Result<Self> {
        Self::builder().build()
    }

    /// The base URL requests are sent to.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// A client for tests that sends requests to `base_url` and retries without delay.
    #[cfg(test)]
    pub(crate) fn for_tests(base_url: &str) -> Self {
        Self::for_tests_with(base_url, false)
    }

    /// Like [`for_tests`](Self::for_tests), with compression on or off.
    #[cfg(test)]
    pub(crate) fn for_tests_with(base_url: &str, compression: bool) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                http: reqwest::Client::new(),
                base_url: base_url.to_owned(),
                headers: crate::transport::default_headers(true, compression),
                stainless_headers: true,
                timeout: Duration::from_secs(10),
                max_retries: 2,
                initial_retry_delay: Duration::from_millis(1),
                max_retry_delay: Duration::from_millis(5),
                compression,
                observer: None,
                trace_connections: false,
            }),
        }
    }

    /// Like [`for_tests`](Self::for_tests), with an observer that's traced like a built client's.
    #[cfg(test)]
    pub(crate) fn for_tests_observed(base_url: &str, observer: Arc<dyn RequestObserver>) -> Self {
        let mut client = Self::for_tests(base_url);
        let inner = Arc::get_mut(&mut client.inner).expect("the client isn't shared yet");
        inner.http = reqwest::Client::builder()
            .connector_layer(crate::observer::ConnectLayer)
            .build()
            .unwrap();
        inner.observer = Some(observer);
        inner.trace_connections = true;
        client
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.inner.base_url)
            .field("timeout", &self.inner.timeout)
            .field("max_retries", &self.inner.max_retries)
            .field("compression", &self.inner.compression)
            .finish_non_exhaustive()
    }
}

/// Configures and creates a [`Client`].
///
/// Settings that aren't set explicitly fall back to environment variables where noted.
#[derive(Clone, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    region: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    compression: Option<bool>,
    headers: Vec<(String, String)>,
    http_client: Option<reqwest::Client>,
    observer: Option<Arc<dyn RequestObserver>>,
}

impl ClientBuilder {
    /// API key used for authentication.
    ///
    /// Defaults to the `TURBOPUFFER_API_KEY` environment variable.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// The turbopuffer region to use.
    ///
    /// Defaults to the `TURBOPUFFER_REGION` environment variable.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Overrides the base URL for requests.
    ///
    /// Defaults to the `TURBOPUFFER_BASE_URL` environment variable, or `https://{region}.turbopuffer.com`.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// The timeout for each request attempt. Defaults to 1 minute.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// The maximum number of times to retry a failed request. Defaults to 4.
    ///
    /// Connection errors, timeouts, and responses with status 408, 409, 429, or 5xx are retried with exponential
    /// backoff, honoring any `Retry-After` header from the server.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Whether clients use gzip compression unless [`compression`](Self::compression) says otherwise. Like most HTTP
    /// clients, they do. APIs whose clients shouldn't by default can change this with custom code.
    const DEFAULT_COMPRESSION: bool = true;

    /// Whether to use gzip compression. Defaults to `true`.
    ///
    /// When it's on, requests ask for gzipped responses. When it's off, they ask for uncompressed responses
    /// (`accept-encoding: identity`).
    pub fn compression(mut self, compression: bool) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Adds a header to send with every request.
    pub fn default_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Uses a preconfigured [`reqwest::Client`] to send requests, e.g. to configure proxies or TLS.
    ///
    /// Timeouts set on the `reqwest::Client` apply in addition to the timeout set on this builder.
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// Observes the client's requests, such as to record their timings.
    #[allow(dead_code)]
    pub(crate) fn request_observer(mut self, observer: Arc<dyn RequestObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    /// Creates the client.
    ///
    /// Fails if a required setting is missing or invalid.
    pub fn build(self) -> Result<Client> {
        self.build_with_env(env_var)
    }

    /// Like [`build`](Self::build), but reads environment variables with `env`, so that tests don't depend on
    /// the process's environment.
    fn build_with_env(self, env: impl Fn(&str) -> Option<String>) -> Result<Client> {
        let Some(api_key) = self.api_key.or_else(|| env("TURBOPUFFER_API_KEY")) else {
            return Err(missing_setting("api_key", Some("TURBOPUFFER_API_KEY")));
        };
        let region_is_explicit = self.region.is_some();
        let region = self.region.or_else(|| env("TURBOPUFFER_REGION"));

        let base_url = self
            .base_url
            .or_else(|| env("TURBOPUFFER_BASE_URL"))
            .unwrap_or_else(|| PRODUCTION_URL.to_owned());
        let base_url = fill_server_variable(
            base_url,
            "region",
            region.as_deref(),
            region_is_explicit,
            "region",
            Some("TURBOPUFFER_REGION"),
        )?;
        reqwest::Url::parse(&base_url)
            .map_err(|err| Error::Config(format!("invalid base URL {base_url:?}: {err}")))?;

        let compression = self.compression.unwrap_or(Self::DEFAULT_COMPRESSION);
        let mut headers = crate::transport::default_headers(true, compression);
        headers.insert(
            "authorization",
            sensitive_header(format!("Bearer {}", api_key))?,
        );
        for (name, value) in self.headers {
            let name = reqwest::header::HeaderName::try_from(name.as_str())
                .map_err(|err| Error::Config(format!("invalid header name {name:?}: {err}")))?;
            headers.insert(name, header_value(value)?);
        }

        let observer = self.observer;
        // Connections can only be traced by the client's own connector.
        let trace_connections = observer.is_some() && self.http_client.is_none();
        let http = match self.http_client {
            Some(http) => http,
            None => {
                let mut builder = reqwest::Client::builder();
                if trace_connections {
                    builder = builder.connector_layer(crate::observer::ConnectLayer);
                }
                builder
                    .build()
                    .map_err(|err| Error::Config(format!("failed to create HTTP client: {err}")))?
            }
        };

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                base_url,
                headers,
                stainless_headers: true,
                timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
                max_retries: self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
                initial_retry_delay: INITIAL_RETRY_DELAY,
                max_retry_delay: MAX_RETRY_DELAY,
                compression,
                observer,
                trace_connections,
            }),
        })
    }
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("region", &self.region)
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("compression", &self.compression)
            .finish_non_exhaustive()
    }
}

/// Reads an environment variable, treating empty values as unset.
fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn header_value(value: String) -> Result<HeaderValue> {
    HeaderValue::try_from(value)
        .map_err(|err| Error::Config(format!("invalid header value: {err}")))
}

/// Fills in a `{placeholder}` in the base URL with the value of a setting.
///
/// Fails if the URL needs the setting but it isn't set, or if it was set explicitly but the URL doesn't use it,
/// since that's almost certainly a mistake.
#[allow(dead_code)]
fn fill_server_variable(
    url: String,
    placeholder: &str,
    value: Option<&str>,
    explicit: bool,
    setting: &str,
    env: Option<&str>,
) -> Result<String> {
    let pattern = format!("{{{placeholder}}}");
    match value {
        Some(value) if url.contains(&pattern) => Ok(url.replace(&pattern, value)),
        None if url.contains(&pattern) => Err(missing_setting(setting, env)),
        Some(_) if explicit => Err(Error::Config(format!(
            "{setting} is set, but the base URL {url:?} doesn't use it"
        ))),
        _ => Ok(url),
    }
}

/// The error for a required setting that wasn't provided.
fn missing_setting(name: &str, env: Option<&str>) -> Error {
    let hint = match env {
        Some(env) => {
            format!("set it with `ClientBuilder::{name}` or the `{env}` environment variable")
        }
        None => format!("set it with `ClientBuilder::{name}`"),
    };
    Error::Config(format!("missing {name}: {hint}"))
}

/// A header value that's redacted from debug output.
#[allow(dead_code)]
fn sensitive_header(value: String) -> Result<HeaderValue> {
    let mut value = header_value(value)?;
    value.set_sensitive(true);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use reqwest::Method;
    use serde_json::{Value, json};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::observer::{RequestInfo, RequestScope};
    use crate::transport::ApiRequest;

    /// A builder with every required setting.
    fn builder() -> ClientBuilder {
        Client::builder().api_key("test-api-key")
    }

    /// Builds a client with a fake set of environment variables.
    fn build(builder: ClientBuilder, env: &[(&str, &str)]) -> Result<Client> {
        let env: HashMap<&str, &str> = env.iter().copied().collect();
        builder.build_with_env(|name| env.get(name).map(|value| (*value).to_owned()))
    }

    fn config_error(result: Result<Client>) -> String {
        match result {
            Err(Error::Config(message)) => message,
            other => panic!("expected a configuration error, got {other:?}"),
        }
    }

    #[test]
    fn uses_the_default_environment() {
        let client = build(
            Client::builder()
                .api_key("test-api-key")
                .region("test-region"),
            &[],
        )
        .unwrap();
        assert_eq!(client.base_url(), "https://test-region.turbopuffer.com");
    }

    #[test]
    fn requires_server_variables_the_default_url_uses() {
        let message = config_error(build(builder(), &[]));
        assert!(message.contains("region"), "{message}");
    }

    #[test]
    fn fills_server_variables_into_overridden_urls() {
        let client = build(
            builder()
                .base_url("https://{region}.custom.example.com")
                .region("test-region"),
            &[],
        )
        .unwrap();
        assert_eq!(client.base_url(), "https://test-region.custom.example.com");
        config_error(build(
            builder().base_url("https://{region}.custom.example.com"),
            &[],
        ));
    }

    #[test]
    fn needs_no_server_variables_for_urls_without_placeholders() {
        let client = build(builder().base_url("https://api.example.com"), &[]).unwrap();
        assert_eq!(client.base_url(), "https://api.example.com");
    }

    #[test]
    fn rejects_explicit_server_variables_the_base_url_ignores() {
        let message = config_error(build(
            builder()
                .base_url("https://api.example.com")
                .region("test-region"),
            &[],
        ));
        assert!(message.contains("doesn't use it"), "{message}");
    }

    #[test]
    fn ignores_server_variables_from_the_environment_the_base_url_doesnt_use() {
        let client = build(
            builder().base_url("https://api.example.com"),
            &[("TURBOPUFFER_REGION", "env-value")],
        )
        .unwrap();
        assert_eq!(client.base_url(), "https://api.example.com");
    }

    #[test]
    fn reads_settings_from_the_environment() {
        let env = [
            ("TURBOPUFFER_API_KEY", "env-api-key"),
            ("TURBOPUFFER_REGION", "env-region"),
        ];
        let client = build(Client::builder(), &env).unwrap();
        assert_eq!(client.base_url(), "https://env-region.turbopuffer.com");
        assert_eq!(client.inner.headers["authorization"], "Bearer env-api-key");

        // Explicit settings take precedence.
        let client = build(
            Client::builder()
                .api_key("test-api-key")
                .region("test-region"),
            &env,
        )
        .unwrap();
        assert_eq!(client.base_url(), "https://test-region.turbopuffer.com");
        assert_eq!(client.inner.headers["authorization"], "Bearer test-api-key");
    }

    #[test]
    fn reads_the_base_url_from_the_environment() {
        let env = [("TURBOPUFFER_BASE_URL", "https://env.example.com")];
        let client = build(builder(), &env).unwrap();
        assert_eq!(client.base_url(), "https://env.example.com");
        let client = build(builder().base_url("https://explicit.example.com"), &env).unwrap();
        assert_eq!(client.base_url(), "https://explicit.example.com");
    }

    #[test]
    fn requires_api_key() {
        let message = config_error(build(
            Client::builder().base_url("https://api.example.com"),
            &[],
        ));
        assert!(message.contains("api_key"), "{message}");
        assert!(message.contains("TURBOPUFFER_API_KEY"), "{message}");
    }

    #[test]
    fn rejects_invalid_settings() {
        config_error(build(builder().base_url("not a url"), &[]));
        config_error(build(
            builder()
                .base_url("https://api.example.com")
                .default_header("bad header", "value"),
            &[],
        ));
    }

    #[test]
    fn redacts_secrets_from_debug_output() {
        let builder = builder().base_url("https://api.example.com");
        assert!(!format!("{builder:?}").contains("test-api-key"));
        let client = build(builder, &[]).unwrap();
        assert!(!format!("{client:?}").contains("test-api-key"));
        let headers = format!("{:?}", client.inner.headers);
        assert!(!headers.contains("test-api-key"), "{headers}");
    }

    #[tokio::test]
    async fn sends_configured_headers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/thing"))
            .and(header("authorization", "Bearer test-api-key"))
            .and(header("x-custom", "custom-value"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let builder = builder();
        let client = build(
            builder
                .base_url(server.uri())
                .default_header("x-custom", "custom-value"),
            &[],
        )
        .unwrap();
        client
            .execute::<Value>(ApiRequest::new(Method::GET, "/thing".to_owned()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn asks_for_compression_only_when_enabled() {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let default = if ClientBuilder::DEFAULT_COMPRESSION {
            "gzip"
        } else {
            "identity"
        };
        for (compression, expected) in [
            (None, default),
            (Some(false), "identity"),
            (Some(true), "gzip"),
        ] {
            let mut builder = builder().base_url(server.uri());
            if let Some(compression) = compression {
                builder = builder.compression(compression);
            }
            let client = build(builder, &[]).unwrap();
            client
                .execute::<Value>(ApiRequest::new(Method::GET, "/thing".to_owned()))
                .await
                .unwrap();
            let requests = server.received_requests().await.unwrap();
            let header = requests
                .last()
                .unwrap()
                .headers
                .get("accept-encoding")
                .and_then(|v| v.to_str().ok());
            assert_eq!(header, Some(expected), "compression: {compression:?}");
        }
    }

    #[tokio::test]
    async fn applies_retry_and_timeout_settings() {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = build(builder().base_url(server.uri()).max_retries(0), &[]).unwrap();
        let err = client
            .execute::<Value>(ApiRequest::new(Method::GET, "/thing".to_owned()))
            .await
            .unwrap_err();
        assert_eq!(
            err.status(),
            Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 1);

        let slow = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&slow)
            .await;
        let client = build(
            builder()
                .base_url(slow.uri())
                .max_retries(0)
                .timeout(Duration::from_millis(50)),
            &[],
        )
        .unwrap();
        let err = client
            .execute::<Value>(ApiRequest::new(Method::GET, "/thing".to_owned()))
            .await
            .unwrap_err();
        assert!(err.is_timeout(), "{err:?}");
    }

    #[tokio::test]
    async fn installs_request_observers() {
        struct Counter(AtomicUsize);
        impl RequestObserver for Counter {
            fn start(&self, _info: &RequestInfo<'_>) -> Option<Box<dyn RequestScope>> {
                self.0.fetch_add(1, Ordering::SeqCst);
                None
            }
        }

        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;
        let observer = Arc::new(Counter(AtomicUsize::new(0)));
        let client = build(
            builder()
                .base_url(server.uri())
                .request_observer(observer.clone()),
            &[],
        )
        .unwrap();
        assert!(client.inner.trace_connections);
        client
            .execute::<Value>(ApiRequest::new(Method::GET, "/thing".to_owned()))
            .await
            .unwrap();
        assert_eq!(observer.0.load(Ordering::SeqCst), 1);

        // Connections can't be traced through an HTTP client the client doesn't own.
        let builder = builder().base_url(server.uri());
        let client = build(
            builder
                .clone()
                .request_observer(observer)
                .http_client(reqwest::Client::new()),
            &[],
        )
        .unwrap();
        assert!(client.inner.observer.is_some() && !client.inner.trace_connections);
        let client = build(builder, &[]).unwrap();
        assert!(client.inner.observer.is_none() && !client.inner.trace_connections);
    }
}
