//! Error types.

use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde_json::Value;

/// A `Result` alias where the error defaults to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by the client.
///
/// Errors caused by another error, like [`Error::Connection`], return it from [`std::error::Error::source`] rather
/// than repeating it in their message, so error reporters like `anyhow`'s show each cause once.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API responded with a non-success status code.
    #[error(transparent)]
    Api(Box<ApiError>),

    /// The request timed out (after exhausting any retries).
    #[error("request timed out")]
    Timeout,

    /// The API ran the request in the background, like a large `copy_from_namespace`, and it was still running
    /// when the request's timeout ran out. The operation carries on.
    ///
    /// For a copy, check on it with [`Namespace::poll_copy_from`](crate::Namespace::poll_copy_from) and `token`,
    /// rather than sending the request again, which would start another copy. Other operations, like recall
    /// evaluations, can't be checked on with this SDK, and `poll_copy_from` can't decode their results: send those
    /// again with a longer timeout.
    #[error("request timed out while operation {token} was still running")]
    OperationTimeout {
        /// The operation's token.
        token: String,
        /// The URL of the operation's status.
        location: String,
    },

    /// The request couldn't be sent or its response couldn't be read (after exhausting any retries).
    #[error("connection error")]
    Connection(#[source] reqwest::Error),

    /// The response body couldn't be decoded into the expected type.
    #[error("failed to decode response body")]
    Decode {
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
        /// The raw response body.
        body: String,
    },

    /// The request body couldn't be encoded as JSON.
    #[error("failed to encode request body")]
    Encode(#[source] serde_json::Error),

    /// A value couldn't be converted to or from JSON, e.g. by a type's `from_serializable` or `deserialize_as`.
    ///
    /// Converts from [`serde_json::Error`], so those conversions can use `?` in functions that return
    /// [`Result`].
    #[error("failed to convert to or from JSON")]
    Json(#[from] serde_json::Error),

    /// The client is misconfigured, e.g. because it's missing an API key.
    #[error("invalid client configuration: {0}")]
    Config(String),
}

impl Error {
    /// The HTTP status code of the response, if the API returned one.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Api(err) => Some(err.status),
            _ => None,
        }
    }

    /// Returns the [`ApiError`] if the API responded with a non-success status code.
    pub fn as_api_error(&self) -> Option<&ApiError> {
        match self {
            Self::Api(err) => Some(err),
            _ => None,
        }
    }

    /// Whether this is a timeout, including an [`Error::OperationTimeout`].
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout | Self::OperationTimeout { .. })
    }

    /// Whether the request might succeed if it's tried again: timeouts, connection errors, and the API errors the
    /// client retries itself (see [`ApiError::is_retryable`]). The client has already retried these up to its
    /// maximum number of retries before returning them.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Connection(err) => crate::transport::is_retryable_reqwest_error(err),
            Self::Api(err) => err.is_retryable(),
            _ => false,
        }
    }

    pub(crate) fn from_reqwest(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else {
            Self::Connection(err)
        }
    }
}

impl From<ApiError> for Error {
    fn from(err: ApiError) -> Self {
        Self::Api(Box::new(err))
    }
}

/// An error response from the API.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{status}: {message}")]
#[non_exhaustive]
pub struct ApiError {
    /// The HTTP status code.
    pub status: StatusCode,
    /// A message describing the error, taken from the response body if possible.
    pub message: String,
    /// The response body, if it was valid JSON.
    pub body: Option<Value>,
    /// The response headers.
    pub headers: HeaderMap,
}

