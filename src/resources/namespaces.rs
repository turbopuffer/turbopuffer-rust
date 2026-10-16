//! Operations on a single namespace, through the [`Namespace`] handle returned by
//! [`Client::namespace`].

use std::collections::HashMap;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::time::Duration;

use reqwest::Method;
use serde::Serialize;

use crate::client::Client;
use crate::error::Result;
use crate::transport::{ApiRequest, RequestOptions, encode_path_segment};
use crate::types::{
    AggregateBy, AttributeSchema, BranchFromNamespaceParams, Columns, CopyFromNamespaceOperation,
    CopyFromNamespaceParams, DistanceMetric, Encryption, Expr, Filter, GroupBy, Id,
    IncludeAttributes, MultiQueryLimit, MultiQueryResult, NamespaceDeleteAllResponse,
    NamespaceExplainQueryResponse, NamespaceHintCacheWarmResponse, NamespaceMetadata,
    NamespaceMetadataPatchPinning, NamespaceRecallResponse, NamespaceSchemaResponse,
    NamespaceStartCopyFromResponse, NamespaceUpdateSchemaResponse, PatchByFilter, Query,
    QueryConfigConsistency, QueryLimit, QueryResult, RankBy, RerankBy, RerankModelParams, Row,
    ShardingConfig, VectorEncoding, WriteResult,
};

/// A handle to a single namespace, returned by [`Client::namespace`].
///
/// Creating a handle doesn't make any requests. It's cheap to create and clone.
#[derive(Debug, Clone)]
pub struct Namespace {
    client: Client,
    namespace: String,
}

impl Client {
    /// Returns a [`Namespace`] handle for calling methods that operate on a single namespace.
    ///
    /// - `namespace`: The name of the namespace.
    ///
    /// The handle is cheap to create, and creating one doesn't make any requests.
    pub fn namespace(&self, namespace: impl Into<String>) -> Namespace {
        Namespace {
            client: self.clone(),
            namespace: namespace.into(),
        }
    }
}

impl Namespace {
    /// The namespace this handle refers to.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

impl Namespace {
    /// Creates an instant, copy-on-write clone of a namespace.
    ///
    /// `POST /v2/namespaces/{namespace}`
    pub fn branch_from(&self, source_namespace: impl Into<String>) -> BranchFromRequest {
        BranchFromRequest::new(
            self.client.clone(),
            self.namespace.clone(),
            source_namespace.into(),
        )
    }

    /// Copy all documents from another namespace into this one.
    ///
    /// `POST /v2/namespaces/{namespace}`
    pub fn copy_from(&self, source_namespace: impl Into<String>) -> CopyFromRequest {
        CopyFromRequest::new(
            self.client.clone(),
            self.namespace.clone(),
            source_namespace.into(),
        )
    }

