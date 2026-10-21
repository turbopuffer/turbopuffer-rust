#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

mod client;
mod error;
mod observer;
pub mod pagination;
pub mod resources;
mod transport;
pub mod types;

pub use crate::client::{Client, ClientBuilder};
pub use crate::error::{ApiError, ApiErrorKind, BuildError, Error, Result};
pub use crate::resources::namespaces::Namespace;

// Re-exported so that applications can use the exact versions this crate was built with, e.g. to configure a
// `reqwest::Client` for `ClientBuilder::http_client`, to build JSON values with `serde_json::json!`, or to
// pass datetimes as free-form values.
pub use jiff;
pub use reqwest;
pub use serde_json;
