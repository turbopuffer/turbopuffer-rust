//! Methods called directly on the [`Client`].

use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::time::Duration;

use reqwest::Method;
use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;
use crate::pagination::{CursorPage, Paginator};
use crate::transport::{ApiRequest, RequestOptions};
use crate::types::NamespaceSummary;

impl Client {
    /// List namespaces.
    ///
    /// `GET /v1/namespaces`
    ///
    /// This endpoint is paginated: `send` returns the first page, while `into_stream` returns a stream
    /// of every item, fetching pages as needed.
    pub fn namespaces(&self) -> NamespacesRequest {
        NamespacesRequest::new(self.clone())
    }
}

/// A request to [`Client::namespaces`](crate::Client::namespaces).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct NamespacesRequest {
    client: Client,
    cursor: Option<String>,
    page_size: Option<u32>,
    prefix: Option<String>,
    options: RequestOptions,
}

impl NamespacesRequest {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            cursor: None,
            page_size: None,
            prefix: None,
            options: RequestOptions::default(),
        }
    }

    /// Retrieve the next page of results.
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Limit the number of results per page.
    pub fn page_size(mut self, page_size: u32) -> Self {
        self.page_size = Some(page_size);
        self
    }

    /// Retrieve only the namespaces that match the prefix.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Overrides the client's timeout for this request.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    /// Overrides the client's maximum number of retries for this request.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.options.max_retries = Some(max_retries);
        self
    }

    /// Adds a header to this request.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.headers.push((name.into(), value.into()));
        self
    }

    fn into_parts(self) -> Result<(Client, ApiRequest)> {
        let path = "/v1/namespaces".to_owned();
        let mut query = Vec::new();
        if let Some(value) = &self.cursor {
            query.push(("cursor".to_owned(), value.clone()));
        }
        if let Some(value) = &self.page_size {
            query.push(("page_size".to_owned(), value.to_string()));
        }
        if let Some(value) = &self.prefix {
            query.push(("prefix".to_owned(), value.clone()));
        }
        let request = ApiRequest::new(Method::GET, path)
            .query(query)
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request, returning the first page of results.
    pub async fn send(self) -> Result<CursorPage<NamespaceSummary>> {
        let (client, request) = self.into_parts()?;
        crate::pagination::fetch_cursor_page(client, request, "cursor", parse_namespaces_page).await
    }

    /// Returns a stream of every result, fetching pages as needed.
    ///
    /// Consume it with [`futures::TryStreamExt`](https://docs.rs/futures/latest/futures/stream/trait.TryStreamExt.html),
    /// e.g. `while let Some(item) = stream.try_next().await? { ... }`.
    pub fn into_stream(self) -> Paginator<NamespaceSummary> {
        match self.into_parts() {
            Ok((client, request)) => {
                crate::pagination::cursor_stream(client, request, "cursor", parse_namespaces_page)
            }
            Err(err) => crate::pagination::error_stream(err),
        }
    }

    /// Returns a stream of every page of results, fetching pages as needed. Each page has the details of the response
    /// it came in, like its request ID.
    pub fn into_page_stream(self) -> Paginator<CursorPage<NamespaceSummary>> {
        match self.into_parts() {
            Ok((client, request)) => crate::pagination::cursor_page_stream(
                client,
                request,
                "cursor",
                parse_namespaces_page,
            ),
            Err(err) => crate::pagination::error_stream(err),
        }
    }
}

impl IntoFuture for NamespacesRequest {
    type Output = Result<CursorPage<NamespaceSummary>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// Parses a page of [`NamespacesRequest`] results.
fn parse_namespaces_page(
    bytes: &[u8],
) -> serde_json::Result<(Vec<NamespaceSummary>, Option<String>)> {
    #[derive(Deserialize)]
    struct Page {
        #[serde(rename = "namespaces", default)]
        items: Vec<NamespaceSummary>,
        #[serde(rename = "next_cursor", default)]
        next_cursor: Option<String>,
    }
    let page: Page = serde_json::from_slice(bytes)?;
    Ok((page.items, page.next_cursor))
}