    /// Delete namespace.
    ///
    /// `DELETE /v2/namespaces/{namespace}`
    pub fn delete_all(&self) -> DeleteAllRequest {
        DeleteAllRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Explain a query plan.
    ///
    /// `POST /v2/namespaces/{namespace}/explain_query`
    pub fn explain_query(&self) -> ExplainQueryRequest {
        ExplainQueryRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Signal turbopuffer to prepare for low-latency requests.
    ///
    /// `GET /v1/namespaces/{namespace}/hint_cache_warm`
    pub fn hint_cache_warm(&self) -> HintCacheWarmRequest {
        HintCacheWarmRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Get metadata about a namespace.
    ///
    /// `GET /v2/namespaces/{namespace}/metadata`
    pub fn metadata(&self) -> MetadataRequest {
        MetadataRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Issue multiple concurrent queries filter or search documents.
    ///
    /// `POST /v2/namespaces/{namespace}/query`
    pub fn multi_query(
        &self,
        queries: impl IntoIterator<Item = impl Into<Query>>,
    ) -> MultiQueryRequest {
        MultiQueryRequest::new(
            self.client.clone(),
            self.namespace.clone(),
            queries.into_iter().map(Into::into).collect(),
        )
    }

    /// Retrieve the current status of a copy operation.
    ///
    /// `GET /v1/namespaces/{namespace}/operations/{token}`
    pub fn poll_copy_from(&self, token: impl Into<String>) -> PollCopyFromRequest {
        PollCopyFromRequest::new(self.client.clone(), self.namespace.clone(), token.into())
    }

    /// Query, filter, full-text search and vector search documents.
    ///
    /// `POST /v2/namespaces/{namespace}/query`
    pub fn query(&self) -> QueryRequest {
        QueryRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Evaluate recall.
    ///
    /// `POST /v1/namespaces/{namespace}/_debug/recall`
    pub fn recall(&self) -> RecallRequest {
        RecallRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Get namespace schema.
    ///
    /// `GET /v1/namespaces/{namespace}/schema`
    pub fn schema(&self) -> SchemaRequest {
        SchemaRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Start copying all documents from another namespace into this one. Returns an operation token
    /// without waiting for the copy to finish. Use the token to poll for progress and the result.
    ///
    ///
    /// `POST /v2/namespaces/{namespace}/async`
    pub fn start_copy_from(&self, source_namespace: impl Into<String>) -> StartCopyFromRequest {
        StartCopyFromRequest::new(
            self.client.clone(),
            self.namespace.clone(),
            source_namespace.into(),
        )
    }

    /// Update metadata configuration for a namespace.
    ///
    /// `PATCH /v1/namespaces/{namespace}/metadata`
    pub fn update_metadata(&self) -> UpdateMetadataRequest {
        UpdateMetadataRequest::new(self.client.clone(), self.namespace.clone())
    }

    /// Update namespace schema.
    ///
    /// `POST /v1/namespaces/{namespace}/schema`
    pub fn update_schema(
        &self,
        schema: impl IntoIterator<Item = (impl Into<String>, impl Into<AttributeSchema>)>,
    ) -> UpdateSchemaRequest {
        UpdateSchemaRequest::new(
            self.client.clone(),
            self.namespace.clone(),
            schema
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }

    /// Create, update, or delete documents.
    ///
    /// `POST /v2/namespaces/{namespace}`
    pub fn write(&self) -> WriteRequest {
        WriteRequest::new(self.client.clone(), self.namespace.clone())
    }
}

/// A request to [`Namespace::branch_from`](Namespace::branch_from).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct BranchFromRequest {
    client: Client,
    namespace: String,
    body: BranchFromBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Serialize)]
struct BranchFromBody {
    source_namespace: String,
}

impl BranchFromRequest {
    pub(crate) fn new(client: Client, namespace: String, source_namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: BranchFromBody { source_namespace },
            options: RequestOptions::default(),
        }
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
        let path = format!("/v2/namespaces/{}", encode_path_segment(&self.namespace));
        let request = ApiRequest::new(Method::POST, path)
            .json(&HashMap::from([("branch_from_namespace", &self.body)]))?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<WriteResult> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for BranchFromRequest {
    type Output = Result<WriteResult>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::copy_from`](Namespace::copy_from).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct CopyFromRequest {
    client: Client,
    namespace: String,
    body: CopyFromBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Serialize)]
struct CopyFromBody {
    source_namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dest_encryption: Option<Encryption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_region: Option<String>,
}

impl CopyFromRequest {
    pub(crate) fn new(client: Client, namespace: String, source_namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: CopyFromBody {
                source_namespace,
                dest_encryption: None,
                source_api_key: None,
                source_region: None,
            },
            options: RequestOptions::default(),
        }
    }

    /// (Optional) The encryption configuration for the destination namespace.
    pub fn dest_encryption(mut self, dest_encryption: impl Into<Encryption>) -> Self {
        self.body.dest_encryption = Some(dest_encryption.into());
        self
    }

    /// (Optional) An API key for the organization containing the source namespace
    pub fn source_api_key(mut self, source_api_key: impl Into<String>) -> Self {
        self.body.source_api_key = Some(source_api_key.into());
        self
    }

    /// (Optional) The region of the source namespace.
    pub fn source_region(mut self, source_region: impl Into<String>) -> Self {
        self.body.source_region = Some(source_region.into());
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
        let path = format!("/v2/namespaces/{}", encode_path_segment(&self.namespace));
        let request = ApiRequest::new(Method::POST, path)
            .json(&HashMap::from([("copy_from_namespace", &self.body)]))?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<WriteResult> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for CopyFromRequest {
    type Output = Result<WriteResult>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::delete_all`](Namespace::delete_all).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct DeleteAllRequest {
    client: Client,
    namespace: String,
    options: RequestOptions,
}

impl DeleteAllRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            options: RequestOptions::default(),
        }
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
        let path = format!("/v2/namespaces/{}", encode_path_segment(&self.namespace));
        let request = ApiRequest::new(Method::DELETE, path).options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceDeleteAllResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for DeleteAllRequest {
    type Output = Result<NamespaceDeleteAllResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::explain_query`](Namespace::explain_query).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct ExplainQueryRequest {
    client: Client,
    namespace: String,
    body: ExplainQueryBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ExplainQueryBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    aggregate_by: Option<HashMap<String, AggregateBy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compute_attributes: Option<HashMap<String, Expr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    consistency: Option<QueryConfigConsistency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_metric: Option<DistanceMetric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_by: Option<Vec<GroupBy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_attributes: Option<IncludeAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<QueryLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rank_by: Option<RankBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector_encoding: Option<VectorEncoding>,
}

impl ExplainQueryRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: ExplainQueryBody::default(),
            options: RequestOptions::default(),
        }
    }

    /// Aggregations to compute over all documents in the namespace that match the filters.
    pub fn aggregate_by(
        mut self,
        aggregate_by: impl IntoIterator<Item = (impl Into<String>, impl Into<AggregateBy>)>,
    ) -> Self {
        self.body.aggregate_by = Some(
            aggregate_by
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Computes additional values on documents returned by a query. Each key is the name of the
    /// computed attribute; each value is an expression describing how to compute it.
    pub fn compute_attributes(
        mut self,
        compute_attributes: impl IntoIterator<Item = (impl Into<String>, impl Into<Expr>)>,
    ) -> Self {
        self.body.compute_attributes = Some(
            compute_attributes
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// The consistency level for a query.
    pub fn consistency(mut self, consistency: impl Into<QueryConfigConsistency>) -> Self {
        self.body.consistency = Some(consistency.into());
        self
    }

    /// A function used to calculate vector similarity.
    pub fn distance_metric(mut self, distance_metric: impl Into<DistanceMetric>) -> Self {
        self.body.distance_metric = Some(distance_metric.into());
        self
    }

    /// List of attribute names to exclude from the response. All other attributes will be included in
    /// the response.
    pub fn exclude_attributes(
        mut self,
        exclude_attributes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.body.exclude_attributes =
            Some(exclude_attributes.into_iter().map(Into::into).collect());
        self
    }

    /// Exact filters for attributes to refine search results for. Think of it as a SQL WHERE clause.
    pub fn filters(mut self, filters: impl Into<Filter>) -> Self {
        self.body.filters = Some(filters.into());
        self
    }

    /// Groups documents by the specified attributes (the "group key") before computing aggregates.
    /// Aggregates are computed separately for each group.
    pub fn group_by(mut self, group_by: impl IntoIterator<Item = impl Into<GroupBy>>) -> Self {
        self.body.group_by = Some(group_by.into_iter().map(Into::into).collect());
        self
    }

    /// Whether to include attributes in the response.
    pub fn include_attributes(mut self, include_attributes: impl Into<IncludeAttributes>) -> Self {
        self.body.include_attributes = Some(include_attributes.into());
        self
    }

    /// Limits the documents returned by a query.
    pub fn limit(mut self, limit: impl Into<QueryLimit>) -> Self {
        self.body.limit = Some(limit.into());
        self
    }

    /// Number of documents to skip before returning results. Supported only in v2 queries with an
    /// explicit `rank_by` and `top_k` or `limit`.
    pub fn offset(mut self, offset: u64) -> Self {
        self.body.offset = Some(offset);
        self
    }

    /// How to rank the documents in the namespace.
    pub fn rank_by(mut self, rank_by: impl Into<RankBy>) -> Self {
        self.body.rank_by = Some(rank_by.into());
        self
    }

    /// The number of results to return.
    pub fn top_k(mut self, top_k: u64) -> Self {
        self.body.top_k = Some(top_k);
        self
    }

    /// The encoding to use for vectors in the response.
    pub fn vector_encoding(mut self, vector_encoding: impl Into<VectorEncoding>) -> Self {
        self.body.vector_encoding = Some(vector_encoding.into());
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
        let path = format!(
            "/v2/namespaces/{}/explain_query",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::POST, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceExplainQueryResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for ExplainQueryRequest {
    type Output = Result<NamespaceExplainQueryResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::hint_cache_warm`](Namespace::hint_cache_warm).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct HintCacheWarmRequest {
    client: Client,
    namespace: String,
    options: RequestOptions,
}

impl HintCacheWarmRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            options: RequestOptions::default(),
        }
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
        let path = format!(
            "/v1/namespaces/{}/hint_cache_warm",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::GET, path).options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceHintCacheWarmResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for HintCacheWarmRequest {
    type Output = Result<NamespaceHintCacheWarmResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::metadata`](Namespace::metadata).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct MetadataRequest {
    client: Client,
    namespace: String,
    options: RequestOptions,
}

impl MetadataRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            options: RequestOptions::default(),
        }
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
        let path = format!(
            "/v2/namespaces/{}/metadata",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::GET, path).options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceMetadata> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for MetadataRequest {
    type Output = Result<NamespaceMetadata>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::multi_query`](Namespace::multi_query).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct MultiQueryRequest {
    client: Client,
    namespace: String,
    body: MultiQueryBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Serialize)]
struct MultiQueryBody {
    queries: Vec<Query>,
    #[serde(skip_serializing_if = "Option::is_none")]
    consistency: Option<QueryConfigConsistency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<MultiQueryLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rerank_by: Option<RerankBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector_encoding: Option<VectorEncoding>,
}

impl MultiQueryRequest {
    pub(crate) fn new(client: Client, namespace: String, queries: Vec<Query>) -> Self {
        Self {
            client,
            namespace,
            body: MultiQueryBody {
                queries,
                consistency: None,
                limit: None,
                offset: None,
                rerank_by: None,
                vector_encoding: None,
            },
            options: RequestOptions::default(),
        }
    }

    /// The consistency level for a query.
    pub fn consistency(mut self, consistency: impl Into<QueryConfigConsistency>) -> Self {
        self.body.consistency = Some(consistency.into());
        self
    }

    /// Limits the total number of reranked documents returned.
    pub fn limit(mut self, limit: impl Into<MultiQueryLimit>) -> Self {
        self.body.limit = Some(limit.into());
        self
    }

    /// Number of reranked documents to skip before returning results. Requires `rerank_by` and `limit`.
    pub fn offset(mut self, offset: u64) -> Self {
        self.body.offset = Some(offset);
        self
    }

    /// How to combine the rows returned by each sub-query into a single ranked list.
    pub fn rerank_by(mut self, rerank_by: impl Into<RerankBy>) -> Self {
        self.body.rerank_by = Some(rerank_by.into());
        self
    }

    /// The encoding to use for vectors in the response.
    pub fn vector_encoding(mut self, vector_encoding: impl Into<VectorEncoding>) -> Self {
        self.body.vector_encoding = Some(vector_encoding.into());
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
        let path = format!(
            "/v2/namespaces/{}/query",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::POST, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<MultiQueryResult> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for MultiQueryRequest {
    type Output = Result<MultiQueryResult>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::poll_copy_from`](Namespace::poll_copy_from).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct PollCopyFromRequest {
    client: Client,
    namespace: String,
    token: String,
    options: RequestOptions,
}

impl PollCopyFromRequest {
    pub(crate) fn new(client: Client, namespace: String, token: String) -> Self {
        Self {
            client,
            namespace,
            token,
            options: RequestOptions::default(),
        }
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
        let path = format!(
            "/v1/namespaces/{}/operations/{}",
            encode_path_segment(&self.namespace),
            encode_path_segment(&self.token)
        );
        let request = ApiRequest::new(Method::GET, path).options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<CopyFromNamespaceOperation> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for PollCopyFromRequest {
    type Output = Result<CopyFromNamespaceOperation>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::query`](Namespace::query).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct QueryRequest {
    client: Client,
    namespace: String,
    body: QueryBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Default, Serialize)]
struct QueryBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    aggregate_by: Option<HashMap<String, AggregateBy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compute_attributes: Option<HashMap<String, Expr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    consistency: Option<QueryConfigConsistency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_metric: Option<DistanceMetric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_by: Option<Vec<GroupBy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_attributes: Option<IncludeAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<QueryLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rank_by: Option<RankBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rerank_by: Option<RerankModelParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector_encoding: Option<VectorEncoding>,
}

impl QueryRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: QueryBody::default(),
            options: RequestOptions::default(),
        }
    }

    /// Aggregations to compute over all documents in the namespace that match the filters.
    pub fn aggregate_by(
        mut self,
        aggregate_by: impl IntoIterator<Item = (impl Into<String>, impl Into<AggregateBy>)>,
    ) -> Self {
        self.body.aggregate_by = Some(
            aggregate_by
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Computes additional values on documents returned by a query. Each key is the name of the
    /// computed attribute; each value is an expression describing how to compute it.
    pub fn compute_attributes(
        mut self,
        compute_attributes: impl IntoIterator<Item = (impl Into<String>, impl Into<Expr>)>,
    ) -> Self {
        self.body.compute_attributes = Some(
            compute_attributes
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// The consistency level for a query.
    pub fn consistency(mut self, consistency: impl Into<QueryConfigConsistency>) -> Self {
        self.body.consistency = Some(consistency.into());
        self
    }

    /// A function used to calculate vector similarity.
    pub fn distance_metric(mut self, distance_metric: impl Into<DistanceMetric>) -> Self {
        self.body.distance_metric = Some(distance_metric.into());
        self
    }

    /// List of attribute names to exclude from the response. All other attributes will be included in
    /// the response.
    pub fn exclude_attributes(
        mut self,
        exclude_attributes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.body.exclude_attributes =
            Some(exclude_attributes.into_iter().map(Into::into).collect());
        self
    }

    /// Exact filters for attributes to refine search results for. Think of it as a SQL WHERE clause.
    pub fn filters(mut self, filters: impl Into<Filter>) -> Self {
        self.body.filters = Some(filters.into());
        self
    }

    /// Groups documents by the specified attributes (the "group key") before computing aggregates.
    /// Aggregates are computed separately for each group.
    pub fn group_by(mut self, group_by: impl IntoIterator<Item = impl Into<GroupBy>>) -> Self {
        self.body.group_by = Some(group_by.into_iter().map(Into::into).collect());
        self
    }

    /// Whether to include attributes in the response.
    pub fn include_attributes(mut self, include_attributes: impl Into<IncludeAttributes>) -> Self {
        self.body.include_attributes = Some(include_attributes.into());
        self
    }

    /// Limits the documents returned by a query.
    pub fn limit(mut self, limit: impl Into<QueryLimit>) -> Self {
        self.body.limit = Some(limit.into());
        self
    }

    /// Number of documents to skip before returning results. Supported only in v2 queries with an
    /// explicit `rank_by` and `top_k` or `limit`.
    pub fn offset(mut self, offset: u64) -> Self {
        self.body.offset = Some(offset);
        self
    }

    /// How to rank the documents in the namespace.
    pub fn rank_by(mut self, rank_by: impl Into<RankBy>) -> Self {
        self.body.rank_by = Some(rank_by.into());
        self
    }

    /// Rerank retrieved candidates with a model.
    pub fn rerank_by(mut self, rerank_by: impl Into<RerankModelParams>) -> Self {
        self.body.rerank_by = Some(rerank_by.into());
        self
    }

    /// The number of results to return.
    pub fn top_k(mut self, top_k: u64) -> Self {
        self.body.top_k = Some(top_k);
        self
    }

    /// The encoding to use for vectors in the response.
    pub fn vector_encoding(mut self, vector_encoding: impl Into<VectorEncoding>) -> Self {
        self.body.vector_encoding = Some(vector_encoding.into());
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
        let path = format!(
            "/v2/namespaces/{}/query",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::POST, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<QueryResult> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for QueryRequest {
    type Output = Result<QueryResult>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::recall`](Namespace::recall).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct RecallRequest {
    client: Client,
    namespace: String,
    body: RecallBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Default, Serialize)]
struct RecallBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_ground_truth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rank_by: Option<RankBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u64>,
}

impl RecallRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: RecallBody::default(),
            options: RequestOptions::default(),
        }
    }

    /// Filter by attributes. Same syntax as the query endpoint.
    pub fn filters(mut self, filters: impl Into<Filter>) -> Self {
        self.body.filters = Some(filters.into());
        self
    }

    /// Include ground truth data (query vectors and true nearest neighbors) in the response.
    pub fn include_ground_truth(mut self, include_ground_truth: bool) -> Self {
        self.body.include_ground_truth = Some(include_ground_truth);
        self
    }

    /// The number of searches to run.
    pub fn num(mut self, num: u64) -> Self {
        self.body.num = Some(num);
        self
    }

    /// The ranking function to evaluate recall for. If provided, `num` must be either null or 1.
    pub fn rank_by(mut self, rank_by: impl Into<RankBy>) -> Self {
        self.body.rank_by = Some(rank_by.into());
        self
    }

    /// Search for `top_k` nearest neighbors.
    pub fn top_k(mut self, top_k: u64) -> Self {
        self.body.top_k = Some(top_k);
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
        let path = format!(
            "/v1/namespaces/{}/_debug/recall",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::POST, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceRecallResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for RecallRequest {
    type Output = Result<NamespaceRecallResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::schema`](Namespace::schema).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct SchemaRequest {
    client: Client,
    namespace: String,
    options: RequestOptions,
}

impl SchemaRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            options: RequestOptions::default(),
        }
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
        let path = format!(
            "/v1/namespaces/{}/schema",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::GET, path).options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceSchemaResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for SchemaRequest {
    type Output = Result<NamespaceSchemaResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::start_copy_from`](Namespace::start_copy_from).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct StartCopyFromRequest {
    client: Client,
    namespace: String,
    body: StartCopyFromBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Serialize)]
struct StartCopyFromBody {
    source_namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dest_encryption: Option<Encryption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_region: Option<String>,
}

impl StartCopyFromRequest {
    pub(crate) fn new(client: Client, namespace: String, source_namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: StartCopyFromBody {
                source_namespace,
                dest_encryption: None,
                source_api_key: None,
                source_region: None,
            },
            options: RequestOptions::default(),
        }
    }

    /// (Optional) The encryption configuration for the destination namespace.
    pub fn dest_encryption(mut self, dest_encryption: impl Into<Encryption>) -> Self {
        self.body.dest_encryption = Some(dest_encryption.into());
        self
    }

    /// (Optional) An API key for the organization containing the source namespace
    pub fn source_api_key(mut self, source_api_key: impl Into<String>) -> Self {
        self.body.source_api_key = Some(source_api_key.into());
        self
    }

    /// (Optional) The region of the source namespace.
    pub fn source_region(mut self, source_region: impl Into<String>) -> Self {
        self.body.source_region = Some(source_region.into());
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
        let path = format!(
            "/v2/namespaces/{}/async",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::POST, path)
            .json(&HashMap::from([("copy_from_namespace", &self.body)]))?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceStartCopyFromResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for StartCopyFromRequest {
    type Output = Result<NamespaceStartCopyFromResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::update_metadata`](Namespace::update_metadata).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct UpdateMetadataRequest {
    client: Client,
    namespace: String,
    body: UpdateMetadataBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Default, Serialize)]
struct UpdateMetadataBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pinning: Option<NamespaceMetadataPatchPinning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_only: Option<bool>,
}

impl UpdateMetadataRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: UpdateMetadataBody::default(),
            options: RequestOptions::default(),
        }
    }

    /// Configuration for namespace pinning. - Missing field: no change to pinning configuration -
    /// `null` or `false`: explicitly remove pinning - `true`: enable pinning with default configuration
    /// - Object: set pinning configuration
    pub fn pinning(mut self, pinning: impl Into<NamespaceMetadataPatchPinning>) -> Self {
        self.body.pinning = Some(pinning.into());
        self
    }

    /// Set to `true` to reject document and schema writes, or `false` to allow them. Writes already in
    /// progress may still commit. Metadata updates remain available.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.body.read_only = Some(read_only);
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
        let path = format!(
            "/v1/namespaces/{}/metadata",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::PATCH, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceMetadata> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for UpdateMetadataRequest {
    type Output = Result<NamespaceMetadata>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::update_schema`](Namespace::update_schema).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct UpdateSchemaRequest {
    client: Client,
    namespace: String,
    body: HashMap<String, AttributeSchema>,
    options: RequestOptions,
}

impl UpdateSchemaRequest {
    pub(crate) fn new(
        client: Client,
        namespace: String,
        schema: HashMap<String, AttributeSchema>,
    ) -> Self {
        Self {
            client,
            namespace,
            body: schema,
            options: RequestOptions::default(),
        }
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
        let path = format!(
            "/v1/namespaces/{}/schema",
            encode_path_segment(&self.namespace)
        );
        let request = ApiRequest::new(Method::POST, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<NamespaceUpdateSchemaResponse> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for UpdateSchemaRequest {
    type Output = Result<NamespaceUpdateSchemaResponse>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A request to [`Namespace::write`](Namespace::write).
///
/// Set optional parameters with the builder methods, then send it with [`send`](Self::send) or by
/// `.await`ing it directly.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it's sent with `.send()` or awaited"]
pub struct WriteRequest {
    client: Client,
    namespace: String,
    body: WriteBody,
    options: RequestOptions,
}

#[derive(Debug, Clone, Default, Serialize)]
struct WriteBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_from_namespace: Option<BranchFromNamespaceParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    copy_from_namespace: Option<CopyFromNamespaceParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    create_namespace: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_by_filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_by_filter_allow_partial: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_condition: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deletes: Option<Vec<Id>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_backpressure: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_metric: Option<DistanceMetric>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption: Option<Encryption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch_by_filter: Option<PatchByFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch_by_filter_allow_partial: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch_columns: Option<Columns>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch_condition: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch_rows: Option<Vec<Row>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_affected_ids: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<HashMap<String, AttributeSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sharding: Option<ShardingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upsert_columns: Option<Columns>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upsert_condition: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upsert_rows: Option<Vec<Row>>,
}

impl WriteRequest {
    pub(crate) fn new(client: Client, namespace: String) -> Self {
        Self {
            client,
            namespace,
            body: WriteBody::default(),
            options: RequestOptions {
                max_retries: Some(6),
                ..Default::default()
            },
        }
    }

    /// The namespace to create an instant, copy-on-write clone of.
    pub fn branch_from_namespace(
        mut self,
        branch_from_namespace: impl Into<BranchFromNamespaceParams>,
    ) -> Self {
        self.body.branch_from_namespace = Some(branch_from_namespace.into());
        self
    }

    /// The namespace to copy documents from.
    pub fn copy_from_namespace(
        mut self,
        copy_from_namespace: impl Into<CopyFromNamespaceParams>,
    ) -> Self {
        self.body.copy_from_namespace = Some(copy_from_namespace.into());
        self
    }

    /// If `true`, ensures the namespace is created, even if the request writes no documents. Creating
    /// an empty namespace requires the `id` type to be declared in `schema`. If `false`, a namespace is
    /// never created, and a 404 is returned if it does not exist. If omitted, a namespace is created by
    /// the first request that writes documents.
    pub fn create_namespace(mut self, create_namespace: bool) -> Self {
        self.body.create_namespace = Some(create_namespace);
        self
    }

    /// The filter specifying which documents to delete.
    pub fn delete_by_filter(mut self, delete_by_filter: impl Into<Filter>) -> Self {
        self.body.delete_by_filter = Some(delete_by_filter.into());
        self
    }

    /// Allow partial completion when filter matches too many documents.
    pub fn delete_by_filter_allow_partial(mut self, delete_by_filter_allow_partial: bool) -> Self {
        self.body.delete_by_filter_allow_partial = Some(delete_by_filter_allow_partial);
        self
    }

    /// A condition evaluated against the current value of each document targeted by a delete write.
    /// Only documents that pass the condition are deleted.
    pub fn delete_condition(mut self, delete_condition: impl Into<Filter>) -> Self {
        self.body.delete_condition = Some(delete_condition.into());
        self
    }

    /// Body param
    pub fn deletes(mut self, deletes: impl IntoIterator<Item = impl Into<Id>>) -> Self {
        self.body.deletes = Some(deletes.into_iter().map(Into::into).collect());
        self
    }

    /// Disables write throttling (HTTP 429 responses) during high-volume ingestion.
    pub fn disable_backpressure(mut self, disable_backpressure: bool) -> Self {
        self.body.disable_backpressure = Some(disable_backpressure);
        self
    }

    /// A function used to calculate vector similarity.
    pub fn distance_metric(mut self, distance_metric: impl Into<DistanceMetric>) -> Self {
        self.body.distance_metric = Some(distance_metric.into());
        self
    }

    /// The encryption configuration for a namespace.
    pub fn encryption(mut self, encryption: impl Into<Encryption>) -> Self {
        self.body.encryption = Some(encryption.into());
        self
    }

    /// The patch and filter specifying which documents to patch.
    pub fn patch_by_filter(mut self, patch_by_filter: impl Into<PatchByFilter>) -> Self {
        self.body.patch_by_filter = Some(patch_by_filter.into());
        self
    }

    /// Allow partial completion when filter matches too many documents.
    pub fn patch_by_filter_allow_partial(mut self, patch_by_filter_allow_partial: bool) -> Self {
        self.body.patch_by_filter_allow_partial = Some(patch_by_filter_allow_partial);
        self
    }

    /// A list of documents in columnar format. Each key is a column name, mapped to an array of values
    /// for that column.
    pub fn patch_columns(mut self, patch_columns: impl Into<Columns>) -> Self {
        self.body.patch_columns = Some(patch_columns.into());
        self
    }

    /// A condition evaluated against the current value of each document targeted by a patch write. Only
    /// documents that pass the condition are patched.
    pub fn patch_condition(mut self, patch_condition: impl Into<Filter>) -> Self {
        self.body.patch_condition = Some(patch_condition.into());
        self
    }

    /// Body param
    pub fn patch_rows(mut self, patch_rows: impl IntoIterator<Item = impl Into<Row>>) -> Self {
        self.body.patch_rows = Some(patch_rows.into_iter().map(Into::into).collect());
        self
    }

    /// If true, return the IDs of affected rows (deleted, patched, upserted) in the response. For
    /// filtered and conditional writes, only IDs for writes that succeeded will be included.
    pub fn return_affected_ids(mut self, return_affected_ids: bool) -> Self {
        self.body.return_affected_ids = Some(return_affected_ids);
        self
    }

    /// The schema of the attributes attached to the documents.
    pub fn schema(
        mut self,
        schema: impl IntoIterator<Item = (impl Into<String>, impl Into<AttributeSchema>)>,
    ) -> Self {
        self.body.schema = Some(
            schema
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Configuration for namespace sharding, which partitions a namespace's documents across multiple
    /// internal shards to scale indexing and query throughput beyond a single machine. Sharding can
    /// only be configured on a namespace's inaugural write, and cannot be added to or changed on an
    /// existing namespace.
    pub fn sharding(mut self, sharding: impl Into<ShardingConfig>) -> Self {
        self.body.sharding = Some(sharding.into());
        self
    }

    /// A list of documents in columnar format. Each key is a column name, mapped to an array of values
    /// for that column.
    pub fn upsert_columns(mut self, upsert_columns: impl Into<Columns>) -> Self {
        self.body.upsert_columns = Some(upsert_columns.into());
        self
    }

    /// A condition evaluated against the current value of each document targeted by an upsert write.
    /// Only documents that pass the condition are upserted.
    pub fn upsert_condition(mut self, upsert_condition: impl Into<Filter>) -> Self {
        self.body.upsert_condition = Some(upsert_condition.into());
        self
    }

    /// Body param
    pub fn upsert_rows(mut self, upsert_rows: impl IntoIterator<Item = impl Into<Row>>) -> Self {
        self.body.upsert_rows = Some(upsert_rows.into_iter().map(Into::into).collect());
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
        let path = format!("/v2/namespaces/{}", encode_path_segment(&self.namespace));
        let request = ApiRequest::new(Method::POST, path)
            .json(&self.body)?
            .options(self.options);
        Ok((self.client, request))
    }

    /// Sends the request.
    pub async fn send(self) -> Result<WriteResult> {
        let (client, request) = self.into_parts()?;
        client.execute(request).await
    }
}

impl IntoFuture for WriteRequest {
    type Output = Result<WriteResult>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}
