//! Waiting for operations that the API runs in the background. This file is custom code: the generator doesn't
//! produce it.
//!
//! Every request asks the API to respond asynchronously with `prefer: respond-async`, which it may do for
//! long-running operations like `copy_from_namespace` and recall evaluation. It then responds with
//! `202 Accepted` and a `location` to poll, which the client does until the operation finishes, returning its
//! result as if the request had been synchronous. Polling stops with [`Error::OperationTimeout`] once the
//! request's timeout runs out, but the operation carries on in the background, and the error has its token (see
//! there for checking on it). Setting a `prefer` header on a request or client opts out.
//!
//! The transport hands every successful response to [`Client::await_operation`] as soon as it arrives, which
//! swaps a `202`'s body for the operation's result, so the rest of the transport reads it like any other
//! response. That keeps the custom code out of the generated code that reads response bodies, so that generator
//! changes there don't conflict with it.
//!
//! See <https://turbopuffer.com/docs/api-overview#asynchronous-requests>.

use std::time::Duration;

use bytes::Bytes;
use reqwest::header::{CONTENT_ENCODING, CONTENT_LENGTH, HeaderMap, LOCATION};
use reqwest::{Response, StatusCode, Url};
use serde::Deserialize;
use serde_json::Value;

use super::{ApiRequest, decode, retry_after, should_retry};
use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::observer::Observation;

pub(super) const PREFER: &str = "prefer";
pub(super) const RESPOND_ASYNC: &str = "respond-async";

/// How long to wait between polls.
const POLL_INTERVAL: Duration = if cfg!(test) {
    Duration::from_millis(10)
} else {
    Duration::from_secs(1)
};

/// The longest a single poll can take.
const POLL_TIMEOUT: Duration = Duration::from_secs(60);

impl Client {
    /// Returns a successful response as is, or if the API chose to run the request in the background, waits for
    /// the operation to finish and returns a response with its result for a body. The response keeps the status
    /// and headers of the one that started the operation, like its `X-Request-Id`.
    pub(super) async fn await_operation(
        &self,
        request: &ApiRequest,
        response: Response,
        observation: &mut Observation,
    ) -> Result<Response> {
        if !responded_async(&response) {
            return Ok(response);
        }

        let (status, mut headers) = (response.status(), response.headers().clone());
        let body = self
            .operation_result(request, response, observation)
            .await?;
        // These described the `202`'s own body.
        headers.remove(CONTENT_LENGTH);
        headers.remove(CONTENT_ENCODING);
        let mut result = http::Response::new(body);
        *result.status_mut() = status;
        *result.headers_mut() = headers;
        Ok(Response::from(result))
    }

