//! Request and response types.
//!
//! Structs are `#[non_exhaustive]` so that new fields can be added without breaking changes. Construct
//! request types with their `new` function and chainable setters.

mod namespaces;
mod shared;

pub use self::namespaces::*;
pub use self::shared::*;

/// Returns the elements of `value` if it's an array of length `len` with the given string constants at the
/// given positions.
pub(crate) fn tuple_elements<'a>(
    value: &'a serde_json::Value,
    len: usize,
    consts: &[(usize, &str)],
) -> Option<&'a [serde_json::Value]> {
    let elements = value.as_array()?;
    let matches = elements.len() == len
        && consts
            .iter()
            .all(|(index, constant)| elements[*index].as_str() == Some(*constant));
    matches.then_some(elements.as_slice())
}

/// Decodes a tuple element, returning `None` if it doesn't have the expected type.
pub(crate) fn decode_element<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
) -> Option<T> {
    T::deserialize(value).ok()
}

/// Converts an expression to JSON, for its `From` conversion into `serde_json::Value`. This can't fail, since
/// expressions only hold values that serialize to JSON, and maps with string keys.
pub(crate) fn expression_to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).expect("expressions serialize to JSON")
}
