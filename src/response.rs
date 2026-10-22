//! Responses along with the details of the HTTP responses they came in, like their request IDs.

use reqwest::StatusCode;
use reqwest::header::HeaderMap;

/// The header that carries a request's ID, which the API echoes back in its response.
const REQUEST_ID_HEADER: &str = "x-request-id";

/// A response's data, along with the status and headers of the HTTP response it came in.
///
/// Returned by request builders' `send_with_response` methods, an alternative to `send` for when the response's
/// details are needed, like its [`request_id`](Self::request_id).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiResponse<T> {
    /// The response's data.
    pub data: T,
    /// The response's HTTP status code.
    pub status: StatusCode,
    /// The response's headers.
    pub headers: HeaderMap,
}

impl<T> ApiResponse<T> {
    pub(crate) fn new(data: T, status: StatusCode, headers: HeaderMap) -> Self {
        Self {
            data,
            status,
            headers,
        }
    }

    /// The request's ID, from the `X-Request-Id` header the API echoes back, if the request was sent with one.
    ///
    /// A request can be sent with an ID of its own with its builder's `header` method. Retries reuse the request's
    /// headers, so they share its ID.
    pub fn request_id(&self) -> Option<&str> {
        request_id(&self.headers)
    }

    /// Consumes the response, returning its data.
    pub fn into_data(self) -> T {
        self.data
    }
}

/// The request ID in a response's headers.
pub(crate) fn request_id(headers: &HeaderMap) -> Option<&str> {
    headers.get(REQUEST_ID_HEADER)?.to_str().ok()
}