    /// Waits for the operation that a `202 Accepted` response started to finish, returning its result.
    async fn operation_result(
        &self,
        request: &ApiRequest,
        response: Response,
        observation: &mut Observation,
    ) -> Result<Bytes> {
        let location = location(&response);
        let body = self.read_accepted_body(response, observation).await?;
        let location = location.map_err(|message| Error::Decode {
            source: serde::de::Error::custom(message),
            body: String::from_utf8_lossy(&body).into_owned(),
        })?;

        let timeout = request.options.timeout.unwrap_or(self.inner.timeout);
        let max_retries = request
            .options
            .max_retries
            .unwrap_or(self.inner.max_retries);
        let mut headers = self.inner.headers.clone();
        headers.remove(PREFER);

        let wait = async {
            loop {
                let body = self.poll(&location, &headers, max_retries).await?;
                match decode::<Operation>(&body)? {
                    Operation::Running {} => {}
                    Operation::Finished {
                        result: OperationResult::Success(result),
                    } => return Ok(serde_json::to_vec(&result).map_err(Error::Json)?.into()),
                    Operation::Finished {
                        result: OperationResult::Error(error),
                    } => return Err(error.into_api_error(&body)?.into()),
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        };
        tokio::time::timeout(timeout, wait)
            .await
            .unwrap_or_else(|_| {
                Err(Error::OperationTimeout {
                    token: operation_token(&location),
                    location: location.to_string(),
                })
            })
    }

    /// Reads the body of the response that started an operation. Its timing is the request's body timing: the
    /// response returned in its place is read afterwards, but the observer keeps the first time it's told about.
    async fn read_accepted_body(
        &self,
        response: Response,
        observation: &mut Observation,
    ) -> Result<Bytes> {
        match response.bytes().await {
            Ok(bytes) => {
                observation.body_end();
                Ok(bytes)
            }
            Err(err) => Err(observation.body_failed(err)),
        }
    }

    /// Fetches an operation's status, retrying failures like any other request.
    async fn poll(&self, location: &Url, headers: &HeaderMap, max_retries: u32) -> Result<Bytes> {
        let mut attempt = 0;
        loop {
            let result = self
                .inner
                .http
                .get(location.clone())
                .timeout(POLL_TIMEOUT)
                .headers(headers.clone())
                .send()
                .await;
            let retry_after = match result {
                Ok(response) if response.status().is_success() => {
                    return response.bytes().await.map_err(Error::from_reqwest);
                }
                Ok(response) => {
                    if attempt >= max_retries
                        || !should_retry(response.status(), response.headers())
                    {
                        return Err(ApiError::from_response(response).await.into());
                    }
                    retry_after(response.headers())
                }
                Err(err) => {
                    if attempt >= max_retries
                        || !(err.is_timeout() || err.is_connect() || err.is_request())
                    {
                        return Err(Error::from_reqwest(err));
                    }
                    None
                }
            };
            let delay = retry_after.unwrap_or_else(|| self.backoff(attempt));
            attempt += 1;
            tokio::time::sleep(delay).await;
        }
    }
}

/// Whether the API chose to run the request in the background.
fn responded_async(response: &Response) -> bool {
    response.status() == StatusCode::ACCEPTED
        && response
            .headers()
            .get("preference-applied")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.trim().eq_ignore_ascii_case(RESPOND_ASYNC))
}

/// The operation's token: the last segment of the URL to poll for it, like `tpuf-job-1` in
/// `/v1/namespaces/ns/operations/tpuf-job-1`. It's percent-decoded, since methods taking a token, like
/// `Namespace::poll_copy_from`, encode it themselves.
fn operation_token(location: &Url) -> String {
    let segment = location
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or_default();
    percent_decode(segment)
}

/// Decodes `%XX` escapes in a URL path segment, leaving malformed ones as they are.
fn percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                i += 3;
            }
            None => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The URL to poll for the operation, which must have the same origin as the request so that the API key isn't
/// sent anywhere else.
fn location(response: &Response) -> Result<Url, String> {
    let raw = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .unwrap_or_default();
    if raw.is_empty() {
        return Err("asynchronous response is missing a `location` header".to_owned());
    }
    let location = response
        .url()
        .join(raw)
        .map_err(|_| format!("malformed `location` header: {raw:?}"))?;
    if location.origin() != response.url().origin() {
        return Err(format!(
            "`location` header has a different origin than the request: {raw:?}"
        ));
    }
    Ok(location)
}

/// An operation's status, from polling its location.
#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Operation {
    Running {},
    Finished { result: OperationResult },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OperationResult {
    /// The response body the request would have had if it had been synchronous.
    Success(Value),
    Error(OperationError),
}

/// The error response the request would have had if it had been synchronous.
#[derive(Deserialize)]
struct OperationError {
    status_code: u16,
    #[serde(default)]
    detail: Value,
}

