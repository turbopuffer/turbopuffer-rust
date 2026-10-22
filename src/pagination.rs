//! Support for paginated list endpoints.
//!
//! List methods return a [`CursorPage`] for the first page of results, which can fetch subsequent pages with
//! [`CursorPage::next_page`]. To iterate over every item across all pages, use the request builder's
//! `into_stream` method (or [`CursorPage::into_stream`]), which fetches pages as needed. To iterate over the pages
//! themselves, along with the details of the responses they came in, like their request IDs, use `into_page_stream`
//! (or [`CursorPage::into_page_stream`]).

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_util::stream::{self, BoxStream, StreamExt};
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::transport::ApiRequest;

/// Parses a page's response body into its items and the cursor for the next page.
pub(crate) type ParsePage<T> = fn(&[u8]) -> serde_json::Result<(Vec<T>, Option<String>)>;

/// A single page of results from a cursor-paginated endpoint.
pub struct CursorPage<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
    /// The status of the response the page came in.
    status: StatusCode,
    /// The headers of the response the page came in.
    headers: HeaderMap,
    fetcher: PageFetcher<T>,
}

/// Everything needed to fetch the page after this one.
struct PageFetcher<T> {
    client: Client,
    request: ApiRequest,
    cursor_param: &'static str,
    parse: ParsePage<T>,
}

impl<T> Clone for PageFetcher<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            request: self.request.clone(),
            cursor_param: self.cursor_param,
            parse: self.parse,
        }
    }
}

impl<T> CursorPage<T> {
    /// The items on this page.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Consumes the page, returning its items.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    /// The cursor identifying the next page, if any.
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }

    /// The HTTP status code of the response this page came in.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// The headers of the response this page came in.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The ID of the request that fetched this page, from the `X-Request-Id` header the API echoes back. Each page is
    /// fetched by its own request. See [`ApiResponse::request_id`](crate::ApiResponse::request_id).
    pub fn request_id(&self) -> Option<&str> {
        crate::response::request_id(&self.headers)
    }

    /// Whether there's another page after this one.
    pub fn has_next_page(&self) -> bool {
        !self.items.is_empty()
            && self
                .next_cursor
                .as_deref()
                .is_some_and(|cursor| !cursor.is_empty())
    }
}

impl<T: DeserializeOwned> CursorPage<T> {
    /// Fetches the next page, or returns `None` if this is the last one.
    pub async fn next_page(&self) -> Result<Option<CursorPage<T>>> {
        match self.next_fetcher() {
            Some(fetcher) => fetcher.fetch().await.map(Some),
            None => Ok(None),
        }
    }
}

impl<T: DeserializeOwned + Send + 'static> CursorPage<T> {
    /// Returns a stream of the items on this page followed by the items on all subsequent pages, which are
    /// fetched as the stream is consumed.
    pub fn into_stream(self) -> Paginator<T> {
        let stream = stream::try_unfold(self.into_state(), |mut state| async move {
            loop {
                if let Some(item) = state.items.next() {
                    return Ok(Some((item, state)));
                }
                match state.next.take() {
                    Some(fetcher) => state = fetcher.fetch().await?.into_state(),
                    None => return Ok(None),
                }
            }
        });
        Paginator {
            inner: stream.boxed(),
        }
    }

    /// Returns a stream of this page followed by all subsequent pages, which are fetched as the stream is consumed.
    ///
    /// Unlike [`into_stream`](Self::into_stream), it yields whole pages, which have the details of the responses
    /// they came in, like their [`request_id`](Self::request_id).
    pub fn into_page_stream(self) -> Paginator<CursorPage<T>> {
        let next = self.next_fetcher();
        let rest = stream::try_unfold(next, |fetcher| async move {
            let Some(fetcher) = fetcher else {
                return Ok(None);
            };
            let page = fetcher.fetch().await?;
            let next = page.next_fetcher();
            Ok(Some((page, next)))
        });
        Paginator {
            inner: stream::once(async move { Ok(self) }).chain(rest).boxed(),
        }
    }
}

impl<T> CursorPage<T> {
    /// A fetcher for the page after this one, if there is one.
    fn next_fetcher(&self) -> Option<PageFetcher<T>> {
        if !self.has_next_page() {
            return None;
        }
        let mut fetcher = self.fetcher.clone();
        let cursor = self.next_cursor.clone().unwrap_or_default();
        fetcher
            .request
            .set_query_param(fetcher.cursor_param, cursor);
        // The next page's request reuses the body serialized for the first one.
        fetcher.request.forget_serialization();
        Some(fetcher)
    }

