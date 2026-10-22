//! Decoding of a namespace's encryption in metadata responses. This file is custom code: the generator doesn't
//! produce it.
//!
//! `update_metadata()` calls `PATCH /v1/namespaces/{namespace}/metadata`, which responds with the v1 shape of a
//! namespace's encryption, `{"sse": true}` or `{"cmek": {"key_name": "..."}}`, rather than the spec's v2 shape,
//! `{"mode": "default"}` or `{"mode": "customer-managed", "key_name": "..."}`. The v1 shapes decode to the same
//! [`Encryption`] variants as their v2 equivalents, and everything else decodes as generated.
//!
//! TODO: Remove this once the API has a v2 `PATCH .../metadata` that responds with the v2 shape, and the spec
//! points `update_metadata()` at it.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::types::{Encryption, EncryptionCustomerManaged};

/// Decodes a namespace's encryption from either its v1 or its v2 shape.
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Encryption, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if let Some(encryption) = decode_v1(&value) {
        return Ok(encryption);
    }
    Encryption::deserialize(value).map_err(serde::de::Error::custom)
}

/// Decodes the v1 shape of an encryption, or returns `None` for anything else.
fn decode_v1(value: &Value) -> Option<Encryption> {
    let object = value.as_object()?;
    if object.contains_key("mode") {
        return None;
    }
    if object.get("sse") == Some(&Value::Bool(true)) {
        return Some(Encryption::Default);
    }
    let key_name = object.get("cmek")?.get("key_name")?.as_str()?;
    Some(EncryptionCustomerManaged::new(key_name).into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::Client;
    use crate::types::NamespaceMetadata;

    fn metadata(encryption: Value) -> Value {
        json!({
            "approx_logical_bytes": 0,
            "approx_row_count": 0,
            "created_at": "2026-10-22T00:00:00Z",
            "encryption": encryption,
            "index": { "status": "up-to-date" },
            "schema": {},
            "updated_at": "2026-10-22T00:00:00Z",
        })
    }

    fn decode(encryption: Value) -> Result<Encryption, serde_json::Error> {
        serde_json::from_value::<NamespaceMetadata>(metadata(encryption)).map(|m| m.encryption)
    }

    #[test]
    fn decodes_v1_encryption() {
        assert_eq!(decode(json!({ "sse": true })).unwrap(), Encryption::Default);
        assert_eq!(
            decode(json!({ "cmek": { "key_name": "key" } })).unwrap(),
            EncryptionCustomerManaged::new("key").into()
        );
    }

    #[test]
    fn decodes_v2_encryption() {
        assert_eq!(
            decode(json!({ "mode": "default" })).unwrap(),
            Encryption::Default
        );
        assert_eq!(
            decode(json!({ "mode": "customer-managed", "key_name": "key" })).unwrap(),
            EncryptionCustomerManaged::new("key").into()
        );
        assert_eq!(
            decode(json!({ "mode": "ekm", "key_name": "key" })).unwrap(),
            Encryption::Other
        );
    }

    #[test]
    fn rejects_other_shapes() {
        let error = decode(json!({})).unwrap_err();
        assert!(
            error.to_string().contains("missing field `mode`"),
            "{error}"
        );
        let error = decode(json!({ "sse": false })).unwrap_err();
        assert!(
            error.to_string().contains("missing field `mode`"),
            "{error}"
        );
        let error = decode(json!({ "cmek": {} })).unwrap_err();
        assert!(
            error.to_string().contains("missing field `mode`"),
            "{error}"
        );
        let error = decode(json!({ "mode": "customer-managed" })).unwrap_err();
        assert!(
            error.to_string().contains("missing field `key_name`"),
            "{error}"
        );
        let error = decode(json!("default")).unwrap_err();
        assert!(
            error.to_string().contains("internally tagged enum"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn update_metadata_decodes_v1_encryption() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/namespaces/ns/metadata"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(metadata(json!({ "sse": true }))),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::builder()
            .api_key("test-api-key")
            .base_url(server.uri())
            .max_retries(0)
            .build()
            .unwrap();

        let metadata = client
            .namespace("ns")
            .update_metadata()
            .read_only(false)
            .send()
            .await
            .unwrap();
        assert_eq!(metadata.encryption, Encryption::Default);
    }
}
