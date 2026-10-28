//! Types used by the `namespaces` resource.

#[allow(unused_imports)]
use super::*;

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The response to a successful write request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WriteResult {
    /// The billing information for a write request.
    pub billing: WriteBilling,
    /// A message describing the result of the write request.
    pub message: String,
    /// The number of rows affected by the write request.
    pub rows_affected: u64,
    /// The status of the request.
    ///
    /// Always `"OK"`.
    #[serde(default)]
    pub status: String,
    /// The IDs of documents that were deleted. Only included when `return_affected_ids` is true and
    /// at least one document was deleted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deleted_ids: Vec<Id>,
    /// The IDs of documents that were patched. Only included when `return_affected_ids` is true and
    /// at least one document was patched.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patched_ids: Vec<Id>,
    /// The performance information for a write request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance: Option<WritePerformance>,
    /// The number of rows deleted by the write request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_deleted: Option<u64>,
    /// The number of rows patched by the write request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_patched: Option<u64>,
    /// Whether more documents match the filter for partial operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_remaining: Option<bool>,
    /// The number of rows upserted by the write request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_upserted: Option<u64>,
    /// The IDs of documents that were upserted. Only included when `return_affected_ids` is true
    /// and at least one document was upserted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upserted_ids: Vec<Id>,
}

/// The billing information for a write request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WriteBilling {
    /// The number of billable logical bytes written to the namespace.
    pub billable_logical_bytes_written: u64,
    /// The billing information for a query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<QueryBilling>,
}

/// The billing information for a query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueryBilling {
    /// The number of billable logical bytes queried from the namespace.
    pub billable_logical_bytes_queried: u64,
    /// The number of billable logical bytes returned from the query.
    pub billable_logical_bytes_returned: u64,
}

/// An identifier for a document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum Id {
    /// An unsigned 64-bit integer ID.
    Int(u64),
    /// A string ID.
    String(String),
    /// A value not known to this version of the SDK.
    Other(Value),
}

impl From<u64> for Id {
    fn from(value: u64) -> Self {
        Self::Int(value)
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(value) => value.fmt(f),
            Self::String(value) => value.fmt(f),
            Self::Other(value) => value.fmt(f),
        }
    }
}

impl From<Id> for Value {
    fn from(value: Id) -> Self {
        match value {
            Id::Int(value) => value.into(),
            Id::String(value) => value.into(),
            Id::Other(value) => value,
        }
    }
}

impl From<&Id> for Value {
    fn from(value: &Id) -> Self {
        value.clone().into()
    }
}

/// The performance information for a write request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WritePerformance {
    /// Request time measured on the server, in milliseconds.
    pub server_total_ms: u64,
    /// Time spent embedding text, in milliseconds. Only set when using a native embedding model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_ms: Option<u64>,
    /// The number of tokens embedded. Only set when using a native embedding model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_tokens: Option<u64>,
}

/// The encryption configuration for a namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode")]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum Encryption {
    /// Encrypt the namespace with a customer-managed encryption key (CMEK).
    #[serde(rename = "customer-managed")]
    CustomerManaged(EncryptionCustomerManaged),
    /// Use the default server-side encryption (SSE).
    #[serde(rename = "default")]
    Default,
    /// A variant not known to this version of the SDK.
    #[serde(other)]
    Other,
}

impl From<EncryptionCustomerManaged> for Encryption {
    fn from(value: EncryptionCustomerManaged) -> Self {
        Self::CustomerManaged(value)
    }
}

/// Encrypt the namespace with a customer-managed encryption key (CMEK).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EncryptionCustomerManaged {
    /// The identifier of the CMEK key to use for encryption. For GCP, the fully-qualified resource
    /// name of the key. For AWS, the ARN of the key.
    pub key_name: String,
    /// The AWS KMS encryption context to send with every request that uses the key. Only supported
    /// on AWS, and must be enabled for your organization. Keys may not start with `tpuf:` or
    /// `aws:`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption_context: Option<BTreeMap<String, String>>,
}

impl EncryptionCustomerManaged {
    /// Creates a new [`EncryptionCustomerManaged`] from its required fields.
    pub fn new(key_name: impl Into<String>) -> Self {
        Self {
            key_name: key_name.into(),
            encryption_context: Default::default(),
        }
    }

    /// The AWS KMS encryption context to send with every request that uses the key. Only supported on
    /// AWS, and must be enabled for your organization. Keys may not start with `tpuf:` or `aws:`.
    ///
    /// Adds to any entries already set, rather than replacing them.
    pub fn encryption_context(
        mut self,
        encryption_context: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.encryption_context
            .get_or_insert_with(Default::default)
            .extend(
                encryption_context
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into())),
            );
        self
    }
}

impl From<String> for EncryptionCustomerManaged {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for EncryptionCustomerManaged {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// The response to a successful namespace deletion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DeleteAllResponse {
    /// The status of the request.
    ///
    /// Always `"OK"`.
    #[serde(default)]
    pub status: String,
}

impl Default for DeleteAllResponse {
    fn default() -> Self {
        Self {
            status: "OK".to_owned(),
        }
    }
}

/// An aggregate function to compute over all documents in the namespace.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`AggregateBy::count`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AggregateBy {
    /// Count documents.
    Count,
    /// Sum the values of the given attribute.
    Sum {
        /// The name of the attribute to sum (must be of numeric type).
        attr: String,
    },
}

#[allow(deprecated)]
impl AggregateBy {
    /// Count documents.
    pub fn count() -> Self {
        Self::Count
    }

    /// Sum the values of the given attribute.
    pub fn sum(attr: impl Into<String>) -> Self {
        Self::Sum { attr: attr.into() }
    }
}

#[allow(deprecated)]
impl Serialize for AggregateBy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Count => ("Count",).serialize(serializer),
            Self::Sum { attr } => ("Sum", attr).serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for AggregateBy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if tuple_elements(&value, 1, &[(0, "Count")]).is_some() {
            return Ok(Self::Count);
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Sum")]) {
            if let Some(attr) = decode_element(&elements[1]) {
                return Ok(Self::Sum { attr });
            }
        }
        Err(serde::de::Error::custom(format!(
            "invalid AggregateBy: {value}"
        )))
    }
}

impl From<AggregateBy> for Value {
    fn from(value: AggregateBy) -> Self {
        to_json(value)
    }
}

/// An expression that computes a value.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`Expr::ref_new`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Expr {
    /// A reference to an attribute in a new document.
    RefNew(ExprRefNew),
    /// Embed the input text using the embedding model configured for the attribute the expression
    /// is evaluated against.
    Embed {
        /// The input text to embed.
        value: String,
        /// Additional (optional) parameters for the Embed expression.
        params: Option<EmbedParams>,
    },
    /// The distance between a vector attribute and a query vector.
    VectorDist {
        /// The name of the vector attribute.
        attr: String,
        /// The query vector.
        value: Vec<f32>,
    },
    /// Highlights matching fragments of a text attribute.
    Highlight {
        /// The name of the text attribute to highlight.
        attr: String,
        /// Additional (optional) parameters for the Highlight compute expression.
        config: Option<HighlightConfigParams>,
    },
    /// The score of a ranking function, e.g. to return the score of one clause of a composite
    /// `rank_by`.
    Score(Box<RankBy>),
}

#[allow(deprecated)]
impl Expr {
    /// Embed the input text using the embedding model configured for the attribute the expression is
    /// evaluated against.
    pub fn embed(value: impl Into<String>) -> Self {
        Self::Embed {
            value: value.into(),
            params: None,
        }
    }

    /// Embed the input text using the embedding model configured for the attribute the expression is
    /// evaluated against.
    ///
    /// Like [`Expr::embed`], with `params`.
    pub fn embed_with_params(value: impl Into<String>, params: impl Into<EmbedParams>) -> Self {
        Self::Embed {
            value: value.into(),
            params: Some(params.into()),
        }
    }