impl OperationError {
    fn into_api_error(self, body: &[u8]) -> Result<ApiError> {
        let status = StatusCode::from_u16(self.status_code).map_err(|err| Error::Decode {
            source: serde::de::Error::custom(err),
            body: String::from_utf8_lossy(body).into_owned(),
        })?;
        let message = self
            .detail
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                status
                    .canonical_reason()
                    .unwrap_or("unknown error")
                    .to_owned()
            });
        Ok(ApiError {
            status,
            message,
            body: (!self.detail.is_null()).then_some(self.detail),
            headers: HeaderMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::Method;
    use serde_json::json;
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::error::ApiErrorKind;
    use crate::transport::RequestOptions;

    const OPERATION: &str = "/v1/namespaces/ns/operations/tpuf-job-1";

    fn copy_request() -> ApiRequest {
        ApiRequest::new(Method::POST, "/v2/namespaces/ns".to_owned())
            .json(&json!({ "copy_from_namespace": "source" }))
            .unwrap()
    }

    fn accepted(location: &str) -> ResponseTemplate {
        ResponseTemplate::new(202)
            .insert_header("preference-applied", "respond-async")
            .insert_header("location", location)
            .set_body_json(json!({ "token": "tpuf-job-1" }))
    }

    async fn mount_operation(server: &MockServer, statuses: &[Value]) {
        for (i, status) in statuses.iter().enumerate() {
            let mock = Mock::given(method("GET"))
                .and(path(OPERATION))
                .respond_with(ResponseTemplate::new(200).set_body_json(status));
            // Earlier statuses take priority until they've been used up.
            let mock = if i + 1 < statuses.len() {
                mock.up_to_n_times(1).with_priority(1 + i as u8)
            } else {
                mock.with_priority(u8::MAX)
            };
            mock.mount(server).await;
        }
    }

    #[tokio::test]
    async fn keeps_the_status_and_headers_of_responses_that_start_operations() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/namespaces/ns"))
            .respond_with(accepted(OPERATION).insert_header("x-request-id", "req_1"))
            .mount(&server)
            .await;
        mount_operation(
            &server,
            &[json!({
                "status": "finished",
                "result": { "success": { "status": "OK", "message": "namespace copied" } },
            })],
        )
        .await;

        let client = Client::for_tests(&server.uri());
        let accepted = client
            .inner
            .http
            .post(format!("{}/v2/namespaces/ns", server.uri()))
            .send()
            .await
            .unwrap();
        let response = client
            .await_operation(&copy_request(), accepted, &mut Observation::inert())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(response.headers()["x-request-id"], "req_1");
        assert!(response.headers().get(CONTENT_LENGTH).is_none());
        let result: Value = serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
        assert_eq!(
            result,
            json!({ "status": "OK", "message": "namespace copied" })
        );
    }

    #[tokio::test]
    async fn waits_for_operations_to_finish() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/namespaces/ns"))
            .and(header("prefer", "respond-async"))
            .respond_with(accepted(OPERATION))
            .expect(1)
            .mount(&server)
            .await;
        mount_operation(
            &server,
            &[
                json!({ "status": "running", "start_time": "2026-10-16T20:19:26Z" }),
                json!({ "status": "running" }),
                json!({
                    "status": "finished",
                    "result": { "success": { "status": "OK", "message": "namespace copied" } },
                }),
            ],
        )
        .await;

        let client = Client::for_tests(&server.uri());
        let result: Value = client.execute(copy_request()).await.unwrap();
        assert_eq!(
            result,
            json!({ "status": "OK", "message": "namespace copied" })
        );

        let polls = server.received_requests().await.unwrap();
        let polls: Vec<_> = polls.iter().filter(|r| r.method == "GET").collect();
        assert_eq!(polls.len(), 3);
        // Polls carry the client's headers, but don't ask to respond asynchronously.
        assert!(polls[0].headers.contains_key("x-stainless-lang"));
        assert!(!polls[0].headers.contains_key("prefer"));
    }

    #[tokio::test]
    async fn returns_failed_operations_as_api_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(accepted(OPERATION))
            .mount(&server)
            .await;
        mount_operation(
            &server,
            &[json!({
                "status": "finished",
                "result": {
                    "error": {
                        "status_code": 400,
                        "detail": { "status": "error", "error": "destination namespace already exists" },
                    },
                },
            })],
        )
        .await;

        let client = Client::for_tests(&server.uri());
        let err = client.execute::<Value>(copy_request()).await.unwrap_err();
        let err = err.as_api_error().unwrap();
        assert_eq!(err.kind(), ApiErrorKind::BadRequest);
        assert_eq!(err.message, "destination namespace already exists");
        assert_eq!(
            err.body,
            Some(json!({ "status": "error", "error": "destination namespace already exists" }))
        );
    }

    #[tokio::test]
    async fn retries_failed_polls() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(accepted(OPERATION))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .with_priority(1)
            .mount(&server)
            .await;
        mount_operation(
            &server,
            &[json!({ "status": "finished", "result": { "success": { "status": "OK" } } })],
        )
        .await;

        let client = Client::for_tests(&server.uri());
        let result: Value = client.execute(copy_request()).await.unwrap();
        assert_eq!(result, json!({ "status": "OK" }));
    }

    #[tokio::test]
    async fn times_out_while_operations_run() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(accepted(OPERATION))
            .mount(&server)
            .await;
        mount_operation(&server, &[json!({ "status": "running" })]).await;

        let client = Client::for_tests(&server.uri());
        let request = copy_request().options(RequestOptions {
            timeout: Some(Duration::from_millis(100)),
            ..Default::default()
        });
        let err = client.execute::<Value>(request).await.unwrap_err();
        assert!(err.is_timeout(), "{err:?}");
        // The error has what's needed to check on the operation, which carries on.
        let Error::OperationTimeout { token, location } = err else {
            panic!("expected an operation timeout, got {err:?}");
        };
        assert_eq!(token, "tpuf-job-1");
        assert_eq!(location, format!("{}{OPERATION}", server.uri()));
    }

    #[tokio::test]
    async fn stops_retrying_polls_at_the_deadline() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(accepted(OPERATION))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "10"))
            .mount(&server)
            .await;

        let client = Client::for_tests(&server.uri());
        let request = copy_request().options(RequestOptions {
            timeout: Some(Duration::from_millis(100)),
            ..Default::default()
        });
        // Waiting out the `retry-after` would run well past the request's timeout.
        let err = tokio::time::timeout(Duration::from_secs(1), client.execute::<Value>(request))
            .await
            .expect("polled past the deadline")
            .unwrap_err();
        assert!(matches!(err, Error::OperationTimeout { .. }), "{err:?}");
    }

    #[test]
    fn percent_decodes_operation_tokens() {
        let token = |location: &str| operation_token(&Url::parse(location).unwrap());
        assert_eq!(
            token("https://api.test/v1/namespaces/ns/operations/tpuf-job-1"),
            "tpuf-job-1"
        );
        assert_eq!(
            token("https://api.test/v1/namespaces/ns/operations/a%2Fb%20c"),
            "a/b c"
        );
        // Malformed escapes are left as they are.
        assert_eq!(token("https://api.test/operations/50%25%zz%"), "50%%zz%");
    }

    #[tokio::test]
    async fn returns_responses_the_api_didnt_run_in_the_background() {
        let server = MockServer::start().await;
        // A `202` without `preference-applied` is an ordinary response.
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(202)
                    .insert_header("location", OPERATION)
                    .set_body_json(json!({ "status": "OK" })),
            )
            .mount(&server)
            .await;

        let client = Client::for_tests(&server.uri());
        let result: Value = client.execute(copy_request()).await.unwrap();
        assert_eq!(result, json!({ "status": "OK" }));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn lets_requests_opt_out() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("prefer", "respond-sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "OK" })))
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::for_tests(&server.uri());
        let request = copy_request().header("prefer", "respond-sync");
        let result: Value = client.execute(request).await.unwrap();
        assert_eq!(result, json!({ "status": "OK" }));
    }

    #[tokio::test]
    async fn rejects_locations_on_other_origins() {
        let server = MockServer::start().await;
        let elsewhere = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(accepted(&format!("{}{OPERATION}", elsewhere.uri())))
            .mount(&server)
            .await;
        Mock::given(header_exists("authorization"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&elsewhere)
            .await;

        let client = Client::for_tests(&server.uri());
        let err = client.execute::<Value>(copy_request()).await.unwrap_err();
        assert!(
            matches!(&err, Error::Decode { source, body }
                if source.to_string().contains("different origin") && body.contains("tpuf-job-1")),
            "{err:?}"
        );
        assert!(elsewhere.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_responses_without_locations() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(202).insert_header("preference-applied", "respond-async"),
            )
            .mount(&server)
            .await;

        let client = Client::for_tests(&server.uri());
        let err = client.execute::<Value>(copy_request()).await.unwrap_err();
        assert!(matches!(err, Error::Decode { .. }), "{err:?}");
    }
}