    fn into_state(self) -> PageState<T> {
        let next = self.next_fetcher();
        PageState {
            items: self.items.into_iter(),
            next,
        }
    }
}

impl<T> PageFetcher<T> {
    async fn fetch(self) -> Result<CursorPage<T>> {
        let parse = self.parse;
        let response = self
            .client
            .execute_with(self.request.clone(), |bytes| {
                parse(bytes).map_err(|source| Error::Decode {
                    source,
                    body: String::from_utf8_lossy(bytes).into_owned(),
                })
            })
            .await?;
        let (items, next_cursor) = response.data;
        Ok(CursorPage {
            items,
            next_cursor,
            status: response.status,
            headers: response.headers,
            fetcher: self,
        })
    }
}

impl<T: fmt::Debug> fmt::Debug for CursorPage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CursorPage")
            .field("items", &self.items)
            .field("next_cursor", &self.next_cursor)
            .finish_non_exhaustive()
    }
}

impl<T> IntoIterator for CursorPage<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    /// Iterates over the items on this page only.
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Iteration state for [`CursorPage::into_stream`]: the remaining items on the current page, and how to fetch
/// the next one.
struct PageState<T> {
    items: std::vec::IntoIter<T>,
    next: Option<PageFetcher<T>>,
}

/// A stream over every item of a paginated list, or over its pages, fetching pages as needed.
///
/// Consume it with the combinators in [`futures::TryStreamExt`](https://docs.rs/futures/latest/futures/stream/trait.TryStreamExt.html),
/// e.g. `try_next` or `try_collect`.
#[must_use = "streams do nothing unless polled"]
pub struct Paginator<T> {
    inner: BoxStream<'static, Result<T>>,
}

impl<T> Stream for Paginator<T> {
    type Item = Result<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> fmt::Debug for Paginator<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paginator").finish_non_exhaustive()
    }
}

/// Fetches the first page of a cursor-paginated endpoint.
pub(crate) async fn fetch_cursor_page<T>(
    client: Client,
    request: ApiRequest,
    cursor_param: &'static str,
    parse: ParsePage<T>,
) -> Result<CursorPage<T>> {
    PageFetcher {
        client,
        request,
        cursor_param,
        parse,
    }
    .fetch()
    .await
}

/// Returns a stream over every item of a cursor-paginated endpoint, starting from the first page.
pub(crate) fn cursor_stream<T: DeserializeOwned + Send + 'static>(
    client: Client,
    request: ApiRequest,
    cursor_param: &'static str,
    parse: ParsePage<T>,
) -> Paginator<T> {
    let stream = stream::once(fetch_cursor_page(client, request, cursor_param, parse))
        .map(|page| match page {
            Ok(page) => page.into_stream(),
            Err(err) => error_stream(err),
        })
        .flatten();
    Paginator {
        inner: stream.boxed(),
    }
}

/// Returns a stream over every page of a cursor-paginated endpoint, starting from the first.
pub(crate) fn cursor_page_stream<T: DeserializeOwned + Send + 'static>(
    client: Client,
    request: ApiRequest,
    cursor_param: &'static str,
    parse: ParsePage<T>,
) -> Paginator<CursorPage<T>> {
    let stream = stream::once(fetch_cursor_page(client, request, cursor_param, parse))
        .map(|page| match page {
            Ok(page) => page.into_page_stream(),
            Err(err) => error_stream(err),
        })
        .flatten();
    Paginator {
        inner: stream.boxed(),
    }
}

/// A stream that yields a single error, for requests that fail before they're sent.
pub(crate) fn error_stream<T: Send + 'static>(err: Error) -> Paginator<T> {
    Paginator {
        inner: stream::once(async move { Err(err) }).boxed(),
    }
}

#[cfg(test)]
mod tests {
    use futures_util::TryStreamExt;
    use serde_json::json;
    use wiremock::matchers::{path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn parse(bytes: &[u8]) -> serde_json::Result<(Vec<u32>, Option<String>)> {
        #[derive(serde::Deserialize)]
        struct Page {
            items: Vec<u32>,
            next_cursor: Option<String>,
        }
        let page: Page = serde_json::from_slice(bytes)?;
        Ok((page.items, page.next_cursor))
    }

    async fn paginated_server() -> MockServer {
        let server = MockServer::start().await;
        let pages = [
            (None, json!({ "items": [1, 2], "next_cursor": "a" })),
            (Some("a"), json!({ "items": [3, 4], "next_cursor": "b" })),
            (Some("b"), json!({ "items": [5], "next_cursor": null })),
        ];
        for (cursor, body) in pages {
            let mock = Mock::given(path("/items")).and(query_param("filter", "x"));
            let mock = match cursor {
                Some(cursor) => mock.and(query_param("cursor", cursor)),
                None => mock.and(query_param_is_missing("cursor")),
            };
            mock.respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&server)
                .await;
        }
        server
    }

