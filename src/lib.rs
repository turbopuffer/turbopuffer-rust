#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

mod client;
mod error;
pub mod pagination;
pub mod resources;
mod row;
mod transport;
pub mod types;

pub use crate::client::{Client, ClientBuilder};
pub use crate::error::{ApiError, ApiErrorKind, Error, Result};
pub use crate::resources::namespaces::Namespace;

// Re-exported so that applications can use the exact versions this crate was built with, e.g. to configure a
// `reqwest::Client` for `ClientBuilder::http_client`, or to build JSON values with `serde_json::json!`.
pub use bytes;
pub use reqwest;
pub use serde_json;
