//! Base64 encoding of vectors. This file is custom code: the generator doesn't produce it.
//!
//! Writes send each vector as a base64 string of its elements as little-endian `f32`s instead of as a JSON
//! array of numbers, like the Python SDK. That's less than half the size, and faster for the API to parse. The
//! API takes this encoding for every vector type, including `f16` and `i8` ones.
//!
//! Reads aren't decoded: query results have arrays of numbers unless `vector_encoding` is `base64`, in which
//! case the API encodes the elements in the vector's own type rather than as `f32`s.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::types::{Ann, AttributeSchema, Columns, ColumnsVector, Row, Vector};

/// Base64-encodes the vectors that a write upserts: the `vector` attribute, and any other attribute that the
/// write's `schema` gives an ANN index, except for vector arrays (`[]` types), which don't take base64 yet.
pub(crate) fn encode_upserts(
    schema: Option<&BTreeMap<String, AttributeSchema>>,
    rows: Option<&mut Vec<Row>>,
    columns: Option<&mut Columns>,
) {
    let mut attributes = vec!["vector"];
    attributes.extend(
        schema
            .into_iter()
            .flatten()
            .filter(|(_, schema)| is_vector(schema))
            .map(|(name, _)| name.as_str()),
    );

    for row in rows.into_iter().flatten() {
        if let Some(vector) = &mut row.vector {
            encode_vector(vector);
        }
        for (name, value) in &mut row.extra {
            if attributes.contains(&name.as_str()) {
                encode_value(value);
            }
        }
    }

    if let Some(columns) = columns {
        // A single vector for the whole column has every document's elements back to back, which the API only
        // takes as an array of numbers.
        if let Some(ColumnsVector::Array(vectors)) = &mut columns.vector {
            vectors.iter_mut().for_each(encode_vector);
        }
        for (name, values) in &mut columns.extra {
            if attributes.contains(&name.as_str()) {
                values.iter_mut().for_each(encode_value);
            }
        }
    }
}

fn is_vector(schema: &AttributeSchema) -> bool {
    let AttributeSchema::Config(config) = schema else {
        return false;
    };
    !config.r#type.starts_with("[]")
        && config
            .ann
            .as_ref()
            .is_some_and(|ann| !matches!(ann, Ann::Bool(false)))
}

fn encode_vector(vector: &mut Vector) {
    if let Vector::Array(elements) = vector {
        if !elements.is_empty() {
            *vector = Vector::String(encode(elements));
        }
    }
}

/// Encodes a value that's an array of numbers, leaving anything else, like `null`, as it is.
fn encode_value(value: &mut Value) {
    let Value::Array(elements) = value else {
        return;
    };
    if elements.is_empty() {
        return;
    }
    let elements: Option<Vec<f32>> = elements
        .iter()
        .map(|element| element.as_f64().map(|element| element as f32))
        .collect();
    if let Some(elements) = elements {
        *value = Value::String(encode(&elements));
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes elements as standard, padded base64 of their little-endian bytes.
fn encode(elements: &[f32]) -> String {
    let bytes: Vec<u8> = elements
        .iter()
        .flat_map(|element| element.to_le_bytes())
        .collect();
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let byte = |i: usize| u32::from(chunk.get(i).copied().unwrap_or(0));
        let group = (byte(0) << 16) | (byte(1) << 8) | byte(2);
        for i in 0..4 {
            if i <= chunk.len() {
                encoded.push(char::from(
                    ALPHABET[((group >> (18 - 6 * i)) & 63) as usize],
                ));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::Client;
    use crate::types::AttributeSchemaConfig;

    #[test]
    fn encodes_base64() {
        // Same as the Python SDK's `b64encode_vector`.
        assert_eq!(encode(&[0.5, -1.25, 3.0]), "AAAAPwAAoL8AAEBA");
        assert_eq!(encode(&[1.0]), "AACAPw==");
        assert_eq!(encode(&[1.0, 2.0]), "AACAPwAAAEA=");
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn encodes_upserted_rows() {
        let schema = BTreeMap::from([
            (
                "image".to_owned(),
                AttributeSchemaConfig::new("[2]f16").ann(true).into(),
            ),
            (
                "ignored".to_owned(),
                AttributeSchemaConfig::new("[2]f32").ann(false).into(),
            ),
            (
                "tags".to_owned(),
                AttributeSchemaConfig::new("[][2]f32").ann(true).into(),
            ),
            ("title".to_owned(), "string".into()),
        ]);
        let mut rows = vec![
            Row::new(1)
                .vector([1.0, 2.0])
                .insert("image", [5, -12, 100])
                .insert("ignored", [1.0, 2.0])
                .insert("tags", [[1.0, 2.0]])
                .insert("title", "Overnight oats"),
            Row::new(2)
                .vector(Vec::<f32>::new())
                .insert("image", Value::Null),
            Row::new(3).vector("AACAPwAAAEA="),
        ];
        encode_upserts(Some(&schema), Some(&mut rows), None);

        assert_eq!(
            serde_json::to_value(&rows).unwrap(),
            json!([
                {
                    "id": 1,
                    "vector": "AACAPwAAAEA=",
                    "image": "AACgQAAAQMEAAMhC",
                    "ignored": [1.0, 2.0],
                    "tags": [[1.0, 2.0]],
                    "title": "Overnight oats",
                },
                { "id": 2, "vector": [], "image": null },
                { "id": 3, "vector": "AACAPwAAAEA=" },
            ])
        );
    }

    #[test]
    fn encodes_upserted_columns() {
        let schema = BTreeMap::from([(
            "image".to_owned(),
            AttributeSchemaConfig::new("[2]f32").ann(true).into(),
        )]);
        let mut columns = Columns::new([1, 2])
            .vector(vec![vec![1.0, 2.0], vec![3.0, 4.0]])
            .insert("image", [json!([1.0, 2.0]), Value::Null])
            .insert("views", [1, 2]);
        encode_upserts(Some(&schema), None, Some(&mut columns));
        assert_eq!(
            serde_json::to_value(&columns).unwrap(),
            json!({
                "id": [1, 2],
                "vector": ["AACAPwAAAEA=", "AABAQAAAgEA="],
                "image": ["AACAPwAAAEA=", null],
                "views": [1, 2],
            })
        );

        // A single vector for the whole column is left as numbers.
        let mut columns = Columns::new([1, 2]).vector(Vector::from([1.0, 2.0, 3.0, 4.0]));
        encode_upserts(None, None, Some(&mut columns));
        assert_eq!(
            serde_json::to_value(&columns).unwrap(),
            json!({ "id": [1, 2], "vector": [1.0, 2.0, 3.0, 4.0] })
        );
    }

    #[tokio::test]
    async fn writes_send_base64_vectors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/namespaces/ns"))
            .and(body_json(json!({
                "upsert_rows": [{ "id": 1, "vector": "AACAPwAAAEA=" }],
                "patch_rows": [{ "id": 2, "vector": [1.0, 2.0] }],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "billing": {
                    "billable_logical_bytes_written": 0,
                    "query": { "billable_logical_bytes_queried": 0, "billable_logical_bytes_returned": 0 },
                },
                "message": "OK",
                "rows_affected": 1,
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::builder()
            .api_key("test-api-key")
            .base_url(server.uri())
            .max_retries(0)
            .build()
            .unwrap();

        client
            .namespace("ns")
            .write()
            .upsert_rows([Row::new(1).vector([1.0, 2.0])])
            .patch_rows([Row::new(2).vector([1.0, 2.0])])
            .send()
            .await
            .unwrap();
    }
}
