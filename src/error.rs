//! Error types.

use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde_json::Value;

/// A `Result` alias where the error defaults to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by the client.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API responded with a non-success status code.
    #[error(transparent)]
    Api(Box<ApiError>),

    /// The request timed out (after exhausting any retries).
    #[error("request timed out")]
    Timeout,

    /// The request couldn't be sent or its response couldn't be read (after exhausting any retries).
    #[error("connection error: {0}")]
    Connection(#[source] reqwest::Error),

    /// The response body couldn't be decoded into the expected type.
    #[error("failed to decode response body: {source}")]
    Decode {
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
        /// The raw response body.
        body: String,
    },

    /// The request body couldn't be encoded as JSON.
    #[error("failed to encode request body: {0}")]
    Encode(#[source] serde_json::Error),

    /// A value couldn't be converted to or from JSON, e.g. by a type's `from_serializable` or `deserialize_as`.
    ///
    /// Converts from [`serde_json::Error`], so those conversions can use `?` in functions that return
    /// [`Result`].
    #[error("failed to convert to or from JSON: {0}")]
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

    /// Whether this is a timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
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
        assert!(
            err.to_string()
                .starts_with("failed to convert to or from JSON: ")
        );
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