impl ApiError {
    /// Creates an error with the given status and message, e.g. to test how an application handles API errors.
    /// Its body is `None` and it has no headers, but both can be set afterwards.
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            body: None,
            headers: HeaderMap::new(),
        }
    }

    /// Whether the client retries requests that fail with this error: `408`, `409`, `429`, and `5xx` statuses,
    /// unless the API says otherwise with an `x-should-retry` header.
    pub fn is_retryable(&self) -> bool {
        crate::transport::should_retry(self.status, &self.headers)
    }

    /// Classifies the error by its status code.
    pub fn kind(&self) -> ApiErrorKind {
        match self.status.as_u16() {
            400 => ApiErrorKind::BadRequest,
            401 => ApiErrorKind::Authentication,
            403 => ApiErrorKind::PermissionDenied,
            404 => ApiErrorKind::NotFound,
            409 => ApiErrorKind::Conflict,
            422 => ApiErrorKind::UnprocessableEntity,
            429 => ApiErrorKind::RateLimit,
            500..=599 => ApiErrorKind::InternalServer,
            _ => ApiErrorKind::Other,
        }
    }

    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await.unwrap_or_default();
        let body = serde_json::from_str::<Value>(&text).ok();
        let message = body
            .as_ref()
            .and_then(|body| ERROR_MESSAGE_PROPERTY.and_then(|property| body.get(property)))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                if text.is_empty() {
                    status
                        .canonical_reason()
                        .unwrap_or("unknown error")
                        .to_owned()
                } else {
                    text
                }
            });
        Self {
            status,
            message,
            body,
            headers,
        }
    }
}

/// The category of an [`ApiError`], based on its status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ApiErrorKind {
    /// `400 Bad Request`.
    BadRequest,
    /// `401 Unauthorized`.
    Authentication,
    /// `403 Forbidden`.
    PermissionDenied,
    /// `404 Not Found`.
    NotFound,
    /// `409 Conflict`.
    Conflict,
    /// `422 Unprocessable Entity`.
    UnprocessableEntity,
    /// `429 Too Many Requests`.
    RateLimit,
    /// A `5xx` status code.
    InternalServer,
    /// Any other non-success status code.
    Other,
}

/// The property of an error response body holding a human-readable message.
const ERROR_MESSAGE_PROPERTY: Option<&str> = Some("error");

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn extracts_messages_from_error_responses() {
        let mut body = json!({ "status": "error" });
        if let Some(property) = ERROR_MESSAGE_PROPERTY {
            body[property] = json!("something went wrong");
        }
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(400).set_body_json(body.clone()))
            .mount(&server)
            .await;

        let response = reqwest::get(format!("{}/thing", server.uri()))
            .await
            .unwrap();
        let err = ApiError::from_response(response).await;
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.kind(), ApiErrorKind::BadRequest);
        assert_eq!(err.body, Some(body));
        if ERROR_MESSAGE_PROPERTY.is_some() {
            assert_eq!(err.message, "something went wrong");
            assert_eq!(err.to_string(), "400 Bad Request: something went wrong");
        }
    }

    #[test]
    fn converts_json_errors() {
        fn parse(text: &str) -> Result<Value> {
            Ok(serde_json::from_str(text)?)
        }
        let err = parse("{").unwrap_err();
        assert!(matches!(err, Error::Json(_)));
        // The cause is the error's source rather than part of its message.
        assert_eq!(err.to_string(), "failed to convert to or from JSON");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn classifies_retryable_errors() {
        assert!(Error::Timeout.is_retryable());
        assert!(!Error::Config("missing API key".to_owned()).is_retryable());
        assert!(
            Error::from(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "slow down")).is_retryable()
        );
        assert!(Error::from(ApiError::new(StatusCode::BAD_GATEWAY, "bad gateway")).is_retryable());
        assert!(!Error::from(ApiError::new(StatusCode::NOT_FOUND, "not found")).is_retryable());

        // The API can override the default with `x-should-retry`.
        let mut err = ApiError::new(StatusCode::BAD_GATEWAY, "bad gateway");
        err.headers
            .insert("x-should-retry", "false".parse().unwrap());
        assert!(!err.is_retryable());
    }

    #[test]
    fn builds_api_errors() {
        let err = ApiError::new(StatusCode::NOT_FOUND, "namespace not found");
        assert_eq!(err.kind(), ApiErrorKind::NotFound);
        assert_eq!(err.to_string(), "404 Not Found: namespace not found");
        assert_eq!(err.body, None);
    }

    #[tokio::test]
    async fn falls_back_to_the_status_for_empty_bodies() {
        let server = MockServer::start().await;
        Mock::given(path("/thing"))
            .respond_with(ResponseTemplate::new(502))
            .mount(&server)
            .await;

        let response = reqwest::get(format!("{}/thing", server.uri()))
            .await
            .unwrap();
        let err = ApiError::from_response(response).await;
        assert_eq!(err.message, "Bad Gateway");
        assert_eq!(err.kind(), ApiErrorKind::InternalServer);
    }
}
