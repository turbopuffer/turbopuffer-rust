//! Request and response types.
//!
//! Structs are `#[non_exhaustive]` so that new fields can be added without breaking changes. Construct
//! request types with their `new` function and chainable setters. Setters for lists and maps add to what
//! they already hold, so calling one again adds more items rather than replacing them.
//!
//! Free-form values, like a filter's value or a row's attributes, take anything that implements
//! `serde::Serialize`: numbers, strings, `serde_json::Value`s, `chrono` datetimes, and your own types. They
//! panic if a value fails to serialize, like `serde_json::json!` does, which only happens for maps with keys
//! that aren't strings or numbers, or `Serialize` implementations that return errors.

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

/// Converts a free-form value to JSON. Like `serde_json::json!`, this panics if the value doesn't serialize,
/// which only happens for maps with keys that aren't strings or numbers, or `Serialize` implementations that
/// return errors. Expressions and the types in this module always serialize.
#[allow(dead_code)] // Unused by APIs without free-form values.
pub(crate) fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).expect("value doesn't serialize to JSON")
}