    /// The distance between a vector attribute and a query vector.
    pub fn vector_dist(attr: impl Into<String>, value: impl Into<Vec<f32>>) -> Self {
        Self::VectorDist {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Highlights matching fragments of a text attribute.
    pub fn highlight(attr: impl Into<String>) -> Self {
        Self::Highlight {
            attr: attr.into(),
            config: None,
        }
    }

    /// Highlights matching fragments of a text attribute.
    ///
    /// Like [`Expr::highlight`], with `config`.
    pub fn highlight_with_config(
        attr: impl Into<String>,
        config: impl Into<HighlightConfigParams>,
    ) -> Self {
        Self::Highlight {
            attr: attr.into(),
            config: Some(config.into()),
        }
    }

    /// A reference to an attribute in a new document.
    pub fn ref_new(value: impl Into<ExprRefNew>) -> Self {
        Self::RefNew(value.into())
    }

    /// The score of a ranking function, e.g. to return the score of one clause of a composite
    /// `rank_by`.
    pub fn score(value: impl Into<RankBy>) -> Self {
        Self::Score(Box::new(value.into()))
    }
}

impl From<ExprRefNew> for Expr {
    fn from(value: ExprRefNew) -> Self {
        Self::RefNew(value)
    }
}

impl From<RankBy> for Expr {
    fn from(value: RankBy) -> Self {
        Self::Score(Box::new(value))
    }
}

impl From<RankByText> for Expr {
    fn from(value: RankByText) -> Self {
        Self::Score(Box::new(RankBy::from(value)))
    }
}

#[allow(deprecated)]
impl Serialize for Expr {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::RefNew(value) => value.serialize(serializer),
            Self::Embed {
                value,
                params: None,
            } => ("Embed", value).serialize(serializer),
            Self::Embed {
                value,
                params: Some(params),
            } => ("Embed", value, params).serialize(serializer),
            Self::VectorDist { attr, value } => (attr, "VectorDist", value).serialize(serializer),
            Self::Highlight { attr, config: None } => ("Highlight", attr).serialize(serializer),
            Self::Highlight {
                attr,
                config: Some(config),
            } => ("Highlight", attr, config).serialize(serializer),
            Self::Score(value) => value.serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(value) = decode_element(&value) {
            return Ok(Self::RefNew(value));
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(0, "Embed")]) {
            if let (Some(value), Some(params)) =
                (decode_element(&elements[1]), decode_element(&elements[2]))
            {
                return Ok(Self::Embed {
                    value,
                    params: Some(params),
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Embed")]) {
            if let Some(value) = decode_element(&elements[1]) {
                return Ok(Self::Embed {
                    value,
                    params: None,
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "VectorDist")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::VectorDist { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(0, "Highlight")]) {
            if let (Some(attr), Some(config)) =
                (decode_element(&elements[1]), decode_element(&elements[2]))
            {
                return Ok(Self::Highlight {
                    attr,
                    config: Some(config),
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Highlight")]) {
            if let Some(attr) = decode_element(&elements[1]) {
                return Ok(Self::Highlight { attr, config: None });
            }
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Score(value));
        }
        Err(serde::de::Error::custom(format!("invalid Expr: {value}")))
    }
}

impl From<Expr> for Value {
    fn from(value: Expr) -> Self {
        to_json(value)
    }
}

/// A reference to an attribute in a new document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExprRefNew {
    /// The name of the attribute referenced in the new document.
    #[serde(rename = "$ref_new")]
    pub ref_new: String,
}

impl ExprRefNew {
    /// Creates a new [`ExprRefNew`] from its required fields.
    pub fn new(ref_new: impl Into<String>) -> Self {
        Self {
            ref_new: ref_new.into(),
        }
    }
}

impl From<ExprRefNew> for Value {
    fn from(value: ExprRefNew) -> Self {
        to_json(value)
    }
}

impl From<String> for ExprRefNew {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ExprRefNew {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Additional (optional) parameters for the Embed expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct EmbedParams {
    /// The model to use for embedding, overriding the model configured for the attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl EmbedParams {
    /// Creates a new [`EmbedParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// The model to use for embedding, overriding the model configured for the attribute.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Additional (optional) parameters for the Highlight compute expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct HighlightConfigParams {
    /// How to split a text attribute into fragments for highlighting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment_by: Option<HighlightFragmentBy>,
    /// The maximum number of fragments to return. Defaults to `3`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment_limit: Option<u64>,
    /// The units to report highlighted fragment offsets in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_offsets: Option<HighlightOffsetUnits>,
    /// How to rank candidate fragments within the attribute before selecting the top
    /// `fragment_limit`. Defaults to the query's `rank_by`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_fragments_by: Option<RankBy>,
}

impl HighlightConfigParams {
    /// Creates a new [`HighlightConfigParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// How to split a text attribute into fragments for highlighting.
    pub fn fragment_by(mut self, fragment_by: impl Into<HighlightFragmentBy>) -> Self {
        self.fragment_by = Some(fragment_by.into());
        self
    }

    /// The maximum number of fragments to return. Defaults to `3`.
    pub fn fragment_limit(mut self, fragment_limit: u64) -> Self {
        self.fragment_limit = Some(fragment_limit);
        self
    }

    /// The units to report highlighted fragment offsets in.
    pub fn include_offsets(mut self, include_offsets: impl Into<HighlightOffsetUnits>) -> Self {
        self.include_offsets = Some(include_offsets.into());
        self
    }

    /// How to rank candidate fragments within the attribute before selecting the top `fragment_limit`.
    /// Defaults to the query's `rank_by`.
    pub fn rank_fragments_by(mut self, rank_fragments_by: impl Into<RankBy>) -> Self {
        self.rank_fragments_by = Some(rank_fragments_by.into());
        self
    }
}

/// How to split a text attribute into fragments for highlighting.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`HighlightFragmentBy::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum HighlightFragmentBy {
    /// Treat the whole attribute as a single fragment.
    #[serde(rename = "none")]
    None,
    /// Split the attribute into sentences. This is the default.
    #[serde(rename = "sentence")]
    Sentence,
    /// Split the attribute into paragraphs.
    #[serde(rename = "paragraph")]
    Paragraph,
    /// Split the attribute into individual words.
    #[serde(rename = "word")]
    Word,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl HighlightFragmentBy {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Sentence => "sentence",
            Self::Paragraph => "paragraph",
            Self::Word => "word",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for HighlightFragmentBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for HighlightFragmentBy {
    fn from(value: &str) -> Self {
        match value {
            "none" => Self::None,
            "sentence" => Self::Sentence,
            "paragraph" => Self::Paragraph,
            "word" => Self::Word,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for HighlightFragmentBy {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// The units to report highlighted fragment offsets in.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`HighlightOffsetUnits::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum HighlightOffsetUnits {
    /// `utf-8`
    #[serde(rename = "utf-8")]
    Utf8,
    /// `utf-16`
    #[serde(rename = "utf-16")]
    Utf16,
    /// `codepoints`
    #[serde(rename = "codepoints")]
    Codepoints,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl HighlightOffsetUnits {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf16 => "utf-16",
            Self::Codepoints => "codepoints",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for HighlightOffsetUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for HighlightOffsetUnits {
    fn from(value: &str) -> Self {
        match value {
            "utf-8" => Self::Utf8,
            "utf-16" => Self::Utf16,
            "codepoints" => Self::Codepoints,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for HighlightOffsetUnits {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// How to rank documents: by vector distance, full-text search relevance, or the value of an
/// attribute.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`RankBy::ann`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RankBy {
    /// Rank by approximate nearest neighbor (ANN) search, ordering documents by the distance
    /// between their vector and the query vector, closest first.
    Ann {
        /// The name of the attribute to rank by.
        attr: String,
        value: RankByAnnValue,
    },
    /// Rank by exact nearest neighbor (kNN) search, ordering documents by the distance between
    /// their vector and the query vector, closest first. Requires `filters`.
    Knn {
        /// The name of the attribute to rank by.
        attr: String,
        value: RankByKnnValue,
    },
    /// Rank by the distance between a sparse vector attribute and a sparse query vector.
    SparseKnn {
        /// The name of the sparse vector attribute to rank by.
        attr: String,
        /// A sparse vector represented as a map from string dimension names to float weights. For
        /// example: `{"token_a": 0.5, "token_b": 1.2}`.
        value: BTreeMap<String, f64>,
    },
    /// Rank by BM25 (Best Matching 25, a classic full-text search scoring function, which scores
    /// documents higher when they contain more of the query's terms, especially rare ones), or by
    /// combining scores with operators like `Sum`, `Max`, and `Product`.
    Text(Box<RankByText>),
    /// Order documents by the value of an attribute, like SQL's `ORDER BY`.
    Attribute(RankByAttribute),
    /// Order by multiple attributes. Results are sorted by the first attribute, then by the second
    /// attribute for ties, and so on.
    Attributes(Vec<RankByAttribute>),
}

#[allow(deprecated)]
impl RankBy {
    /// Rank by approximate nearest neighbor (ANN) search, ordering documents by the distance between
    /// their vector and the query vector, closest first.
    pub fn ann(attr: impl Into<String>, value: impl Into<RankByAnnValue>) -> Self {
        Self::Ann {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Rank by exact nearest neighbor (kNN) search, ordering documents by the distance between their
    /// vector and the query vector, closest first. Requires `filters`.
    pub fn knn(attr: impl Into<String>, value: impl Into<RankByKnnValue>) -> Self {
        Self::Knn {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Rank by the distance between a sparse vector attribute and a sparse query vector.
    pub fn sparse_knn(
        attr: impl Into<String>,
        value: impl IntoIterator<Item = (impl Into<String>, f64)>,
    ) -> Self {
        Self::SparseKnn {
            attr: attr.into(),
            value: value.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }

    /// Rank by BM25 (Best Matching 25, a classic full-text search scoring function, which scores
    /// documents higher when they contain more of the query's terms, especially rare ones), or by
    /// combining scores with operators like `Sum`, `Max`, and `Product`.
    pub fn text(value: impl Into<RankByText>) -> Self {
        Self::Text(Box::new(value.into()))
    }

    /// Order documents by the value of an attribute, like SQL's `ORDER BY`.
    pub fn attribute(attr: impl Into<String>, order: impl Into<RankByAttributeOrder>) -> Self {
        Self::Attribute(RankByAttribute::new(attr, order))
    }

    /// Order by multiple attributes. Results are sorted by the first attribute, then by the second
    /// attribute for ties, and so on.
    pub fn attributes(value: impl IntoIterator<Item = impl Into<RankByAttribute>>) -> Self {
        Self::Attributes(value.into_iter().map(Into::into).collect())
    }

    /// Rank by how relevant an attribute is to a query string, using BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of the
    /// query's terms, especially rare ones).
    ///
    /// Shorthand for [`RankByText::bm25`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn bm25(attr: impl Into<String>, value: impl Into<RankByTextBm25Value>) -> RankByText {
        RankByText::bm25(attr, value)
    }

    /// Rank by how relevant an attribute is to a query string, using BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of the
    /// query's terms, especially rare ones).
    ///
    /// Shorthand for [`RankByText::bm25_with_params`]. It returns a [`RankByText`], which converts into
    /// a [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn bm25_with_params(
        attr: impl Into<String>,
        value: impl Into<RankByTextBm25Value>,
        params: impl Into<Bm25ClauseParams>,
    ) -> RankByText {
        RankByText::bm25_with_params(attr, value, params)
    }

    /// Sum the scores of the subqueries.
    ///
    /// Shorthand for [`RankByText::sum`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn sum(subqueries: impl IntoIterator<Item = impl Into<RankByText>>) -> RankByText {
        RankByText::sum(subqueries)
    }

    /// Use the highest score of the subqueries.
    ///
    /// Shorthand for [`RankByText::max`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn max(subqueries: impl IntoIterator<Item = impl Into<RankByText>>) -> RankByText {
        RankByText::max(subqueries)
    }

    /// Multiply a subquery's score by a non-negative weight, e.g. to boost one attribute over others.
    ///
    /// Shorthand for [`RankByText::product`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn product(weight: f64, subquery: impl Into<RankByText>) -> RankByText {
        RankByText::product(weight, subquery)
    }

    /// Map a non-negative score into `[0, 1)` with `x^exponent / (x^exponent + midpoint^exponent)`,
    /// e.g. to combine an attribute's value with scores from BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of the
    /// query's terms, especially rare ones).
    ///
    /// Shorthand for [`RankByText::saturate`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn saturate(
        subquery: impl Into<RankByText>,
        params: impl Into<SaturateParams>,
    ) -> RankByText {
        RankByText::saturate(subquery, params)
    }

    /// Map a non-negative score into `(0, 1]` with `midpoint^exponent / (x^exponent +
    /// midpoint^exponent)`, the inverse of `Saturate`, e.g. for an attribute whose higher values make
    /// documents less relevant.
    ///
    /// Shorthand for [`RankByText::decay`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn decay(subquery: impl Into<RankByText>, params: impl Into<DecayParams>) -> RankByText {
        RankByText::decay(subquery, params)
    }

    /// The distance between a subquery's value and an origin, e.g. to boost recent documents by passing
    /// the distance from now to `Decay`.
    ///
    /// Shorthand for [`RankByText::dist`]. It returns a [`RankByText`], which converts into a
    /// [`RankBy`], so it can also be combined with [`RankByText`]'s other functions.
    pub fn dist(subquery: impl Into<RankByText>, origin: impl Serialize) -> RankByText {
        RankByText::dist(subquery, origin)
    }
}

impl From<RankByText> for RankBy {
    fn from(value: RankByText) -> Self {
        Self::Text(Box::new(value))
    }
}

impl From<RankByAttribute> for RankBy {
    fn from(value: RankByAttribute) -> Self {
        Self::Attribute(value)
    }
}

impl From<Vec<RankByAttribute>> for RankBy {
    fn from(value: Vec<RankByAttribute>) -> Self {
        Self::Attributes(value)
    }
}

impl<const N: usize> From<[RankByAttribute; N]> for RankBy {
    fn from(value: [RankByAttribute; N]) -> Self {
        Self::Attributes(value.into())
    }
}

#[allow(deprecated)]
impl Serialize for RankBy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Ann { attr, value } => (attr, "ANN", value).serialize(serializer),
            Self::Knn { attr, value } => (attr, "kNN", value).serialize(serializer),
            Self::SparseKnn { attr, value } => (attr, "SparseKNN", value).serialize(serializer),
            Self::Text(value) => value.serialize(serializer),
            Self::Attribute(value) => value.serialize(serializer),
            Self::Attributes(value) => value.serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for RankBy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "ANN")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Ann { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "kNN")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Knn { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "SparseKNN")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::SparseKnn { attr, value });
            }
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Text(value));
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Attribute(value));
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Attributes(value));
        }
        Err(serde::de::Error::custom(format!("invalid RankBy: {value}")))
    }
}

impl From<RankBy> for Value {
    fn from(value: RankBy) -> Self {
        to_json(value)
    }
}

/// Rank by BM25 (Best Matching 25, a classic full-text search scoring function, which scores
/// documents higher when they contain more of the query's terms, especially rare ones), or by
/// combining scores with operators like `Sum`, `Max`, and `Product`.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`RankByText::bm25`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RankByText {
    /// Rank by how relevant an attribute is to a query string, using BM25 (Best Matching 25, a
    /// classic full-text search scoring function, which scores documents higher when they contain
    /// more of the query's terms, especially rare ones).
    Bm25 {
        /// The name of the attribute to rank by.
        attr: String,
        value: RankByTextBm25Value,
        /// Additional (optional) parameters for a single BM25 query clause.
        params: Option<Bm25ClauseParams>,
    },
    /// Sum the scores of the subqueries.
    Sum {
        /// The subqueries whose scores to sum.
        subqueries: Vec<RankByText>,
    },
    /// Use the highest score of the subqueries.
    Max {
        /// The subqueries whose highest score to use.
        subqueries: Vec<RankByText>,
    },
    /// Multiply a subquery's score by a non-negative weight, e.g. to boost one attribute over
    /// others.
    Product {
        /// The weight of the attribute.
        weight: f64,
        /// The subquery whose score to multiply.
        subquery: Box<RankByText>,
    },
    /// Score documents that match a filter 1, and other documents 0, e.g. to boost documents that
    /// match some criteria.
    Filter(Box<Filter>),
    /// Use the value of a numeric attribute as the score. Attributes of signed types, like `int`
    /// and `float`, must be wrapped in `Max` or `Saturate`, since scores can't be negative.
    Attribute {
        /// The name of the attribute to rank by.
        attr: String,
    },
    /// Map a non-negative score into `[0, 1)` with `x^exponent / (x^exponent + midpoint^exponent)`,
    /// e.g. to combine an attribute's value with scores from BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of
    /// the query's terms, especially rare ones).
    Saturate {
        /// The subquery whose score to saturate.
        subquery: Box<RankByText>,
        /// Additional parameters for the Saturate operator.
        params: SaturateParams,
    },
    /// Map a non-negative score into `(0, 1]` with `midpoint^exponent / (x^exponent +
    /// midpoint^exponent)`, the inverse of `Saturate`, e.g. for an attribute whose higher values
    /// make documents less relevant.
    Decay {
        /// The subquery whose score to decay.
        subquery: Box<RankByText>,
        /// Additional parameters for the Decay operator.
        params: DecayParams,
    },
    /// The distance between a subquery's value and an origin, e.g. to boost recent documents by
    /// passing the distance from now to `Decay`.
    Dist {
        /// The subquery whose distance from `origin` to compute, e.g. an `Attribute`.
        subquery: Box<RankByText>,
        /// The value to compute the distance from, e.g. a timestamp.
        origin: Value,
    },
}

#[allow(deprecated)]
impl RankByText {
    /// Rank by how relevant an attribute is to a query string, using BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of the
    /// query's terms, especially rare ones).
    pub fn bm25(attr: impl Into<String>, value: impl Into<RankByTextBm25Value>) -> Self {
        Self::Bm25 {
            attr: attr.into(),
            value: value.into(),
            params: None,
        }
    }

    /// Rank by how relevant an attribute is to a query string, using BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of the
    /// query's terms, especially rare ones).
    ///
    /// Like [`RankByText::bm25`], with `params`.
    pub fn bm25_with_params(
        attr: impl Into<String>,
        value: impl Into<RankByTextBm25Value>,
        params: impl Into<Bm25ClauseParams>,
    ) -> Self {
        Self::Bm25 {
            attr: attr.into(),
            value: value.into(),
            params: Some(params.into()),
        }
    }

    /// Sum the scores of the subqueries.
    pub fn sum(subqueries: impl IntoIterator<Item = impl Into<RankByText>>) -> Self {
        Self::Sum {
            subqueries: subqueries.into_iter().map(Into::into).collect(),
        }
    }

    /// Use the highest score of the subqueries.
    pub fn max(subqueries: impl IntoIterator<Item = impl Into<RankByText>>) -> Self {
        Self::Max {
            subqueries: subqueries.into_iter().map(Into::into).collect(),
        }
    }

    /// Multiply a subquery's score by a non-negative weight, e.g. to boost one attribute over others.
    pub fn product(weight: f64, subquery: impl Into<RankByText>) -> Self {
        Self::Product {
            weight,
            subquery: Box::new(subquery.into()),
        }
    }

    /// Use the value of a numeric attribute as the score. Attributes of signed types, like `int` and
    /// `float`, must be wrapped in `Max` or `Saturate`, since scores can't be negative.
    pub fn attribute(attr: impl Into<String>) -> Self {
        Self::Attribute { attr: attr.into() }
    }

    /// Map a non-negative score into `[0, 1)` with `x^exponent / (x^exponent + midpoint^exponent)`,
    /// e.g. to combine an attribute's value with scores from BM25 (Best Matching 25, a classic
    /// full-text search scoring function, which scores documents higher when they contain more of the
    /// query's terms, especially rare ones).
    pub fn saturate(subquery: impl Into<RankByText>, params: impl Into<SaturateParams>) -> Self {
        Self::Saturate {
            subquery: Box::new(subquery.into()),
            params: params.into(),
        }
    }

    /// Map a non-negative score into `(0, 1]` with `midpoint^exponent / (x^exponent +
    /// midpoint^exponent)`, the inverse of `Saturate`, e.g. for an attribute whose higher values make
    /// documents less relevant.
    pub fn decay(subquery: impl Into<RankByText>, params: impl Into<DecayParams>) -> Self {
        Self::Decay {
            subquery: Box::new(subquery.into()),
            params: params.into(),
        }
    }

    /// The distance between a subquery's value and an origin, e.g. to boost recent documents by passing
    /// the distance from now to `Decay`.
    pub fn dist(subquery: impl Into<RankByText>, origin: impl Serialize) -> Self {
        Self::Dist {
            subquery: Box::new(subquery.into()),
            origin: to_json(origin),
        }
    }

    /// Score documents that match a filter 1, and other documents 0, e.g. to boost documents that match
    /// some criteria.
    pub fn filter(value: impl Into<Filter>) -> Self {
        Self::Filter(Box::new(value.into()))
    }
}

impl From<Filter> for RankByText {
    fn from(value: Filter) -> Self {
        Self::Filter(Box::new(value))
    }
}

#[allow(deprecated)]
impl Serialize for RankByText {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bm25 {
                attr,
                value,
                params: None,
            } => (attr, "BM25", value).serialize(serializer),
            Self::Bm25 {
                attr,
                value,
                params: Some(params),
            } => (attr, "BM25", value, params).serialize(serializer),
            Self::Sum { subqueries } => ("Sum", subqueries).serialize(serializer),
            Self::Max { subqueries } => ("Max", subqueries).serialize(serializer),
            Self::Product { weight, subquery } => {
                ("Product", weight, subquery).serialize(serializer)
            }
            Self::Filter(value) => value.serialize(serializer),
            Self::Attribute { attr } => ("Attribute", attr).serialize(serializer),
            Self::Saturate { subquery, params } => {
                ("Saturate", subquery, params).serialize(serializer)
            }
            Self::Decay { subquery, params } => ("Decay", subquery, params).serialize(serializer),
            Self::Dist { subquery, origin } => ("Dist", subquery, origin).serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for RankByText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(elements) = tuple_elements(&value, 4, &[(1, "BM25")]) {
            if let (Some(attr), Some(value), Some(params)) = (
                decode_element(&elements[0]),
                decode_element(&elements[2]),
                decode_element(&elements[3]),
            ) {
                return Ok(Self::Bm25 {
                    attr,
                    value,
                    params: Some(params),
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "BM25")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Bm25 {
                    attr,
                    value,
                    params: None,
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Sum")]) {
            if let Some(subqueries) = decode_element(&elements[1]) {
                return Ok(Self::Sum { subqueries });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Max")]) {
            if let Some(subqueries) = decode_element(&elements[1]) {
                return Ok(Self::Max { subqueries });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(0, "Product")]) {
            if let (Some(weight), Some(subquery)) =
                (decode_element(&elements[1]), decode_element(&elements[2]))
            {
                return Ok(Self::Product { weight, subquery });
            }
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Filter(value));
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Attribute")]) {
            if let Some(attr) = decode_element(&elements[1]) {
                return Ok(Self::Attribute { attr });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(0, "Saturate")]) {
            if let (Some(subquery), Some(params)) =
                (decode_element(&elements[1]), decode_element(&elements[2]))
            {
                return Ok(Self::Saturate { subquery, params });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(0, "Decay")]) {
            if let (Some(subquery), Some(params)) =
                (decode_element(&elements[1]), decode_element(&elements[2]))
            {
                return Ok(Self::Decay { subquery, params });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(0, "Dist")]) {
            if let (Some(subquery), Some(origin)) =
                (decode_element(&elements[1]), decode_element(&elements[2]))
            {
                return Ok(Self::Dist { subquery, origin });
            }
        }
        Err(serde::de::Error::custom(format!(
            "invalid RankByText: {value}"
        )))
    }
}

impl From<RankByText> for Value {
    fn from(value: RankByText) -> Self {
        to_json(value)
    }
}

/// Additional (optional) parameters for a single BM25 query clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct Bm25ClauseParams {
    /// Whether to treat the last token in the query input as a literal prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_as_prefix: Option<bool>,
}

impl Bm25ClauseParams {
    /// Creates a new [`Bm25ClauseParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether to treat the last token in the query input as a literal prefix.
    pub fn last_as_prefix(mut self, last_as_prefix: bool) -> Self {
        self.last_as_prefix = Some(last_as_prefix);
        self
    }
}

/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`Filter::eq`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Filter {
    /// Exact match for attribute value. If `null`, matches documents missing the attribute.
    Eq {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Inverse of `Eq`. If value is `null`, matches documents with the attribute.
    NotEq {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Matches any attribute value contained in the provided list.
    In {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Vec<Value>,
    },
    /// Inverse of `In`, matches any attributes values not contained in the provided list.
    NotIn {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Vec<Value>,
    },
    /// Checks whether the selected array attribute contains the provided value.
    Contains {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Inverse of Contains
    NotContains {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Checks whether the selected array attribute contains any of the values provided
    /// (intersection filter).
    ContainsAny {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Vec<Value>,
    },
    /// Inverse of ContainsAny.
    NotContainsAny {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Vec<Value>,
    },
    /// For ints, this is a numeric less-than. For strings, lexicographic less-than. For datetimes,
    /// numeric less-than on millisecond representation.
    Lt {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// For ints, this is a numeric less-than-or-equal. For strings, lexicographic
    /// less-than-or-equal. For datetimes, numeric less-than-or-equal on millisecond representation.
    Lte {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// For ints, this is a numeric greater-than. For strings, lexicographic greater-than. For
    /// datetimes, numeric greater-than on millisecond representation.
    Gt {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// For ints, this is a numeric greater-than-or-equal. For strings, lexicographic
    /// greater-than-or-equal. For datetimes, numeric greater-than-or-equal on millisecond
    /// representation.
    Gte {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Checks whether any element of an array attribute is less than the provided value, using the
    /// same rules as `Lt`.
    AnyLt {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Checks whether any element of an array attribute is less than or equal to the provided
    /// value, using the same rules as `Lte`.
    AnyLte {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Checks whether any element of an array attribute is greater than the provided value, using
    /// the same rules as `Gt`.
    AnyGt {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Checks whether any element of an array attribute is greater than or equal to the provided
    /// value, using the same rules as `Gte`.
    AnyGte {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: Value,
    },
    /// Unix-style glob match against string values. The full syntax is described in the Rust
    /// `globset` crate documentation. Glob patterns with a concrete prefix like "foo*" internally
    /// compile to efficient range queries.
    Glob {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: String,
    },
    /// Inverse of `Glob`, Unix-style glob filters against string attributes values. The full syntax
    /// is described in the Rust `globset` crate documentation.
    NotGlob {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: String,
    },
    /// Case insensitive version of `Glob`.
    IGlob {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: String,
    },
    /// Case insensitive version of `NotGlob`.
    NotIGlob {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The value of the filter.
        value: String,
    },
    /// Regular expression match against string values. Requires the regex schema attribute to be
    /// enabled before use.
    Regex {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The regular expression to match against.
        value: String,
    },
    /// Fuzzy substring match against string values. Requires the fuzzy schema attribute to be
    /// enabled before use.
    Fuzzy {
        /// The name of the attribute to apply the filter to.
        attr: String,
        /// The string to fuzzy-match against.
        value: String,
        /// Additional parameters for the Fuzzy filter.
        params: FuzzyParams,
    },
    /// Matches if all tokens in the input string are present in the attributes value. Requires that
    /// the attribute is configured for full-text search.
    ContainsAllTokens {
        /// The name of the attribute to apply the filter to.
        attr: String,
        value: FilterContainsAllTokensValue,
        /// Additional (optional) parameters for the ContainsAllTokens filter.
        params: Option<ContainsAllTokensFilterParams>,
    },
    /// Matches if any of the tokens in the input string are present in the attribute value.
    /// Requires that the attribute is configured for full-text search.
    ContainsAnyToken {
        /// The name of the attribute to apply the filter to.
        attr: String,
        value: FilterContainsAnyTokenValue,
        /// Additional (optional) parameters for the ContainsAnyToken filter.
        params: Option<ContainsAnyTokenFilterParams>,
    },
    /// Matches if all the tokens in the input string are present in the attribute value, in the
    /// correct order (i.e., as a phrase). Requires that the attribute is configured for full-text
    /// search.
    ContainsTokenSequence {
        /// The name of the attribute to apply the filter to.
        attr: String,
        value: FilterContainsTokenSequenceValue,
    },
    /// Matches documents that don't match the filter.
    Not {
        /// The filter to negate.
        filter: Box<Filter>,
    },
    /// Matches documents that match all of the filters.
    And {
        /// The filters to combine.
        filters: Vec<Filter>,
    },
    /// Matches documents that match at least one of the filters.
    Or {
        /// The filters to combine.
        filters: Vec<Filter>,
    },
}

#[allow(deprecated)]
impl Filter {
    /// Exact match for attribute value. If `null`, matches documents missing the attribute.
    pub fn eq(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::Eq {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Inverse of `Eq`. If value is `null`, matches documents with the attribute.
    pub fn not_eq(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::NotEq {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Matches any attribute value contained in the provided list.
    pub fn in_(attr: impl Into<String>, value: impl IntoIterator<Item = impl Serialize>) -> Self {
        Self::In {
            attr: attr.into(),
            value: value.into_iter().map(to_json).collect(),
        }
    }

    /// Inverse of `In`, matches any attributes values not contained in the provided list.
    pub fn not_in(
        attr: impl Into<String>,
        value: impl IntoIterator<Item = impl Serialize>,
    ) -> Self {
        Self::NotIn {
            attr: attr.into(),
            value: value.into_iter().map(to_json).collect(),
        }
    }

    /// Checks whether the selected array attribute contains the provided value.
    pub fn contains(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::Contains {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Inverse of Contains
    pub fn not_contains(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::NotContains {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Checks whether the selected array attribute contains any of the values provided (intersection
    /// filter).
    pub fn contains_any(
        attr: impl Into<String>,
        value: impl IntoIterator<Item = impl Serialize>,
    ) -> Self {
        Self::ContainsAny {
            attr: attr.into(),
            value: value.into_iter().map(to_json).collect(),
        }
    }

    /// Inverse of ContainsAny.
    pub fn not_contains_any(
        attr: impl Into<String>,
        value: impl IntoIterator<Item = impl Serialize>,
    ) -> Self {
        Self::NotContainsAny {
            attr: attr.into(),
            value: value.into_iter().map(to_json).collect(),
        }
    }

    /// For ints, this is a numeric less-than. For strings, lexicographic less-than. For datetimes,
    /// numeric less-than on millisecond representation.
    pub fn lt(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::Lt {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// For ints, this is a numeric less-than-or-equal. For strings, lexicographic less-than-or-equal.
    /// For datetimes, numeric less-than-or-equal on millisecond representation.
    pub fn lte(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::Lte {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// For ints, this is a numeric greater-than. For strings, lexicographic greater-than. For
    /// datetimes, numeric greater-than on millisecond representation.
    pub fn gt(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::Gt {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// For ints, this is a numeric greater-than-or-equal. For strings, lexicographic
    /// greater-than-or-equal. For datetimes, numeric greater-than-or-equal on millisecond
    /// representation.
    pub fn gte(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::Gte {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Checks whether any element of an array attribute is less than the provided value, using the same
    /// rules as `Lt`.
    pub fn any_lt(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::AnyLt {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Checks whether any element of an array attribute is less than or equal to the provided value,
    /// using the same rules as `Lte`.
    pub fn any_lte(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::AnyLte {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Checks whether any element of an array attribute is greater than the provided value, using the
    /// same rules as `Gt`.
    pub fn any_gt(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::AnyGt {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Checks whether any element of an array attribute is greater than or equal to the provided value,
    /// using the same rules as `Gte`.
    pub fn any_gte(attr: impl Into<String>, value: impl Serialize) -> Self {
        Self::AnyGte {
            attr: attr.into(),
            value: to_json(value),
        }
    }

    /// Unix-style glob match against string values. The full syntax is described in the Rust `globset`
    /// crate documentation. Glob patterns with a concrete prefix like "foo*" internally compile to
    /// efficient range queries.
    pub fn glob(attr: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Glob {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Inverse of `Glob`, Unix-style glob filters against string attributes values. The full syntax is
    /// described in the Rust `globset` crate documentation.
    pub fn not_glob(attr: impl Into<String>, value: impl Into<String>) -> Self {
        Self::NotGlob {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Case insensitive version of `Glob`.
    pub fn iglob(attr: impl Into<String>, value: impl Into<String>) -> Self {
        Self::IGlob {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Case insensitive version of `NotGlob`.
    pub fn not_iglob(attr: impl Into<String>, value: impl Into<String>) -> Self {
        Self::NotIGlob {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Regular expression match against string values. Requires the regex schema attribute to be
    /// enabled before use.
    pub fn regex(attr: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Regex {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Fuzzy substring match against string values. Requires the fuzzy schema attribute to be enabled
    /// before use.
    pub fn fuzzy(
        attr: impl Into<String>,
        value: impl Into<String>,
        params: impl Into<FuzzyParams>,
    ) -> Self {
        Self::Fuzzy {
            attr: attr.into(),
            value: value.into(),
            params: params.into(),
        }
    }

    /// Matches if all tokens in the input string are present in the attributes value. Requires that the
    /// attribute is configured for full-text search.
    pub fn contains_all_tokens(
        attr: impl Into<String>,
        value: impl Into<FilterContainsAllTokensValue>,
    ) -> Self {
        Self::ContainsAllTokens {
            attr: attr.into(),
            value: value.into(),
            params: None,
        }
    }

    /// Matches if all tokens in the input string are present in the attributes value. Requires that the
    /// attribute is configured for full-text search.
    ///
    /// Like [`Filter::contains_all_tokens`], with `params`.
    pub fn contains_all_tokens_with_params(
        attr: impl Into<String>,
        value: impl Into<FilterContainsAllTokensValue>,
        params: impl Into<ContainsAllTokensFilterParams>,
    ) -> Self {
        Self::ContainsAllTokens {
            attr: attr.into(),
            value: value.into(),
            params: Some(params.into()),
        }
    }

    /// Matches if any of the tokens in the input string are present in the attribute value. Requires
    /// that the attribute is configured for full-text search.
    pub fn contains_any_token(
        attr: impl Into<String>,
        value: impl Into<FilterContainsAnyTokenValue>,
    ) -> Self {
        Self::ContainsAnyToken {
            attr: attr.into(),
            value: value.into(),
            params: None,
        }
    }

    /// Matches if any of the tokens in the input string are present in the attribute value. Requires
    /// that the attribute is configured for full-text search.
    ///
    /// Like [`Filter::contains_any_token`], with `params`.
    pub fn contains_any_token_with_params(
        attr: impl Into<String>,
        value: impl Into<FilterContainsAnyTokenValue>,
        params: impl Into<ContainsAnyTokenFilterParams>,
    ) -> Self {
        Self::ContainsAnyToken {
            attr: attr.into(),
            value: value.into(),
            params: Some(params.into()),
        }
    }

    /// Matches if all the tokens in the input string are present in the attribute value, in the correct
    /// order (i.e., as a phrase). Requires that the attribute is configured for full-text search.
    pub fn contains_token_sequence(
        attr: impl Into<String>,
        value: impl Into<FilterContainsTokenSequenceValue>,
    ) -> Self {
        Self::ContainsTokenSequence {
            attr: attr.into(),
            value: value.into(),
        }
    }

    /// Matches documents that don't match the filter.
    pub fn not(filter: impl Into<Filter>) -> Self {
        Self::Not {
            filter: Box::new(filter.into()),
        }
    }

    /// Matches documents that match all of the filters.
    pub fn and(filters: impl IntoIterator<Item = impl Into<Filter>>) -> Self {
        Self::And {
            filters: filters.into_iter().map(Into::into).collect(),
        }
    }

    /// Matches documents that match at least one of the filters.
    pub fn or(filters: impl IntoIterator<Item = impl Into<Filter>>) -> Self {
        Self::Or {
            filters: filters.into_iter().map(Into::into).collect(),
        }
    }
}

#[allow(deprecated)]
impl Serialize for Filter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Eq { attr, value } => (attr, "Eq", value).serialize(serializer),
            Self::NotEq { attr, value } => (attr, "NotEq", value).serialize(serializer),
            Self::In { attr, value } => (attr, "In", value).serialize(serializer),
            Self::NotIn { attr, value } => (attr, "NotIn", value).serialize(serializer),
            Self::Contains { attr, value } => (attr, "Contains", value).serialize(serializer),
            Self::NotContains { attr, value } => (attr, "NotContains", value).serialize(serializer),
            Self::ContainsAny { attr, value } => (attr, "ContainsAny", value).serialize(serializer),
            Self::NotContainsAny { attr, value } => {
                (attr, "NotContainsAny", value).serialize(serializer)
            }
            Self::Lt { attr, value } => (attr, "Lt", value).serialize(serializer),
            Self::Lte { attr, value } => (attr, "Lte", value).serialize(serializer),
            Self::Gt { attr, value } => (attr, "Gt", value).serialize(serializer),
            Self::Gte { attr, value } => (attr, "Gte", value).serialize(serializer),
            Self::AnyLt { attr, value } => (attr, "AnyLt", value).serialize(serializer),
            Self::AnyLte { attr, value } => (attr, "AnyLte", value).serialize(serializer),
            Self::AnyGt { attr, value } => (attr, "AnyGt", value).serialize(serializer),
            Self::AnyGte { attr, value } => (attr, "AnyGte", value).serialize(serializer),
            Self::Glob { attr, value } => (attr, "Glob", value).serialize(serializer),
            Self::NotGlob { attr, value } => (attr, "NotGlob", value).serialize(serializer),
            Self::IGlob { attr, value } => (attr, "IGlob", value).serialize(serializer),
            Self::NotIGlob { attr, value } => (attr, "NotIGlob", value).serialize(serializer),
            Self::Regex { attr, value } => (attr, "Regex", value).serialize(serializer),
            Self::Fuzzy {
                attr,
                value,
                params,
            } => (attr, "Fuzzy", value, params).serialize(serializer),
            Self::ContainsAllTokens {
                attr,
                value,
                params: None,
            } => (attr, "ContainsAllTokens", value).serialize(serializer),
            Self::ContainsAllTokens {
                attr,
                value,
                params: Some(params),
            } => (attr, "ContainsAllTokens", value, params).serialize(serializer),
            Self::ContainsAnyToken {
                attr,
                value,
                params: None,
            } => (attr, "ContainsAnyToken", value).serialize(serializer),
            Self::ContainsAnyToken {
                attr,
                value,
                params: Some(params),
            } => (attr, "ContainsAnyToken", value, params).serialize(serializer),
            Self::ContainsTokenSequence { attr, value } => {
                (attr, "ContainsTokenSequence", value).serialize(serializer)
            }
            Self::Not { filter } => ("Not", filter).serialize(serializer),
            Self::And { filters } => ("And", filters).serialize(serializer),
            Self::Or { filters } => ("Or", filters).serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for Filter {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Eq")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Eq { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "NotEq")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::NotEq { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "In")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::In { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "NotIn")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::NotIn { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Contains")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Contains { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "NotContains")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::NotContains { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "ContainsAny")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::ContainsAny { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "NotContainsAny")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::NotContainsAny { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Lt")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Lt { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Lte")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Lte { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Gt")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Gt { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Gte")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Gte { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "AnyLt")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::AnyLt { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "AnyLte")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::AnyLte { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "AnyGt")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::AnyGt { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "AnyGte")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::AnyGte { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Glob")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Glob { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "NotGlob")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::NotGlob { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "IGlob")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::IGlob { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "NotIGlob")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::NotIGlob { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "Regex")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::Regex { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 4, &[(1, "Fuzzy")]) {
            if let (Some(attr), Some(value), Some(params)) = (
                decode_element(&elements[0]),
                decode_element(&elements[2]),
                decode_element(&elements[3]),
            ) {
                return Ok(Self::Fuzzy {
                    attr,
                    value,
                    params,
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 4, &[(1, "ContainsAllTokens")]) {
            if let (Some(attr), Some(value), Some(params)) = (
                decode_element(&elements[0]),
                decode_element(&elements[2]),
                decode_element(&elements[3]),
            ) {
                return Ok(Self::ContainsAllTokens {
                    attr,
                    value,
                    params: Some(params),
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "ContainsAllTokens")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::ContainsAllTokens {
                    attr,
                    value,
                    params: None,
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 4, &[(1, "ContainsAnyToken")]) {
            if let (Some(attr), Some(value), Some(params)) = (
                decode_element(&elements[0]),
                decode_element(&elements[2]),
                decode_element(&elements[3]),
            ) {
                return Ok(Self::ContainsAnyToken {
                    attr,
                    value,
                    params: Some(params),
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "ContainsAnyToken")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::ContainsAnyToken {
                    attr,
                    value,
                    params: None,
                });
            }
        }
        if let Some(elements) = tuple_elements(&value, 3, &[(1, "ContainsTokenSequence")]) {
            if let (Some(attr), Some(value)) =
                (decode_element(&elements[0]), decode_element(&elements[2]))
            {
                return Ok(Self::ContainsTokenSequence { attr, value });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Not")]) {
            if let Some(filter) = decode_element(&elements[1]) {
                return Ok(Self::Not { filter });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "And")]) {
            if let Some(filters) = decode_element(&elements[1]) {
                return Ok(Self::And { filters });
            }
        }
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "Or")]) {
            if let Some(filters) = decode_element(&elements[1]) {
                return Ok(Self::Or { filters });
            }
        }
        Err(serde::de::Error::custom(format!("invalid Filter: {value}")))
    }
}

impl From<Filter> for Value {
    fn from(value: Filter) -> Self {
        to_json(value)
    }
}

/// Additional parameters for the Fuzzy filter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FuzzyParams {
    /// Maximum edit distance allowed at each query length. Queries shorter than the first threshold
    /// return no matches.
    pub max_edit_distance: Vec<FuzzyMaxEditDistance>,
    /// Whether searching with Fuzzy filter is case-sensitive. Defaults to `true` (i.e.
    /// case-sensitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
}

impl FuzzyParams {
    /// Creates a new [`FuzzyParams`] from its required fields.
    pub fn new(
        max_edit_distance: impl IntoIterator<Item = impl Into<FuzzyMaxEditDistance>>,
    ) -> Self {
        Self {
            max_edit_distance: max_edit_distance.into_iter().map(Into::into).collect(),
            case_sensitive: Default::default(),
        }
    }

    /// Whether searching with Fuzzy filter is case-sensitive. Defaults to `true` (i.e. case-sensitive).
    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = Some(case_sensitive);
        self
    }
}

/// An edit distance threshold for the Fuzzy filter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FuzzyMaxEditDistance {
    /// The maximum edit distance to allow.
    pub distance: u64,
    /// Minimum number of characters in a query where this distance applies. Must be at least 3 ·
    /// (distance + 1).
    pub min_query_chars: u64,
}

impl FuzzyMaxEditDistance {
    /// Creates a new [`FuzzyMaxEditDistance`] from its required fields.
    pub fn new(min_query_chars: u64, distance: u64) -> Self {
        Self {
            distance,
            min_query_chars,
        }
    }
}

/// Additional (optional) parameters for the ContainsAllTokens filter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct ContainsAllTokensFilterParams {
    /// Whether to treat the last token in the query input as a literal prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_as_prefix: Option<bool>,
}

impl ContainsAllTokensFilterParams {
    /// Creates a new [`ContainsAllTokensFilterParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether to treat the last token in the query input as a literal prefix.
    pub fn last_as_prefix(mut self, last_as_prefix: bool) -> Self {
        self.last_as_prefix = Some(last_as_prefix);
        self
    }
}

/// Additional (optional) parameters for the ContainsAnyToken filter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct ContainsAnyTokenFilterParams {
    /// Whether to treat the last token in the query input as a literal prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_as_prefix: Option<bool>,
}

impl ContainsAnyTokenFilterParams {
    /// Creates a new [`ContainsAnyTokenFilterParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether to treat the last token in the query input as a literal prefix.
    pub fn last_as_prefix(mut self, last_as_prefix: bool) -> Self {
        self.last_as_prefix = Some(last_as_prefix);
        self
    }
}

/// One of the shapes `ContainsAllTokens` accepts for its `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum FilterContainsAllTokensValue {
    /// The string to search for.
    String(String),
    /// The tokens to search for.
    Array(Vec<String>),
}

impl From<String> for FilterContainsAllTokensValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for FilterContainsAllTokensValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Vec<String>> for FilterContainsAllTokensValue {
    fn from(value: Vec<String>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<&str>> for FilterContainsAllTokensValue {
    fn from(value: Vec<&str>) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<&[&str]> for FilterContainsAllTokensValue {
    fn from(value: &[&str]) -> Self {
        Self::Array(value.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for FilterContainsAllTokensValue {
    fn from(value: [&str; N]) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<FilterContainsAnyTokenValue> for FilterContainsAllTokensValue {
    fn from(value: FilterContainsAnyTokenValue) -> Self {
        match value {
            FilterContainsAnyTokenValue::String(value) => Self::String(value),
            FilterContainsAnyTokenValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<FilterContainsTokenSequenceValue> for FilterContainsAllTokensValue {
    fn from(value: FilterContainsTokenSequenceValue) -> Self {
        match value {
            FilterContainsTokenSequenceValue::String(value) => Self::String(value),
            FilterContainsTokenSequenceValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<RankByTextBm25Value> for FilterContainsAllTokensValue {
    fn from(value: RankByTextBm25Value) -> Self {
        match value {
            RankByTextBm25Value::String(value) => Self::String(value),
            RankByTextBm25Value::Array(value) => Self::Array(value),
        }
    }
}

/// One of the shapes `ContainsAnyToken` accepts for its `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum FilterContainsAnyTokenValue {
    /// The string to search for.
    String(String),
    /// The tokens to search for.
    Array(Vec<String>),
}

impl From<String> for FilterContainsAnyTokenValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for FilterContainsAnyTokenValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Vec<String>> for FilterContainsAnyTokenValue {
    fn from(value: Vec<String>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<&str>> for FilterContainsAnyTokenValue {
    fn from(value: Vec<&str>) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<&[&str]> for FilterContainsAnyTokenValue {
    fn from(value: &[&str]) -> Self {
        Self::Array(value.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for FilterContainsAnyTokenValue {
    fn from(value: [&str; N]) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<FilterContainsAllTokensValue> for FilterContainsAnyTokenValue {
    fn from(value: FilterContainsAllTokensValue) -> Self {
        match value {
            FilterContainsAllTokensValue::String(value) => Self::String(value),
            FilterContainsAllTokensValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<FilterContainsTokenSequenceValue> for FilterContainsAnyTokenValue {
    fn from(value: FilterContainsTokenSequenceValue) -> Self {
        match value {
            FilterContainsTokenSequenceValue::String(value) => Self::String(value),
            FilterContainsTokenSequenceValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<RankByTextBm25Value> for FilterContainsAnyTokenValue {
    fn from(value: RankByTextBm25Value) -> Self {
        match value {
            RankByTextBm25Value::String(value) => Self::String(value),
            RankByTextBm25Value::Array(value) => Self::Array(value),
        }
    }
}

/// One of the shapes `ContainsTokenSequence` accepts for its `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum FilterContainsTokenSequenceValue {
    /// The string to search for.
    String(String),
    /// The tokens to search for.
    Array(Vec<String>),
}

impl From<String> for FilterContainsTokenSequenceValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for FilterContainsTokenSequenceValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Vec<String>> for FilterContainsTokenSequenceValue {
    fn from(value: Vec<String>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<&str>> for FilterContainsTokenSequenceValue {
    fn from(value: Vec<&str>) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<&[&str]> for FilterContainsTokenSequenceValue {
    fn from(value: &[&str]) -> Self {
        Self::Array(value.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for FilterContainsTokenSequenceValue {
    fn from(value: [&str; N]) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<FilterContainsAllTokensValue> for FilterContainsTokenSequenceValue {
    fn from(value: FilterContainsAllTokensValue) -> Self {
        match value {
            FilterContainsAllTokensValue::String(value) => Self::String(value),
            FilterContainsAllTokensValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<FilterContainsAnyTokenValue> for FilterContainsTokenSequenceValue {
    fn from(value: FilterContainsAnyTokenValue) -> Self {
        match value {
            FilterContainsAnyTokenValue::String(value) => Self::String(value),
            FilterContainsAnyTokenValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<RankByTextBm25Value> for FilterContainsTokenSequenceValue {
    fn from(value: RankByTextBm25Value) -> Self {
        match value {
            RankByTextBm25Value::String(value) => Self::String(value),
            RankByTextBm25Value::Array(value) => Self::Array(value),
        }
    }
}

/// Additional parameters for the Saturate operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct SaturateParams {
    /// An exponent that helps further control the shape of the Saturate function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exponent: Option<f64>,
    /// The midpoint of the Saturate operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midpoint: Option<Value>,
}

impl SaturateParams {
    /// Creates a new [`SaturateParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// An exponent that helps further control the shape of the Saturate function.
    pub fn exponent(mut self, exponent: f64) -> Self {
        self.exponent = Some(exponent);
        self
    }

    /// The midpoint of the Saturate operator.
    pub fn midpoint(mut self, midpoint: impl Serialize) -> Self {
        self.midpoint = Some(to_json(midpoint));
        self
    }
}

/// Additional parameters for the Decay operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct DecayParams {
    /// An exponent that helps further control the shape of the Decay function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exponent: Option<f64>,
    /// The midpoint of the Decay operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midpoint: Option<Value>,
}

impl DecayParams {
    /// Creates a new [`DecayParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// An exponent that helps further control the shape of the Decay function.
    pub fn exponent(mut self, exponent: f64) -> Self {
        self.exponent = Some(exponent);
        self
    }

    /// The midpoint of the Decay operator.
    pub fn midpoint(mut self, midpoint: impl Serialize) -> Self {
        self.midpoint = Some(to_json(midpoint));
        self
    }
}

/// One of the shapes `BM25` accepts for its `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum RankByTextBm25Value {
    /// The string to search for.
    String(String),
    /// The tokens to search for.
    Array(Vec<String>),
}

impl From<String> for RankByTextBm25Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for RankByTextBm25Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Vec<String>> for RankByTextBm25Value {
    fn from(value: Vec<String>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<&str>> for RankByTextBm25Value {
    fn from(value: Vec<&str>) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<&[&str]> for RankByTextBm25Value {
    fn from(value: &[&str]) -> Self {
        Self::Array(value.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for RankByTextBm25Value {
    fn from(value: [&str; N]) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<FilterContainsAllTokensValue> for RankByTextBm25Value {
    fn from(value: FilterContainsAllTokensValue) -> Self {
        match value {
            FilterContainsAllTokensValue::String(value) => Self::String(value),
            FilterContainsAllTokensValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<FilterContainsAnyTokenValue> for RankByTextBm25Value {
    fn from(value: FilterContainsAnyTokenValue) -> Self {
        match value {
            FilterContainsAnyTokenValue::String(value) => Self::String(value),
            FilterContainsAnyTokenValue::Array(value) => Self::Array(value),
        }
    }
}

impl From<FilterContainsTokenSequenceValue> for RankByTextBm25Value {
    fn from(value: FilterContainsTokenSequenceValue) -> Self {
        match value {
            FilterContainsTokenSequenceValue::String(value) => Self::String(value),
            FilterContainsTokenSequenceValue::Array(value) => Self::Array(value),
        }
    }
}

/// Order documents by the value of an attribute, like SQL's `ORDER BY`.
///
/// Serialized as a JSON array (tuple).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RankByAttribute {
    /// The name of the attribute to rank by.
    pub attr: String,
    /// The direction to order in.
    pub order: RankByAttributeOrder,
}

impl RankByAttribute {
    /// Creates a new [`RankByAttribute`].
    pub fn new(attr: impl Into<String>, order: impl Into<RankByAttributeOrder>) -> Self {
        Self {
            attr: attr.into(),
            order: order.into(),
        }
    }
}

impl<A: Into<String>, B: Into<RankByAttributeOrder>> From<(A, B)> for RankByAttribute {
    fn from(value: (A, B)) -> Self {
        Self {
            attr: value.0.into(),
            order: value.1.into(),
        }
    }
}

impl Serialize for RankByAttribute {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (&self.attr, &self.order).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RankByAttribute {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(elements) = tuple_elements(&value, 2, &[]) {
            if let (Some(attr), Some(order)) =
                (decode_element(&elements[0]), decode_element(&elements[1]))
            {
                return Ok(Self { attr, order });
            }
        }
        Err(serde::de::Error::custom(format!(
            "invalid RankByAttribute: {value}"
        )))
    }
}

impl From<RankByAttribute> for Value {
    fn from(value: RankByAttribute) -> Self {
        to_json(value)
    }
}

/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`RankByAttributeOrder::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RankByAttributeOrder {
    /// Ascending order.
    #[serde(rename = "asc")]
    Asc,
    /// Descending order.
    #[serde(rename = "desc")]
    Desc,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl RankByAttributeOrder {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for RankByAttributeOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for RankByAttributeOrder {
    fn from(value: &str) -> Self {
        match value {
            "asc" => Self::Asc,
            "desc" => Self::Desc,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for RankByAttributeOrder {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// One of the shapes `ANN` accepts for its `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum RankByAnnValue {
    /// The query vector.
    Vector(Vec<f32>),
    /// The query vectors, e.g. one per query token.
    Multi(Vec<Vec<f32>>),
    /// An expression that computes a value.
    Expr(Box<Expr>),
}

impl From<Vec<f32>> for RankByAnnValue {
    fn from(value: Vec<f32>) -> Self {
        Self::Vector(value)
    }
}

impl From<&[f32]> for RankByAnnValue {
    fn from(value: &[f32]) -> Self {
        Self::Vector(value.to_vec())
    }
}

impl<const N: usize> From<[f32; N]> for RankByAnnValue {
    fn from(value: [f32; N]) -> Self {
        Self::Vector(value.into())
    }
}

impl From<Vec<Vec<f32>>> for RankByAnnValue {
    fn from(value: Vec<Vec<f32>>) -> Self {
        Self::Multi(value)
    }
}

impl From<&[Vec<f32>]> for RankByAnnValue {
    fn from(value: &[Vec<f32>]) -> Self {
        Self::Multi(value.to_vec())
    }
}

impl<const N: usize> From<[Vec<f32>; N]> for RankByAnnValue {
    fn from(value: [Vec<f32>; N]) -> Self {
        Self::Multi(value.into())
    }
}

impl<const M: usize, const N: usize> From<[[f32; M]; N]> for RankByAnnValue {
    fn from(value: [[f32; M]; N]) -> Self {
        Self::Multi(value.into_iter().map(Vec::from).collect())
    }
}

impl From<Expr> for RankByAnnValue {
    fn from(value: Expr) -> Self {
        Self::Expr(Box::new(value))
    }
}

impl From<RankByKnnValue> for RankByAnnValue {
    fn from(value: RankByKnnValue) -> Self {
        match value {
            RankByKnnValue::Vector(value) => Self::Vector(value),
            RankByKnnValue::Multi(value) => Self::Multi(value),
            RankByKnnValue::Expr(value) => Self::Expr(value),
        }
    }
}

/// One of the shapes `kNN` accepts for its `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum RankByKnnValue {
    /// The query vector.
    Vector(Vec<f32>),
    /// The query vectors, e.g. one per query token.
    Multi(Vec<Vec<f32>>),
    /// An expression that computes a value.
    Expr(Box<Expr>),
}

impl From<Vec<f32>> for RankByKnnValue {
    fn from(value: Vec<f32>) -> Self {
        Self::Vector(value)
    }
}

impl From<&[f32]> for RankByKnnValue {
    fn from(value: &[f32]) -> Self {
        Self::Vector(value.to_vec())
    }
}

impl<const N: usize> From<[f32; N]> for RankByKnnValue {
    fn from(value: [f32; N]) -> Self {
        Self::Vector(value.into())
    }
}

impl From<Vec<Vec<f32>>> for RankByKnnValue {
    fn from(value: Vec<Vec<f32>>) -> Self {
        Self::Multi(value)
    }
}

impl From<&[Vec<f32>]> for RankByKnnValue {
    fn from(value: &[Vec<f32>]) -> Self {
        Self::Multi(value.to_vec())
    }
}

impl<const N: usize> From<[Vec<f32>; N]> for RankByKnnValue {
    fn from(value: [Vec<f32>; N]) -> Self {
        Self::Multi(value.into())
    }
}

impl<const M: usize, const N: usize> From<[[f32; M]; N]> for RankByKnnValue {
    fn from(value: [[f32; M]; N]) -> Self {
        Self::Multi(value.into_iter().map(Vec::from).collect())
    }
}

impl From<Expr> for RankByKnnValue {
    fn from(value: Expr) -> Self {
        Self::Expr(Box::new(value))
    }
}

impl From<RankByAnnValue> for RankByKnnValue {
    fn from(value: RankByAnnValue) -> Self {
        match value {
            RankByAnnValue::Vector(value) => Self::Vector(value),
            RankByAnnValue::Multi(value) => Self::Multi(value),
            RankByAnnValue::Expr(value) => Self::Expr(value),
        }
    }
}

/// The consistency level for a query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "level")]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum QueryConfigConsistency {
    /// Strong consistency. Requires a round-trip to object storage to fetch the latest writes.
    #[serde(rename = "strong")]
    Strong,
    /// Eventual consistency. Does not require a round-trip to object storage, but may not see the
    /// latest writes.
    #[serde(rename = "eventual")]
    Eventual(QueryConfigConsistencyEventual),
}

impl From<QueryConfigConsistencyEventual> for QueryConfigConsistency {
    fn from(value: QueryConfigConsistencyEventual) -> Self {
        Self::Eventual(value)
    }
}

/// Eventual consistency. Does not require a round-trip to object storage, but may not see the
/// latest writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct QueryConfigConsistencyEventual {
    /// How long the query may reuse the node's cached view of the namespace before refreshing it
    /// from object storage. Defaults to 60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_staleness_seconds: Option<u64>,
}

impl QueryConfigConsistencyEventual {
    /// Creates a new [`QueryConfigConsistencyEventual`].
    pub fn new() -> Self {
        Self::default()
    }

    /// How long the query may reuse the node's cached view of the namespace before refreshing it from
    /// object storage. Defaults to 60.
    pub fn max_staleness_seconds(mut self, max_staleness_seconds: u64) -> Self {
        self.max_staleness_seconds = Some(max_staleness_seconds);
        self
    }
}

/// A function used to calculate vector similarity.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`DistanceMetric::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DistanceMetric {
    /// Defined as `1 - cosine_similarity` and ranges from 0 to 2. Lower is better.
    #[serde(rename = "cosine_distance")]
    CosineDistance,
    /// Defined as `sum((x - y)^2)`. Lower is better.
    #[serde(rename = "euclidean_squared")]
    EuclideanSquared,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl DistanceMetric {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::CosineDistance => "cosine_distance",
            Self::EuclideanSquared => "euclidean_squared",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for DistanceMetric {
    fn from(value: &str) -> Self {
        match value {
            "cosine_distance" => Self::CosineDistance,
            "euclidean_squared" => Self::EuclideanSquared,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for DistanceMetric {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Specifies a value to group documents by.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`GroupBy::attr`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GroupBy {
    /// An attribute name to group documents by.
    Attr(String),
    /// An expression to group documents by, with an explicit output attribute name.
    Expr(BTreeMap<String, GroupByFunction>),
}

#[allow(deprecated)]
impl GroupBy {
    /// An attribute name to group documents by.
    pub fn attr(value: impl Into<String>) -> Self {
        Self::Attr(value.into())
    }

    /// An expression to group documents by, with an explicit output attribute name.
    pub fn expr(
        value: impl IntoIterator<Item = (impl Into<String>, impl Into<GroupByFunction>)>,
    ) -> Self {
        Self::Expr(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl From<String> for GroupBy {
    fn from(value: String) -> Self {
        Self::Attr(value)
    }
}

impl From<&str> for GroupBy {
    fn from(value: &str) -> Self {
        Self::Attr(value.to_owned())
    }
}

impl From<BTreeMap<String, GroupByFunction>> for GroupBy {
    fn from(value: BTreeMap<String, GroupByFunction>) -> Self {
        Self::Expr(value)
    }
}

#[allow(deprecated)]
impl Serialize for GroupBy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Attr(value) => value.serialize(serializer),
            Self::Expr(value) => value.serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for GroupBy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Attr(value));
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Expr(value));
        }
        Err(serde::de::Error::custom(format!(
            "invalid GroupBy: {value}"
        )))
    }
}

impl From<GroupBy> for Value {
    fn from(value: GroupBy) -> Self {
        to_json(value)
    }
}

/// A function that produces group keys.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`GroupByFunction::for_each_unique`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GroupByFunction {
    /// Use the `ForEachUnique` operator to explode an array attribute when grouping. Each unique
    /// element of the array becomes a separate group.
    ForEachUnique {
        /// The name of the array attribute to explode.
        attr: String,
    },
}

#[allow(deprecated)]
impl GroupByFunction {
    /// Use the `ForEachUnique` operator to explode an array attribute when grouping. Each unique
    /// element of the array becomes a separate group.
    pub fn for_each_unique(attr: impl Into<String>) -> Self {
        Self::ForEachUnique { attr: attr.into() }
    }
}

#[allow(deprecated)]
impl Serialize for GroupByFunction {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::ForEachUnique { attr } => ("ForEachUnique", attr).serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for GroupByFunction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "ForEachUnique")]) {
            if let Some(attr) = decode_element(&elements[1]) {
                return Ok(Self::ForEachUnique { attr });
            }
        }
        Err(serde::de::Error::custom(format!(
            "invalid GroupByFunction: {value}"
        )))
    }
}

impl From<GroupByFunction> for Value {
    fn from(value: GroupByFunction) -> Self {
        to_json(value)
    }
}

/// Whether to include attributes in the response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum IncludeAttributes {
    /// When `true`, include all attributes in the response. When `false`, include no attributes in
    /// the response.
    Bool(bool),
    /// Include exactly the specified attributes in the response.
    Array(Vec<String>),
}

impl From<bool> for IncludeAttributes {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Vec<String>> for IncludeAttributes {
    fn from(value: Vec<String>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<&str>> for IncludeAttributes {
    fn from(value: Vec<&str>) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<&[&str]> for IncludeAttributes {
    fn from(value: &[&str]) -> Self {
        Self::Array(value.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for IncludeAttributes {
    fn from(value: [&str; N]) -> Self {
        Self::Array(value.into_iter().map(str::to_owned).collect())
    }
}

/// Limits the documents returned by a query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum QueryLimit {
    Int(u64),
    /// Limits the documents returned by a query.
    Limit(Limit),
}

impl From<u64> for QueryLimit {
    fn from(value: u64) -> Self {
        Self::Int(value)
    }
}

impl From<Limit> for QueryLimit {
    fn from(value: Limit) -> Self {
        Self::Limit(value)
    }
}

/// Limits the documents returned by a query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Limit {
    /// Limits the total number of documents returned.
    pub total: u64,
    /// Limits the number of documents with the same value for a set of attributes (the "limit key")
    /// that can appear in the results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per: Option<LimitPer>,
}

impl Limit {
    /// Creates a new [`Limit`] from its required fields.
    pub fn new(total: u64) -> Self {
        Self {
            total,
            per: Default::default(),
        }
    }

    /// Limits the number of documents with the same value for a set of attributes (the "limit key")
    /// that can appear in the results.
    pub fn per(mut self, per: impl Into<LimitPer>) -> Self {
        self.per = Some(per.into());
        self
    }
}

/// Limits the number of documents with the same value for a set of attributes (the "limit key")
/// that can appear in the results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LimitPer {
    /// The attributes to include in the limit key.
    pub attributes: Vec<String>,
    /// The maximum number of documents to return for each value of the limit key.
    pub limit: u64,
}

impl LimitPer {
    /// Creates a new [`LimitPer`] from its required fields.
    pub fn new(attributes: impl IntoIterator<Item = impl Into<String>>, limit: u64) -> Self {
        Self {
            attributes: attributes.into_iter().map(Into::into).collect(),
            limit,
        }
    }
}

/// The encoding to use for vectors in the response.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`VectorEncoding::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VectorEncoding {
    /// `float`
    #[serde(rename = "float")]
    Float,
    /// `base64`
    #[serde(rename = "base64")]
    Base64,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl VectorEncoding {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Float => "float",
            Self::Base64 => "base64",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for VectorEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for VectorEncoding {
    fn from(value: &str) -> Self {
        match value {
            "float" => Self::Float,
            "base64" => Self::Base64,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for VectorEncoding {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// The response to a successful query explain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct ExplainQueryResponse {
    /// The textual representation of the query plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_text: Option<String>,
}

/// The response to a successful cache warm request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HintCacheWarmResponse {
    /// The status of the request.
    ///
    /// Always `"ACCEPTED"`.
    #[serde(default)]
    pub status: String,
    /// A message describing the result of the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for HintCacheWarmResponse {
    fn default() -> Self {
        Self {
            status: "ACCEPTED".to_owned(),
            message: Default::default(),
        }
    }
}

/// Metadata about a namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NamespaceMetadata {
    /// The approximate number of logical bytes in the namespace.
    pub approx_logical_bytes: u64,
    /// The approximate number of rows in the namespace.
    pub approx_row_count: u64,
    /// The timestamp when the namespace was created.
    pub created_at: jiff::Timestamp,
    /// The encryption configuration for a namespace.
    #[serde(deserialize_with = "crate::custom::encryption::deserialize")]
    pub encryption: Encryption,
    /// The state of the namespace's index.
    pub index: NamespaceMetadataIndex,
    /// The schema of the namespace.
    pub schema: BTreeMap<String, AttributeSchemaConfig>,
    /// The timestamp when the namespace was last modified by a write operation.
    pub updated_at: jiff::Timestamp,
    /// Configuration for namespace pinning, along with the current status of the pinned namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinning: Option<PinningConfigResponse>,
    /// Whether document and schema writes are rejected. Omitted when `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    /// Configuration for namespace sharding, which partitions a namespace's documents across
    /// multiple internal shards to scale indexing and query throughput beyond a single machine.
    /// Sharding can only be configured on a namespace's inaugural write, and cannot be added to or
    /// changed on an existing namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharding: Option<ShardingConfig>,
}

/// The state of the namespace's index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum NamespaceMetadataIndex {
    /// Every document in the namespace is indexed.
    #[serde(rename = "up-to-date")]
    UpToDate,
    /// Some documents are in the write-ahead log and haven't been indexed yet.
    #[serde(rename = "updating")]
    Updating(NamespaceMetadataIndexUpdating),
    /// A variant not known to this version of the SDK.
    #[serde(other)]
    Other,
}

impl From<NamespaceMetadataIndexUpdating> for NamespaceMetadataIndex {
    fn from(value: NamespaceMetadataIndexUpdating) -> Self {
        Self::Updating(value)
    }
}

/// Some documents are in the write-ahead log and haven't been indexed yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NamespaceMetadataIndexUpdating {
    /// The number of bytes in the namespace that are in the write-ahead log but have not yet been
    /// indexed.
    pub unindexed_bytes: u64,
    /// The number of rows in the write-ahead log that have not yet been indexed.
    pub unindexed_rows: u64,
}

/// Detailed configuration for an attribute attached to a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AttributeSchemaConfig {
    /// The data type of the attribute. Valid values: string, int, uint, float, uuid, datetime,
    /// bool, \[\]string, \[\]int, \[\]uint, \[\]float, \[\]uuid, \[\]datetime, \[\]bool,
    /// \[DIMS\]f16, \[DIMS\]f32, {}f16.
    pub r#type: AttributeType,
    /// Whether to create an approximate nearest neighbor index for the attribute. Can be a boolean
    /// or a detailed configuration object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ann: Option<Ann>,
    /// Whether to automatically embed this string attribute into a vector attribute. Can be a model
    /// name, a detailed configuration object, or `null` to remove an existing embedding
    /// configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed: Option<AttributeEmbed>,
    /// Whether or not the attributes can be used in filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filterable: Option<bool>,
    /// Whether this attribute can be used as part of a BM25 full-text search. Requires the `string`
    /// or `[]string` type, and by default, BM25-enabled attributes are not filterable. You can
    /// override this by setting `filterable: true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text_search: Option<FullTextSearch>,
    /// Whether to enable Fuzzy filters on this attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuzzy: Option<bool>,
    /// Whether to enable Glob filters on this attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<bool>,
    /// Whether to enable Regex filters on this attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<bool>,
    /// Whether to create a sparse kNN index for the attribute. Requires the `{}f16` type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sparse_knn: Option<SparseKnn>,
}

impl AttributeSchemaConfig {
    /// Creates a new [`AttributeSchemaConfig`] from its required fields.
    pub fn new(r#type: impl Into<AttributeType>) -> Self {
        Self {
            r#type: r#type.into(),
            ann: Default::default(),
            embed: Default::default(),
            filterable: Default::default(),
            full_text_search: Default::default(),
            fuzzy: Default::default(),
            glob: Default::default(),
            regex: Default::default(),
            sparse_knn: Default::default(),
        }
    }

    /// Whether to create an approximate nearest neighbor index for the attribute. Can be a boolean or a
    /// detailed configuration object.
    pub fn ann(mut self, ann: impl Into<Ann>) -> Self {
        self.ann = Some(ann.into());
        self
    }

    /// Whether to automatically embed this string attribute into a vector attribute. Can be a model
    /// name, a detailed configuration object, or `null` to remove an existing embedding configuration.
    pub fn embed(mut self, embed: impl Into<AttributeEmbed>) -> Self {
        self.embed = Some(embed.into());
        self
    }

    /// Whether or not the attributes can be used in filters.
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = Some(filterable);
        self
    }

    /// Whether this attribute can be used as part of a BM25 full-text search. Requires the `string` or
    /// `[]string` type, and by default, BM25-enabled attributes are not filterable. You can override
    /// this by setting `filterable: true`.
    pub fn full_text_search(mut self, full_text_search: impl Into<FullTextSearch>) -> Self {
        self.full_text_search = Some(full_text_search.into());
        self
    }

    /// Whether to enable Fuzzy filters on this attribute.
    pub fn fuzzy(mut self, fuzzy: bool) -> Self {
        self.fuzzy = Some(fuzzy);
        self
    }

    /// Whether to enable Glob filters on this attribute.
    pub fn glob(mut self, glob: bool) -> Self {
        self.glob = Some(glob);
        self
    }

    /// Whether to enable Regex filters on this attribute.
    pub fn regex(mut self, regex: bool) -> Self {
        self.regex = Some(regex);
        self
    }

    /// Whether to create a sparse kNN index for the attribute. Requires the `{}f16` type.
    pub fn sparse_knn(mut self, sparse_knn: impl Into<SparseKnn>) -> Self {
        self.sparse_knn = Some(sparse_knn.into());
        self
    }
}

impl From<AttributeType> for AttributeSchemaConfig {
    fn from(value: AttributeType) -> Self {
        Self::new(value)
    }
}

/// The data type of the attribute. Valid values: string, int, uint, float, uuid, datetime, bool,
/// \[\]string, \[\]int, \[\]uint, \[\]float, \[\]uuid, \[\]datetime, \[\]bool, \[DIMS\]f16,
/// \[DIMS\]f32, {}f16.
pub type AttributeType = String;

/// Whether to create an approximate nearest neighbor index for the attribute. Can be a boolean or a
/// detailed configuration object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum Ann {
    /// Whether to create an ANN index with the default configuration.
    Bool(bool),
    /// Configuration options for ANN (Approximate Nearest Neighbor) indexing.
    Config(AnnConfig),
    /// A value not known to this version of the SDK.
    Other(Value),
}

impl From<bool> for Ann {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<AnnConfig> for Ann {
    fn from(value: AnnConfig) -> Self {
        Self::Config(value)
    }
}

/// Configuration options for ANN (Approximate Nearest Neighbor) indexing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct AnnConfig {
    /// A function used to calculate vector similarity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_metric: Option<DistanceMetric>,
    /// Opt in to late-interaction (MUVERA) indexing. Only valid on fixed-dim `[][N]f32` vector
    /// array attributes, and is required to enable an ANN index on such attributes. Defaults to
    /// `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub late_interaction: Option<bool>,
}

impl AnnConfig {
    /// Creates a new [`AnnConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// A function used to calculate vector similarity.
    pub fn distance_metric(mut self, distance_metric: impl Into<DistanceMetric>) -> Self {
        self.distance_metric = Some(distance_metric.into());
        self
    }

    /// Opt in to late-interaction (MUVERA) indexing. Only valid on fixed-dim `[][N]f32` vector array
    /// attributes, and is required to enable an ANN index on such attributes. Defaults to `false`.
    pub fn late_interaction(mut self, late_interaction: bool) -> Self {
        self.late_interaction = Some(late_interaction);
        self
    }
}

/// Whether to automatically embed this string attribute into a vector attribute. Can be a model
/// name, a detailed configuration object, or `null` to remove an existing embedding configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum AttributeEmbed {
    /// An explicit `null`.
    Null,
    /// The model to use for embedding. If you only specify a model, turbopuffer will generate a
    /// vector attribute for you to store the embedding.
    String(String),
    /// Configuration options for automatic embedding.
    Config(AttributeEmbedConfig),
    /// A value not known to this version of the SDK.
    Other(Value),
}

impl From<String> for AttributeEmbed {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for AttributeEmbed {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<AttributeEmbedConfig> for AttributeEmbed {
    fn from(value: AttributeEmbedConfig) -> Self {
        Self::Config(value)
    }
}

/// Configuration options for automatic embedding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AttributeEmbedConfig {
    /// The model to use for embedding. See our documentation for a list of models supported in each
    /// region.
    pub model: String,
    /// The name of an existing vector attribute to store embeddings in. If omitted, turbopuffer
    /// will generate a computed vector attribute named `$embed_<attribute>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// The dimensionality to embed at. If not set, will pick the default for this model. If you're
    /// storing embeddings in an existing attribute, this can be omitted, and may not be set to a
    /// value other than the dimensions of that attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dims: Option<u64>,
}

impl AttributeEmbedConfig {
    /// Creates a new [`AttributeEmbedConfig`] from its required fields.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            attribute: Default::default(),
            dims: Default::default(),
        }
    }

    /// The name of an existing vector attribute to store embeddings in. If omitted, turbopuffer will
    /// generate a computed vector attribute named `$embed_<attribute>`.
    pub fn attribute(mut self, attribute: impl Into<String>) -> Self {
        self.attribute = Some(attribute.into());
        self
    }

    /// The dimensionality to embed at. If not set, will pick the default for this model. If you're
    /// storing embeddings in an existing attribute, this can be omitted, and may not be set to a value
    /// other than the dimensions of that attribute.
    pub fn dims(mut self, dims: u64) -> Self {
        self.dims = Some(dims);
        self
    }
}

impl From<String> for AttributeEmbedConfig {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for AttributeEmbedConfig {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Whether this attribute can be used as part of a BM25 full-text search. Requires the `string` or
/// `[]string` type, and by default, BM25-enabled attributes are not filterable. You can override
/// this by setting `filterable: true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum FullTextSearch {
    /// Whether to enable full-text search with the default configuration.
    Bool(bool),
    /// Configuration options for full-text search.
    Config(FullTextSearchConfig),
    /// A value not known to this version of the SDK.
    Other(Value),
}

impl From<bool> for FullTextSearch {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<FullTextSearchConfig> for FullTextSearch {
    fn from(value: FullTextSearchConfig) -> Self {
        Self::Config(value)
    }
}

/// Configuration options for full-text search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct FullTextSearchConfig {
    /// Whether to convert each non-ASCII character in a token to its ASCII equivalent, if one
    /// exists (e.g., à -> a). Defaults to `false` (i.e., no folding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ascii_folding: Option<bool>,
    /// The `b` document length normalization parameter for BM25. Defaults to `0.75`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<f64>,
    /// Whether searching is case-sensitive. Defaults to `false` (i.e. case-insensitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    /// The `k1` term saturation parameter for BM25. Defaults to `1.2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k1: Option<f64>,
    /// Describes the language of a text attribute. Defaults to `english`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    /// Maximum length of a token in bytes. Tokens larger than this value during tokenization will
    /// be filtered out. Has to be between `1` and `254` (inclusive). Defaults to `39`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_token_length: Option<u64>,
    /// Removes common words from the text based on language. Defaults to `true` (i.e. remove common
    /// words).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_stopwords: Option<bool>,
    /// Language-specific stemming for the text. Defaults to `false` (i.e., do not stem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stemming: Option<bool>,
    /// The tokenizer to use for full-text search on an attribute. Defaults to `word_v4`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokenizer: Option<Tokenizer>,
}

impl FullTextSearchConfig {
    /// Creates a new [`FullTextSearchConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether to convert each non-ASCII character in a token to its ASCII equivalent, if one exists
    /// (e.g., à -> a). Defaults to `false` (i.e., no folding).
    pub fn ascii_folding(mut self, ascii_folding: bool) -> Self {
        self.ascii_folding = Some(ascii_folding);
        self
    }

    /// The `b` document length normalization parameter for BM25. Defaults to `0.75`.
    pub fn b(mut self, b: f64) -> Self {
        self.b = Some(b);
        self
    }

    /// Whether searching is case-sensitive. Defaults to `false` (i.e. case-insensitive).
    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = Some(case_sensitive);
        self
    }

    /// The `k1` term saturation parameter for BM25. Defaults to `1.2`.
    pub fn k1(mut self, k1: f64) -> Self {
        self.k1 = Some(k1);
        self
    }

    /// Describes the language of a text attribute. Defaults to `english`.
    pub fn language(mut self, language: impl Into<Language>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Maximum length of a token in bytes. Tokens larger than this value during tokenization will be
    /// filtered out. Has to be between `1` and `254` (inclusive). Defaults to `39`.
    pub fn max_token_length(mut self, max_token_length: u64) -> Self {
        self.max_token_length = Some(max_token_length);
        self
    }

    /// Removes common words from the text based on language. Defaults to `true` (i.e. remove common
    /// words).
    pub fn remove_stopwords(mut self, remove_stopwords: bool) -> Self {
        self.remove_stopwords = Some(remove_stopwords);
        self
    }

    /// Language-specific stemming for the text. Defaults to `false` (i.e., do not stem).
    pub fn stemming(mut self, stemming: bool) -> Self {
        self.stemming = Some(stemming);
        self
    }

    /// The tokenizer to use for full-text search on an attribute. Defaults to `word_v4`.
    pub fn tokenizer(mut self, tokenizer: impl Into<Tokenizer>) -> Self {
        self.tokenizer = Some(tokenizer.into());
        self
    }
}

/// Describes the language of a text attribute. Defaults to `english`.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`Language::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Language {
    /// `arabic`
    #[serde(rename = "arabic")]
    Arabic,
    /// `danish`
    #[serde(rename = "danish")]
    Danish,
    /// `dutch`
    #[serde(rename = "dutch")]
    Dutch,
    /// `english`
    #[serde(rename = "english")]
    English,
    /// `finnish`
    #[serde(rename = "finnish")]
    Finnish,
    /// `french`
    #[serde(rename = "french")]
    French,
    /// `german`
    #[serde(rename = "german")]
    German,
    /// `greek`
    #[serde(rename = "greek")]
    Greek,
    /// `hungarian`
    #[serde(rename = "hungarian")]
    Hungarian,
    /// `italian`
    #[serde(rename = "italian")]
    Italian,
    /// `norwegian`
    #[serde(rename = "norwegian")]
    Norwegian,
    /// `portuguese`
    #[serde(rename = "portuguese")]
    Portuguese,
    /// `romanian`
    #[serde(rename = "romanian")]
    Romanian,
    /// `russian`
    #[serde(rename = "russian")]
    Russian,
    /// `spanish`
    #[serde(rename = "spanish")]
    Spanish,
    /// `swedish`
    #[serde(rename = "swedish")]
    Swedish,
    /// `tamil`
    #[serde(rename = "tamil")]
    Tamil,
    /// `turkish`
    #[serde(rename = "turkish")]
    Turkish,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl Language {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Arabic => "arabic",
            Self::Danish => "danish",
            Self::Dutch => "dutch",
            Self::English => "english",
            Self::Finnish => "finnish",
            Self::French => "french",
            Self::German => "german",
            Self::Greek => "greek",
            Self::Hungarian => "hungarian",
            Self::Italian => "italian",
            Self::Norwegian => "norwegian",
            Self::Portuguese => "portuguese",
            Self::Romanian => "romanian",
            Self::Russian => "russian",
            Self::Spanish => "spanish",
            Self::Swedish => "swedish",
            Self::Tamil => "tamil",
            Self::Turkish => "turkish",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for Language {
    fn from(value: &str) -> Self {
        match value {
            "arabic" => Self::Arabic,
            "danish" => Self::Danish,
            "dutch" => Self::Dutch,
            "english" => Self::English,
            "finnish" => Self::Finnish,
            "french" => Self::French,
            "german" => Self::German,
            "greek" => Self::Greek,
            "hungarian" => Self::Hungarian,
            "italian" => Self::Italian,
            "norwegian" => Self::Norwegian,
            "portuguese" => Self::Portuguese,
            "romanian" => Self::Romanian,
            "russian" => Self::Russian,
            "spanish" => Self::Spanish,
            "swedish" => Self::Swedish,
            "tamil" => Self::Tamil,
            "turkish" => Self::Turkish,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for Language {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// The tokenizer to use for full-text search on an attribute. Defaults to `word_v4`.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`Tokenizer::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Tokenizer {
    /// `pre_tokenized_array`
    #[serde(rename = "pre_tokenized_array")]
    PreTokenizedArray,
    /// `word_v0`
    #[serde(rename = "word_v0")]
    WordV0,
    /// `word_v1`
    #[serde(rename = "word_v1")]
    WordV1,
    /// `word_v2`
    #[serde(rename = "word_v2")]
    WordV2,
    /// `word_v3`
    #[serde(rename = "word_v3")]
    WordV3,
    /// `word_v4`
    #[serde(rename = "word_v4")]
    WordV4,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl Tokenizer {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::PreTokenizedArray => "pre_tokenized_array",
            Self::WordV0 => "word_v0",
            Self::WordV1 => "word_v1",
            Self::WordV2 => "word_v2",
            Self::WordV3 => "word_v3",
            Self::WordV4 => "word_v4",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for Tokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for Tokenizer {
    fn from(value: &str) -> Self {
        match value {
            "pre_tokenized_array" => Self::PreTokenizedArray,
            "word_v0" => Self::WordV0,
            "word_v1" => Self::WordV1,
            "word_v2" => Self::WordV2,
            "word_v3" => Self::WordV3,
            "word_v4" => Self::WordV4,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for Tokenizer {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Whether to create a sparse kNN index for the attribute. Requires the `{}f16` type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SparseKnn {
    /// A function used to calculate sparse vector similarity.
    pub distance_metric: SparseDistanceMetric,
}

impl SparseKnn {
    /// Creates a new [`SparseKnn`] from its required fields.
    pub fn new(distance_metric: impl Into<SparseDistanceMetric>) -> Self {
        Self {
            distance_metric: distance_metric.into(),
        }
    }
}

impl From<SparseDistanceMetric> for SparseKnn {
    fn from(value: SparseDistanceMetric) -> Self {
        Self::new(value)
    }
}

/// A function used to calculate sparse vector similarity.
///
/// Converts from strings with [`From`], mapping values this version of the SDK doesn't know about to
/// [`SparseDistanceMetric::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SparseDistanceMetric {
    /// `dot_product`
    #[serde(rename = "dot_product")]
    DotProduct,
    /// A value not known to this version of the SDK.
    #[serde(untagged)]
    Other(String),
}

impl SparseDistanceMetric {
    /// The value's representation in the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::DotProduct => "dot_product",
            Self::Other(value) => value,
        }
    }
}

impl std::fmt::Display for SparseDistanceMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for SparseDistanceMetric {
    fn from(value: &str) -> Self {
        match value {
            "dot_product" => Self::DotProduct,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for SparseDistanceMetric {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Configuration for namespace pinning, along with the current status of the pinned namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct PinningConfigResponse {
    /// The number of read replicas to provision. Defaults to 1 if not specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u64>,
    /// Operational status for a pinned namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PinningStatus>,
}

/// Operational status for a pinned namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PinningStatus {
    /// The number of replicas that are warm and serving traffic.
    pub ready_replicas: u64,
    /// The number of running replicas for the namespace. Replicas are billed once running, even
    /// before they finish warming their caches and become ready to serve traffic. This count is
    /// updated independently and may briefly disagree with the other status fields.
    pub replicas: u64,
    /// The timestamp of the latest pinning status snapshot.
    pub updated_at: jiff::Timestamp,
    /// Aggregate utilization for the pinned namespace, reported as a value between 0.0 and 1.0.
    pub utilization: f64,
}

/// Configuration for namespace sharding, which partitions a namespace's documents across multiple
/// internal shards to scale indexing and query throughput beyond a single machine. Sharding can
/// only be configured on a namespace's inaugural write, and cannot be added to or changed on an
/// existing namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ShardingConfig {
    /// The number of shards to partition the namespace into.
    pub num_shards: u32,
}

impl ShardingConfig {
    /// Creates a new [`ShardingConfig`] from its required fields.
    pub fn new(num_shards: u32) -> Self {
        Self { num_shards }
    }
}

/// Query, filter, full-text search and vector search documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct Query {
    /// Aggregations to compute over all documents in the namespace that match the filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate_by: Option<BTreeMap<String, AggregateBy>>,
    /// Computes additional values on documents returned by a query. Each key is the name of the
    /// computed attribute; each value is an expression describing how to compute it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compute_attributes: Option<BTreeMap<String, Expr>>,
    /// A function used to calculate vector similarity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_metric: Option<DistanceMetric>,
    /// List of attribute names to exclude from the response. All other attributes will be included
    /// in the response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_attributes: Option<Vec<String>>,
    /// Exact filters for attributes to refine search results for. Think of it as a SQL WHERE
    /// clause.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Filter>,
    /// Groups documents by the specified attributes (the "group key") before computing aggregates.
    /// Aggregates are computed separately for each group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<GroupBy>>,
    /// Whether to include attributes in the response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_attributes: Option<IncludeAttributes>,
    /// Limits the documents returned by a query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<QueryLimit>,
    /// Number of documents to skip before returning results. Supported only in v2 queries with an
    /// explicit `rank_by` and `top_k` or `limit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// How to rank the documents in the namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_by: Option<RankBy>,
    /// The number of results to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
}

impl Query {
    /// Creates a new [`Query`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Aggregations to compute over all documents in the namespace that match the filters.
    ///
    /// Adds to any entries already set, rather than replacing them.
    pub fn aggregate_by(
        mut self,
        aggregate_by: impl IntoIterator<Item = (impl Into<String>, impl Into<AggregateBy>)>,
    ) -> Self {
        self.aggregate_by
            .get_or_insert_with(Default::default)
            .extend(aggregate_by.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Computes additional values on documents returned by a query. Each key is the name of the
    /// computed attribute; each value is an expression describing how to compute it.
    ///
    /// Adds to any entries already set, rather than replacing them.
    pub fn compute_attributes(
        mut self,
        compute_attributes: impl IntoIterator<Item = (impl Into<String>, impl Into<Expr>)>,
    ) -> Self {
        self.compute_attributes
            .get_or_insert_with(Default::default)
            .extend(
                compute_attributes
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into())),
            );
        self
    }

    /// A function used to calculate vector similarity.
    pub fn distance_metric(mut self, distance_metric: impl Into<DistanceMetric>) -> Self {
        self.distance_metric = Some(distance_metric.into());
        self
    }

    /// List of attribute names to exclude from the response. All other attributes will be included in
    /// the response.
    ///
    /// Adds to any items already set, rather than replacing them.
    pub fn exclude_attributes(
        mut self,
        exclude_attributes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.exclude_attributes
            .get_or_insert_with(Default::default)
            .extend(exclude_attributes.into_iter().map(Into::into));
        self
    }

    /// Exact filters for attributes to refine search results for. Think of it as a SQL WHERE clause.
    pub fn filters(mut self, filters: impl Into<Filter>) -> Self {
        self.filters = Some(filters.into());
        self
    }

    /// Groups documents by the specified attributes (the "group key") before computing aggregates.
    /// Aggregates are computed separately for each group.
    ///
    /// Adds to any items already set, rather than replacing them.
    pub fn group_by(mut self, group_by: impl IntoIterator<Item = impl Into<GroupBy>>) -> Self {
        self.group_by
            .get_or_insert_with(Default::default)
            .extend(group_by.into_iter().map(Into::into));
        self
    }

    /// Whether to include attributes in the response.
    ///
    /// A list adds to a list already set, rather than replacing it.
    pub fn include_attributes(mut self, include_attributes: impl Into<IncludeAttributes>) -> Self {
        self.include_attributes = Some(
            match (self.include_attributes.take(), include_attributes.into()) {
                (Some(IncludeAttributes::Array(mut existing)), IncludeAttributes::Array(more)) => {
                    existing.extend(more);
                    IncludeAttributes::Array(existing)
                }
                (_, value) => value,
            },
        );
        self
    }

    /// Limits the documents returned by a query.
    pub fn limit(mut self, limit: impl Into<QueryLimit>) -> Self {
        self.limit = Some(limit.into());
        self
    }

    /// Number of documents to skip before returning results. Supported only in v2 queries with an
    /// explicit `rank_by` and `top_k` or `limit`.
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// How to rank the documents in the namespace.
    pub fn rank_by(mut self, rank_by: impl Into<RankBy>) -> Self {
        self.rank_by = Some(rank_by.into());
        self
    }

    /// The number of results to return.
    pub fn top_k(mut self, top_k: u64) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

impl From<&Query> for Query {
    fn from(value: &Query) -> Self {
        value.clone()
    }
}

/// Limits the total number of reranked documents returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum MultiQueryLimit {
    /// Limits the total number of reranked documents returned.
    Int(u64),
    /// Limits the total number of reranked documents returned.
    RerankLimit(RerankLimit),
}

impl From<u64> for MultiQueryLimit {
    fn from(value: u64) -> Self {
        Self::Int(value)
    }
}

impl From<RerankLimit> for MultiQueryLimit {
    fn from(value: RerankLimit) -> Self {
        Self::RerankLimit(value)
    }
}

/// Limits the total number of reranked documents returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RerankLimit {
    /// Limits the total number of documents returned after reranking.
    pub total: u64,
}

impl RerankLimit {
    /// Creates a new [`RerankLimit`] from its required fields.
    pub fn new(total: u64) -> Self {
        Self { total }
    }
}

/// Configuration for combining the rows returned by a multi-query into a single ranked list, using
/// RRF or a reranking model.
///
/// Serialized as JSON arrays (tuples) and constructed with the associated functions, e.g.
/// [`RerankBy::rrf`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RerankBy {
    /// Rerank with RRF.
    Rrf {
        /// Configuration options for RRF.
        params: Option<RrfParams>,
    },
    /// Rerank retrieved candidates with a model.
    Model(RerankModelParams),
}

#[allow(deprecated)]
impl RerankBy {
    /// Rerank with RRF.
    pub fn rrf() -> Self {
        Self::Rrf { params: None }
    }

    /// Rerank with RRF.
    ///
    /// Like [`RerankBy::rrf`], with `params`.
    pub fn rrf_with_params(params: impl Into<RrfParams>) -> Self {
        Self::Rrf {
            params: Some(params.into()),
        }
    }

    /// Rerank retrieved candidates with a model.
    pub fn model(value: impl Into<RerankModelParams>) -> Self {
        Self::Model(value.into())
    }
}

impl From<RerankModelParams> for RerankBy {
    fn from(value: RerankModelParams) -> Self {
        Self::Model(value)
    }
}

#[allow(deprecated)]
impl Serialize for RerankBy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Rrf { params: None } => ("RRF",).serialize(serializer),
            Self::Rrf {
                params: Some(params),
            } => ("RRF", params).serialize(serializer),
            Self::Model(value) => value.serialize(serializer),
        }
    }
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for RerankBy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Some(elements) = tuple_elements(&value, 2, &[(0, "RRF")]) {
            if let Some(params) = decode_element(&elements[1]) {
                return Ok(Self::Rrf {
                    params: Some(params),
                });
            }
        }
        if tuple_elements(&value, 1, &[(0, "RRF")]).is_some() {
            return Ok(Self::Rrf { params: None });
        }
        if let Some(value) = decode_element(&value) {
            return Ok(Self::Model(value));
        }
        Err(serde::de::Error::custom(format!(
            "invalid RerankBy: {value}"
        )))
    }
}

impl From<RerankBy> for Value {
    fn from(value: RerankBy) -> Self {
        to_json(value)
    }
}

/// Configuration options for RRF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct RrfParams {
    /// RRF rank constant (`k`). Must be greater than zero. Defaults to `60`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_constant: Option<u64>,
    /// A positive weight for each subquery, in the same order as `queries`. The number of weights
    /// must match the number of subqueries. When omitted, every subquery has a weight of `1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weights: Option<Vec<f32>>,
}

impl RrfParams {
    /// Creates a new [`RrfParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// RRF rank constant (`k`). Must be greater than zero. Defaults to `60`.
    pub fn rank_constant(mut self, rank_constant: u64) -> Self {
        self.rank_constant = Some(rank_constant);
        self
    }

    /// A positive weight for each subquery, in the same order as `queries`. The number of weights must
    /// match the number of subqueries. When omitted, every subquery has a weight of `1`.
    pub fn weights(mut self, weights: impl Into<Vec<f32>>) -> Self {
        self.weights = Some(weights.into());
        self
    }
}

/// Rerank retrieved candidates with a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RerankModelParams {
    /// The string attribute to score. Must be included in `include_attributes` in every query.
    pub attribute: String,
    /// Which reranking model to use.
    pub model: String,
    /// The query text to score each document against.
    pub query: String,
}

impl RerankModelParams {
    /// Creates a new [`RerankModelParams`] from its required fields.
    pub fn new(
        model: impl Into<String>,
        attribute: impl Into<String>,
        query: impl Into<String>,
    ) -> Self {
        Self {
            attribute: attribute.into(),
            model: model.into(),
            query: query.into(),
        }
    }
}

impl From<RerankModelParams> for Value {
    fn from(value: RerankModelParams) -> Self {
        to_json(value)
    }
}

/// The result of a multi-query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MultiQueryResult {
    /// The billing information for a query.
    pub billing: QueryBilling,
    /// The performance information for a query.
    pub performance: QueryPerformance,
    /// The results of each subquery, in the same order as `queries`. With `rerank_by`, a single
    /// result combining the subqueries' rows.
    pub results: Vec<SingleQueryResult>,
}

/// The performance information for a query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueryPerformance {
    /// the approximate number of documents in the namespace.
    pub approx_namespace_size: u64,
    /// The ratio of cache hits to total cache lookups.
    pub cache_hit_ratio: f64,
    /// A qualitative description of the cache hit ratio (`hot`, `warm`, or `cold`).
    pub cache_temperature: String,
    /// The number of unindexed documents processed by the query.
    pub exhaustive_search_count: u64,
    /// Request time measured on the server, excluding time spent waiting due to the namespace
    /// concurrency limit.
    pub query_execution_ms: u64,
    /// Request time measured on the server, including time spent waiting for other queries to
    /// complete if the namespace was at its concurrency limit.
    pub server_total_ms: u64,
    /// Time spent embedding text, in milliseconds. Only set when using a native embedding model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_ms: Option<u64>,
    /// The number of tokens embedded. Only set when using a native embedding model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_tokens: Option<u64>,
}

/// The result of a query, or of one subquery of a multi-query. With `aggregate_by`, `aggregations`
/// maps each aggregation's label to its value. With `group_by` as well, `aggregation_groups` holds
/// one object per group instead, with its group key and the value of each aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct SingleQueryResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregation_groups: Vec<AggregationGroup>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub aggregations: BTreeMap<String, Value>,
    /// The documents that matched the query, ordered by `rank_by`. Only present when `rank_by` is
    /// set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Row>,
}

/// A single aggregation group.
pub type AggregationGroup = BTreeMap<String, Value>;

/// A single document, in a row-based format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Row {
    /// An identifier for a document.
    pub id: Id,
    /// A vector embedding associated with a document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vector>,
    /// Properties not covered by the fields above.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Row {
    /// Creates a new [`Row`] from its required fields.
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            vector: Default::default(),
            extra: Default::default(),
        }
    }

    /// A vector embedding associated with a document.
    pub fn vector(mut self, vector: impl Into<Vector>) -> Self {
        self.vector = Some(vector.into());
        self
    }

    /// Sets a property not covered by the fields of this type.
    ///
    /// `id` and `vector` are fields of this type, so they're set on those fields instead.
    pub fn insert(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        let key = key.into();
        let value = to_json(value);
        match key.as_str() {
            "id" => match <Id as Deserialize>::deserialize(&value) {
                Ok(field) => self.id = field,
                Err(_) => {
                    self.extra.insert(key, value);
                }
            },
            "vector" => match <Vector as Deserialize>::deserialize(&value) {
                Ok(field) => self.vector = Some(field),
                Err(_) => {
                    self.extra.insert(key, value);
                }
            },
            _ => {
                self.extra.insert(key, value);
            }
        }
        self
    }

    /// Creates a [`Row`] from any serializable value with the same shape, such as one of your
    /// application's own types. Properties that aren't fields of this type end up as additional properties.
    ///
    /// Fails if `value` doesn't serialize to a JSON object with the required fields.
    pub fn from_serializable<T: Serialize + ?Sized>(value: &T) -> Result<Self, serde_json::Error> {
        serde_json::to_value(value).and_then(serde_json::from_value)
    }

    /// Deserializes this value, including any additional properties, into another type, such as one of your
    /// application's own types.
    pub fn deserialize_as<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::to_value(self).and_then(serde_json::from_value)
    }

    /// Returns a property not covered by the fields of this type.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.extra.get(key)
    }
}

impl From<&Row> for Row {
    fn from(value: &Row) -> Self {
        value.clone()
    }
}

/// A vector embedding associated with a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum Vector {
    /// A dense vector encoded as a base64 string.
    String(String),
    /// A dense vector encoded as an array of floats.
    Array(Vec<f32>),
    /// A value not known to this version of the SDK.
    Other(Value),
}

impl From<String> for Vector {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Vector {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Vec<f32>> for Vector {
    fn from(value: Vec<f32>) -> Self {
        Self::Array(value)
    }
}

impl From<&[f32]> for Vector {
    fn from(value: &[f32]) -> Self {
        Self::Array(value.to_vec())
    }
}

impl<const N: usize> From<[f32; N]> for Vector {
    fn from(value: [f32; N]) -> Self {
        Self::Array(value.into())
    }
}

/// The current status of a copy operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum CopyFromNamespaceOperation {
    /// The copy is still in progress.
    #[serde(rename = "running")]
    Running(CopyFromNamespaceOperationRunning),
    /// The copy has finished, successfully or not.
    #[serde(rename = "finished")]
    Finished(CopyFromNamespaceOperationFinished),
    /// A variant not known to this version of the SDK.
    #[serde(other)]
    Other,
}

impl From<CopyFromNamespaceOperationRunning> for CopyFromNamespaceOperation {
    fn from(value: CopyFromNamespaceOperationRunning) -> Self {
        Self::Running(value)
    }
}

impl From<CopyFromNamespaceOperationFinished> for CopyFromNamespaceOperation {
    fn from(value: CopyFromNamespaceOperationFinished) -> Self {
        Self::Finished(value)
    }
}

/// The copy is still in progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CopyFromNamespaceOperationRunning {
    /// The time at which the operation started.
    pub start_time: jiff::Timestamp,
    /// A freeform description of the operation's progress. May be absent, and its format may
    /// change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
}

/// The copy has finished, successfully or not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CopyFromNamespaceOperationFinished {
    /// The time at which the operation finished.
    pub finish_time: jiff::Timestamp,
    /// The result of the copy.
    pub result: CopyFromNamespaceOperationResult,
    /// The time at which the operation started.
    pub start_time: jiff::Timestamp,
}

/// The result of a finished copy operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum CopyFromNamespaceOperationResult {
    /// The copy succeeded, with the result of its write.
    #[serde(rename = "success")]
    Success(WriteResult),
    /// The copy failed, with its error.
    #[serde(rename = "error")]
    Error(OperationError),
}

impl From<WriteResult> for CopyFromNamespaceOperationResult {
    fn from(value: WriteResult) -> Self {
        Self::Success(value)
    }
}

impl From<OperationError> for CopyFromNamespaceOperationResult {
    fn from(value: OperationError) -> Self {
        Self::Error(value)
    }
}

/// The error of an asynchronous operation that failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OperationError {
    /// The response to an unsuccessful request.
    pub detail: ErrorResponse,
    /// The HTTP status code of the operation's error.
    pub status_code: u64,
}

/// The response to an unsuccessful request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ErrorResponse {
    /// The error message.
    pub error: String,
    /// The status of the request.
    ///
    /// Always `"error"`.
    #[serde(default)]
    pub status: String,
}

/// The result of a query. With `aggregate_by`, `aggregations` maps each aggregation's label to its
/// value. With `group_by` as well, `aggregation_groups` holds one object per group instead, with
/// its group key and the value of each aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueryResult {
    /// The billing information for a query.
    pub billing: QueryBilling,
    /// The performance information for a query.
    pub performance: QueryPerformance,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregation_groups: Vec<AggregationGroup>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub aggregations: BTreeMap<String, Value>,
    /// The documents that matched the query, ordered by `rank_by`. Only present when `rank_by` is
    /// set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Row>,
}

/// The response to a successful cache warm request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecallResponse {
    /// The average number of documents retrieved by the approximate nearest neighbor searches.
    pub avg_ann_count: f64,
    /// The average number of documents retrieved by the exhaustive searches.
    pub avg_exhaustive_count: f64,
    /// The average recall of the queries.
    pub avg_recall: f64,
    /// Ground truth data including query vectors and true nearest neighbors. Only included when
    /// include_ground_truth is true.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ground_truth: Vec<NamespaceRecallResponseGroundTruth>,
}

/// One of the searches the evaluation ran: its query vector, sampled at random from the namespace's
/// vectors unless `rank_by` is set, and the query's true nearest neighbors, found by exhaustive
/// search. Recall compares these neighbors to the results of the approximate nearest neighbor (ANN)
/// search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NamespaceRecallResponseGroundTruth {
    /// The true nearest neighbors with their distances and vectors.
    pub nearest_neighbors: Vec<Row>,
    /// The query vector used for this search.
    pub query_vector: Vec<f64>,
}

/// The response to a successful namespace schema request.
pub type SchemaResponse = BTreeMap<String, AttributeSchemaConfig>;

/// The response to a successful request to start a copy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StartCopyFromResponse {
    /// The token identifying the copy operation.
    pub token: String,
}

/// Configuration for namespace pinning.
/// - Missing field: no change to pinning configuration
/// - `null` or `false`: explicitly remove pinning
/// - `true`: enable pinning with default configuration
/// - Object: set pinning configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum NamespaceMetadataPatchPinning {
    /// An explicit `null`.
    Null,
    /// `true` to enable pinning with the default configuration, or `false` to remove pinning.
    Bool(bool),
    /// Configuration for namespace pinning.
    PinningConfig(PinningConfig),
}

impl From<bool> for NamespaceMetadataPatchPinning {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<PinningConfig> for NamespaceMetadataPatchPinning {
    fn from(value: PinningConfig) -> Self {
        Self::PinningConfig(value)
    }
}

/// Configuration for namespace pinning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct PinningConfig {
    /// The number of read replicas to provision. Defaults to 1 if not specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u64>,
}

impl PinningConfig {
    /// Creates a new [`PinningConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of read replicas to provision. Defaults to 1 if not specified.
    pub fn replicas(mut self, replicas: u64) -> Self {
        self.replicas = Some(replicas);
        self
    }
}

/// The schema for an attribute attached to a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum AttributeSchema {
    /// The data type of the attribute. Valid values: string, int, uint, float, uuid, datetime,
    /// bool, \[\]string, \[\]int, \[\]uint, \[\]float, \[\]uuid, \[\]datetime, \[\]bool,
    /// \[DIMS\]f16, \[DIMS\]f32, {}f16.
    Type(AttributeType),
    /// Detailed configuration for an attribute attached to a document.
    Config(AttributeSchemaConfig),
    /// Drops the attribute from the namespace. Cannot be combined with other schema settings.
    Drop(AttributeSchemaDrop),
}

impl From<AttributeType> for AttributeSchema {
    fn from(value: AttributeType) -> Self {
        Self::Type(value)
    }
}

impl From<&str> for AttributeSchema {
    fn from(value: &str) -> Self {
        Self::Type(value.to_owned())
    }
}

impl From<AttributeSchemaConfig> for AttributeSchema {
    fn from(value: AttributeSchemaConfig) -> Self {
        Self::Config(value)
    }
}

impl From<AttributeSchemaDrop> for AttributeSchema {
    fn from(value: AttributeSchemaDrop) -> Self {
        Self::Drop(value)
    }
}

/// Drops the attribute from the namespace. Cannot be combined with other schema settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AttributeSchemaDrop {
    /// Must be `true`.
    pub drop: bool,
}

impl AttributeSchemaDrop {
    /// Creates a new [`AttributeSchemaDrop`] from its required fields.
    pub fn new(drop: bool) -> Self {
        Self { drop }
    }
}

/// The updated schema for the namespace.
pub type UpdateSchemaResponse = BTreeMap<String, AttributeSchemaConfig>;

/// The namespace to create an instant, copy-on-write clone of.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum BranchFromNamespaceParams {
    /// The namespace to create an instant, copy-on-write clone of.
    String(String),
    /// Configuration for branching from another namespace.
    Config(BranchFromNamespaceConfig),
}

impl From<String> for BranchFromNamespaceParams {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for BranchFromNamespaceParams {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<BranchFromNamespaceConfig> for BranchFromNamespaceParams {
    fn from(value: BranchFromNamespaceConfig) -> Self {
        Self::Config(value)
    }
}

/// Configuration for branching from another namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BranchFromNamespaceConfig {
    /// The namespace to create an instant, copy-on-write clone of.
    pub source_namespace: String,
}

impl BranchFromNamespaceConfig {
    /// Creates a new [`BranchFromNamespaceConfig`] from its required fields.
    pub fn new(source_namespace: impl Into<String>) -> Self {
        Self {
            source_namespace: source_namespace.into(),
        }
    }
}

impl From<String> for BranchFromNamespaceConfig {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for BranchFromNamespaceConfig {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// The namespace to copy documents from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum CopyFromNamespaceParams {
    /// The namespace to copy documents from.
    String(String),
    /// Configuration for copying documents from another namespace.
    Config(CopyFromNamespaceConfig),
}

impl From<String> for CopyFromNamespaceParams {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for CopyFromNamespaceParams {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<CopyFromNamespaceConfig> for CopyFromNamespaceParams {
    fn from(value: CopyFromNamespaceConfig) -> Self {
        Self::Config(value)
    }
}

/// Configuration for copying documents from another namespace.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CopyFromNamespaceConfig {
    /// The namespace to copy documents from.
    pub source_namespace: String,
    /// (Optional) An API key for the organization containing the source namespace
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_api_key: Option<String>,
    /// (Optional) The region of the source namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_region: Option<String>,
}

impl CopyFromNamespaceConfig {
    /// Creates a new [`CopyFromNamespaceConfig`] from its required fields.
    pub fn new(source_namespace: impl Into<String>) -> Self {
        Self {
            source_namespace: source_namespace.into(),
            source_api_key: Default::default(),
            source_region: Default::default(),
        }
    }

    /// (Optional) An API key for the organization containing the source namespace
    pub fn source_api_key(mut self, source_api_key: impl Into<String>) -> Self {
        self.source_api_key = Some(source_api_key.into());
        self
    }

    /// (Optional) The region of the source namespace.
    pub fn source_region(mut self, source_region: impl Into<String>) -> Self {
        self.source_region = Some(source_region.into());
        self
    }
}

impl std::fmt::Debug for CopyFromNamespaceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopyFromNamespaceConfig")
            .field("source_namespace", &self.source_namespace)
            .field(
                "source_api_key",
                &self.source_api_key.as_ref().map(|_| "<redacted>"),
            )
            .field("source_region", &self.source_region)
            .finish()
    }
}

impl From<String> for CopyFromNamespaceConfig {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CopyFromNamespaceConfig {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Sets the attributes in `patch` on every document that matches `filters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PatchByFilter {
    /// Filter by attributes. Same syntax as the query endpoint.
    pub filters: Filter,
    pub patch: BTreeMap<String, Value>,
}

impl PatchByFilter {
    /// Creates a new [`PatchByFilter`] from its required fields.
    pub fn new(
        patch: impl IntoIterator<Item = (impl Into<String>, impl Serialize)>,
        filters: impl Into<Filter>,
    ) -> Self {
        Self {
            filters: filters.into(),
            patch: patch
                .into_iter()
                .map(|(k, v)| (k.into(), to_json(v)))
                .collect(),
        }
    }
}

/// A list of documents in columnar format. Each key is a column name, mapped to an array of values
/// for that column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Columns {
    /// The IDs of the documents.
    pub id: Vec<Id>,
    /// The vector embeddings of the documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<ColumnsVector>,
    /// Properties not covered by the fields above.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Vec<Value>>,
}

impl Columns {
    /// Creates a new [`Columns`] from its required fields.
    pub fn new(id: impl IntoIterator<Item = impl Into<Id>>) -> Self {
        Self {
            id: id.into_iter().map(Into::into).collect(),
            vector: Default::default(),
            extra: Default::default(),
        }
    }

    /// The vector embeddings of the documents.
    ///
    /// A list adds to a list already set, rather than replacing it.
    pub fn vector(mut self, vector: impl Into<ColumnsVector>) -> Self {
        self.vector = Some(match (self.vector.take(), vector.into()) {
            (Some(ColumnsVector::Array(mut existing)), ColumnsVector::Array(more)) => {
                existing.extend(more);
                ColumnsVector::Array(existing)
            }
            (_, value) => value,
        });
        self
    }

    /// Sets a property not covered by the fields of this type.
    ///
    /// `id` and `vector` are fields of this type, so they're set on those fields instead.
    pub fn insert(
        mut self,
        key: impl Into<String>,
        value: impl IntoIterator<Item = impl Serialize>,
    ) -> Self {
        let key = key.into();
        let value = value.into_iter().map(to_json).collect::<Vec<_>>();
        match key.as_str() {
            "id" => {
                let json = Value::Array(value);
                match <Vec<Id> as Deserialize>::deserialize(&json) {
                    Ok(field) => self.id = field,
                    Err(_) => {
                        if let Value::Array(value) = json {
                            self.extra.insert(key, value);
                        }
                    }
                }
            }
            "vector" => {
                let json = Value::Array(value);
                match <ColumnsVector as Deserialize>::deserialize(&json) {
                    Ok(field) => self.vector = Some(field),
                    Err(_) => {
                        if let Value::Array(value) = json {
                            self.extra.insert(key, value);
                        }
                    }
                }
            }
            _ => {
                self.extra.insert(key, value);
            }
        }
        self
    }

    /// Creates a [`Columns`] from any serializable value with the same shape, such as one of your
    /// application's own types. Properties that aren't fields of this type end up as additional properties.
    ///
    /// Fails if `value` doesn't serialize to a JSON object with the required fields.
    pub fn from_serializable<T: Serialize + ?Sized>(value: &T) -> Result<Self, serde_json::Error> {
        serde_json::to_value(value).and_then(serde_json::from_value)
    }

    /// Deserializes this value, including any additional properties, into another type, such as one of your
    /// application's own types.
    pub fn deserialize_as<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::to_value(self).and_then(serde_json::from_value)
    }

    /// Returns a property not covered by the fields of this type.
    pub fn get(&self, key: &str) -> Option<&Vec<Value>> {
        self.extra.get(key)
    }
}

/// The vector embeddings of the documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum ColumnsVector {
    /// The vector embeddings of the documents.
    Array(Vec<Vector>),
    /// A vector embedding associated with a document.
    Vector(Vector),
}

impl<T: Into<Vector>> From<Vec<T>> for ColumnsVector {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl From<&[Vector]> for ColumnsVector {
    fn from(value: &[Vector]) -> Self {
        Self::Array(value.to_vec())
    }
}

impl<T: Into<Vector>, const N: usize> From<[T; N]> for ColumnsVector {
    fn from(value: [T; N]) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl From<Vector> for ColumnsVector {
    fn from(value: Vector) -> Self {
        Self::Vector(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    #[test]
    #[allow(deprecated)]
    fn aggregate_by_round_trips() {
        let cases: Vec<(AggregateBy, Value)> = vec![
            (AggregateBy::count(), json!(["Count"])),
            (AggregateBy::sum("attr"), json!(["Sum", "attr"])),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: AggregateBy = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn expr_round_trips() {
        let cases: Vec<(Expr, Value)> = vec![
            (
                Expr::ref_new(ExprRefNew::new("$ref_new")),
                json!({"$ref_new":"$ref_new"}),
            ),
            (
                Expr::embed_with_params("value", EmbedParams::new()),
                json!(["Embed", "value", {}]),
            ),
            (Expr::embed("value"), json!(["Embed", "value"])),
            (
                Expr::vector_dist("attr", [0.5]),
                json!(["attr", "VectorDist", [0.5]]),
            ),
            (
                Expr::highlight_with_config("attr", HighlightConfigParams::new()),
                json!(["Highlight", "attr", {}]),
            ),
            (Expr::highlight("attr"), json!(["Highlight", "attr"])),
            (
                Expr::score(RankBy::ann("attr", [0.5])),
                json!(["attr", "ANN", [0.5]]),
            ),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: Expr = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn rank_by_round_trips() {
        let cases: Vec<(RankBy, Value)> = vec![
            (RankBy::ann("attr", [0.5]), json!(["attr", "ANN", [0.5]])),
            (
                RankBy::ann("attr", [vec![0.5]]),
                json!(["attr", "ANN", [[0.5]]]),
            ),
            (
                RankBy::ann("attr", Expr::embed_with_params("value", EmbedParams::new())),
                json!(["attr", "ANN", ["Embed", "value", {}]]),
            ),
            (RankBy::knn("attr", [0.5]), json!(["attr", "kNN", [0.5]])),
            (
                RankBy::knn("attr", [vec![0.5]]),
                json!(["attr", "kNN", [[0.5]]]),
            ),
            (
                RankBy::knn("attr", Expr::embed_with_params("value", EmbedParams::new())),
                json!(["attr", "kNN", ["Embed", "value", {}]]),
            ),
            (
                RankBy::sparse_knn("attr", [("key", 0.5)]),
                json!(["attr","SparseKNN",{"key":0.5}]),
            ),
            (
                RankBy::text(RankByText::bm25_with_params(
                    "attr",
                    "string",
                    Bm25ClauseParams::new(),
                )),
                json!(["attr", "BM25", "string", {}]),
            ),
            (
                RankBy::attribute("attr", RankByAttributeOrder::Asc),
                json!(["attr", "asc"]),
            ),
            (
                RankBy::attributes([RankByAttribute::new("attr", RankByAttributeOrder::Asc)]),
                json!([["attr", "asc"]]),
            ),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: RankBy = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn rank_by_text_round_trips() {
        let cases: Vec<(RankByText, Value)> = vec![
            (
                RankByText::bm25_with_params("attr", "string", Bm25ClauseParams::new()),
                json!(["attr", "BM25", "string", {}]),
            ),
            (
                RankByText::bm25_with_params("attr", ["string"], Bm25ClauseParams::new()),
                json!(["attr", "BM25", ["string"], {}]),
            ),
            (
                RankByText::bm25("attr", "string"),
                json!(["attr", "BM25", "string"]),
            ),
            (
                RankByText::bm25("attr", ["string"]),
                json!(["attr", "BM25", ["string"]]),
            ),
            (
                RankByText::sum([RankByText::bm25_with_params(
                    "attr",
                    "string",
                    Bm25ClauseParams::new(),
                )]),
                json!(["Sum", [["attr", "BM25", "string", {}]]]),
            ),
            (
                RankByText::max([RankByText::bm25_with_params(
                    "attr",
                    "string",
                    Bm25ClauseParams::new(),
                )]),
                json!(["Max", [["attr", "BM25", "string", {}]]]),
            ),
            (
                RankByText::product(
                    0.25,
                    RankByText::bm25_with_params("attr", "string", Bm25ClauseParams::new()),
                ),
                json!(["Product", 0.25, ["attr", "BM25", "string", {}]]),
            ),
            (
                RankByText::filter(Filter::eq("attr", "value")),
                json!(["attr", "Eq", "value"]),
            ),
            (RankByText::attribute("attr"), json!(["Attribute", "attr"])),
            (
                RankByText::saturate(
                    RankByText::bm25_with_params("attr", "string", Bm25ClauseParams::new()),
                    SaturateParams::new(),
                ),
                json!(["Saturate", ["attr", "BM25", "string", {}], {}]),
            ),
            (
                RankByText::decay(
                    RankByText::bm25_with_params("attr", "string", Bm25ClauseParams::new()),
                    DecayParams::new(),
                ),
                json!(["Decay", ["attr", "BM25", "string", {}], {}]),
            ),
            (
                RankByText::dist(
                    RankByText::bm25_with_params("attr", "string", Bm25ClauseParams::new()),
                    "value",
                ),
                json!(["Dist", ["attr", "BM25", "string", {}], "value"]),
            ),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: RankByText = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn filter_round_trips() {
        let cases: Vec<(Filter, Value)> = vec![
            (Filter::eq("attr", "value"), json!(["attr", "Eq", "value"])),
            (
                Filter::not_eq("attr", "value"),
                json!(["attr", "NotEq", "value"]),
            ),
            (
                Filter::in_("attr", ["value"]),
                json!(["attr", "In", ["value"]]),
            ),
            (
                Filter::not_in("attr", ["value"]),
                json!(["attr", "NotIn", ["value"]]),
            ),
            (
                Filter::contains("attr", "value"),
                json!(["attr", "Contains", "value"]),
            ),
            (
                Filter::not_contains("attr", "value"),
                json!(["attr", "NotContains", "value"]),
            ),
            (
                Filter::contains_any("attr", ["value"]),
                json!(["attr", "ContainsAny", ["value"]]),
            ),
            (
                Filter::not_contains_any("attr", ["value"]),
                json!(["attr", "NotContainsAny", ["value"]]),
            ),
            (Filter::lt("attr", "value"), json!(["attr", "Lt", "value"])),
            (
                Filter::lte("attr", "value"),
                json!(["attr", "Lte", "value"]),
            ),
            (Filter::gt("attr", "value"), json!(["attr", "Gt", "value"])),
            (
                Filter::gte("attr", "value"),
                json!(["attr", "Gte", "value"]),
            ),
            (
                Filter::any_lt("attr", "value"),
                json!(["attr", "AnyLt", "value"]),
            ),
            (
                Filter::any_lte("attr", "value"),
                json!(["attr", "AnyLte", "value"]),
            ),
            (
                Filter::any_gt("attr", "value"),
                json!(["attr", "AnyGt", "value"]),
            ),
            (
                Filter::any_gte("attr", "value"),
                json!(["attr", "AnyGte", "value"]),
            ),
            (
                Filter::glob("attr", "value"),
                json!(["attr", "Glob", "value"]),
            ),
            (
                Filter::not_glob("attr", "value"),
                json!(["attr", "NotGlob", "value"]),
            ),
            (
                Filter::iglob("attr", "value"),
                json!(["attr", "IGlob", "value"]),
            ),
            (
                Filter::not_iglob("attr", "value"),
                json!(["attr", "NotIGlob", "value"]),
            ),
            (
                Filter::regex("attr", "value"),
                json!(["attr", "Regex", "value"]),
            ),
            (
                Filter::fuzzy(
                    "attr",
                    "value",
                    FuzzyParams::new([FuzzyMaxEditDistance::new(4, 3)]),
                ),
                json!(["attr","Fuzzy","value",{"max_edit_distance":[{"distance":3,"min_query_chars":4}]}]),
            ),
            (
                Filter::contains_all_tokens_with_params(
                    "attr",
                    "string",
                    ContainsAllTokensFilterParams::new(),
                ),
                json!(["attr", "ContainsAllTokens", "string", {}]),
            ),
            (
                Filter::contains_all_tokens_with_params(
                    "attr",
                    ["string"],
                    ContainsAllTokensFilterParams::new(),
                ),
                json!(["attr", "ContainsAllTokens", ["string"], {}]),
            ),
            (
                Filter::contains_all_tokens("attr", "string"),
                json!(["attr", "ContainsAllTokens", "string"]),
            ),
            (
                Filter::contains_all_tokens("attr", ["string"]),
                json!(["attr", "ContainsAllTokens", ["string"]]),
            ),
            (
                Filter::contains_any_token_with_params(
                    "attr",
                    "string",
                    ContainsAnyTokenFilterParams::new(),
                ),
                json!(["attr", "ContainsAnyToken", "string", {}]),
            ),
            (
                Filter::contains_any_token_with_params(
                    "attr",
                    ["string"],
                    ContainsAnyTokenFilterParams::new(),
                ),
                json!(["attr", "ContainsAnyToken", ["string"], {}]),
            ),
            (
                Filter::contains_any_token("attr", "string"),
                json!(["attr", "ContainsAnyToken", "string"]),
            ),
            (
                Filter::contains_any_token("attr", ["string"]),
                json!(["attr", "ContainsAnyToken", ["string"]]),
            ),
            (
                Filter::contains_token_sequence("attr", "string"),
                json!(["attr", "ContainsTokenSequence", "string"]),
            ),
            (
                Filter::contains_token_sequence("attr", ["string"]),
                json!(["attr", "ContainsTokenSequence", ["string"]]),
            ),
            (
                Filter::not(Filter::eq("attr", "value")),
                json!(["Not", ["attr", "Eq", "value"]]),
            ),
            (
                Filter::and([Filter::eq("attr", "value")]),
                json!(["And", [["attr", "Eq", "value"]]]),
            ),
            (
                Filter::or([Filter::eq("attr", "value")]),
                json!(["Or", [["attr", "Eq", "value"]]]),
            ),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: Filter = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn group_by_round_trips() {
        let cases: Vec<(GroupBy, Value)> = vec![
            (GroupBy::attr("string"), json!("string")),
            (
                GroupBy::expr([("key", GroupByFunction::for_each_unique("attr"))]),
                json!({"key":["ForEachUnique","attr"]}),
            ),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: GroupBy = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn group_by_function_round_trips() {
        let cases: Vec<(GroupByFunction, Value)> = vec![(
            GroupByFunction::for_each_unique("attr"),
            json!(["ForEachUnique", "attr"]),
        )];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: GroupByFunction = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn rerank_by_round_trips() {
        let cases: Vec<(RerankBy, Value)> = vec![
            (
                RerankBy::rrf_with_params(RrfParams::new()),
                json!(["RRF", {}]),
            ),
            (RerankBy::rrf(), json!(["RRF"])),
            (
                RerankBy::model(RerankModelParams::new("model", "attribute", "query")),
                json!({"attribute":"attribute","model":"model","query":"query"}),
            ),
        ];
        for (value, expected) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(json, expected);
            let decoded: RerankBy = serde_json::from_value(json).unwrap();
            assert_eq!(decoded, value, "{expected}");
        }
    }
}