    fn request() -> ApiRequest {
        ApiRequest::new(reqwest::Method::GET, "/items".to_owned())
            .query(vec![("filter".to_owned(), "x".to_owned())])
    }

    #[tokio::test]
    async fn fetches_pages_one_at_a_time() {
        let server = paginated_server().await;
        let client = Client::for_tests(&server.uri());

        let page = fetch_cursor_page(client, request(), "cursor", parse)
            .await
            .unwrap();
        assert_eq!(page.items(), [1, 2]);
        assert!(page.has_next_page());

        let page = page.next_page().await.unwrap().unwrap();
        assert_eq!(page.items(), [3, 4]);

        let page = page.next_page().await.unwrap().unwrap();
        assert_eq!(page.items(), [5]);
        assert!(!page.has_next_page());
        assert!(page.next_page().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn pages_have_their_responses_headers() {
        let server = MockServer::start().await;
        for (cursor, body, request_id) in [
            (None, json!({ "items": [1], "next_cursor": "a" }), "req_1"),
            (
                Some("a"),
                json!({ "items": [2], "next_cursor": null }),
                "req_2",
            ),
        ] {
            let mock = Mock::given(path("/items"));
            let mock = match cursor {
                Some(cursor) => mock.and(query_param("cursor", cursor)),
                None => mock.and(query_param_is_missing("cursor")),
            };
            let template = ResponseTemplate::new(200)
                .set_body_json(body)
                .insert_header("x-request-id", request_id);
            mock.respond_with(template).mount(&server).await;
        }
        let client = Client::for_tests(&server.uri());

        let page = fetch_cursor_page(client.clone(), request(), "cursor", parse)
            .await
            .unwrap();
        assert_eq!(page.status(), reqwest::StatusCode::OK);
        assert_eq!(page.request_id(), Some("req_1"));
        assert_eq!(page.headers()["content-type"], "application/json");
        let page = page.next_page().await.unwrap().unwrap();
        assert_eq!(page.request_id(), Some("req_2"));

        let streamed: Vec<Option<String>> = cursor_page_stream(client, request(), "cursor", parse)
            .map_ok(|page| page.request_id().map(str::to_owned))
            .try_collect()
            .await
            .unwrap();
        assert_eq!(
            streamed,
            [Some("req_1".to_owned()), Some("req_2".to_owned())]
        );
    }

    #[tokio::test]
    async fn streams_every_item() {
        let server = paginated_server().await;
        let client = Client::for_tests(&server.uri());

        let items: Vec<u32> = cursor_stream(client, request(), "cursor", parse)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(items, [1, 2, 3, 4, 5]);
        assert_eq!(server.received_requests().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn streams_every_page() {
        let server = paginated_server().await;
        let client = Client::for_tests(&server.uri());

        let pages: Vec<Vec<u32>> = cursor_page_stream(client.clone(), request(), "cursor", parse)
            .map_ok(CursorPage::into_items)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(pages, [vec![1, 2], vec![3, 4], vec![5]]);
        assert_eq!(server.received_requests().await.unwrap().len(), 3);

        // Streaming from a page that's already been fetched starts with it, without fetching it again.
        let page = fetch_cursor_page(client, request(), "cursor", parse)
            .await
            .unwrap();
        let pages: Vec<CursorPage<u32>> = page.into_page_stream().try_collect().await.unwrap();
        assert_eq!(pages.len(), 3);
        assert_eq!(server.received_requests().await.unwrap().len(), 6);
    }

    #[tokio::test]
    async fn streams_page_errors() {
        let server = MockServer::start().await;
        Mock::given(path("/items"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;
        let client = Client::for_tests(&server.uri());

        let result: Result<Vec<CursorPage<u32>>> =
            cursor_page_stream(client, request(), "cursor", parse)
                .try_collect()
                .await;
        assert_eq!(
            result.unwrap_err().status(),
            Some(reqwest::StatusCode::BAD_REQUEST)
        );
    }

    #[tokio::test]
    async fn streams_errors() {
        let server = MockServer::start().await;
        Mock::given(path("/items"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;
        let client = Client::for_tests(&server.uri());

        let result: Result<Vec<u32>> = cursor_stream(client, request(), "cursor", parse)
            .try_collect()
            .await;
        assert_eq!(
            result.unwrap_err().status(),
            Some(reqwest::StatusCode::BAD_REQUEST)
        );
    }
}
