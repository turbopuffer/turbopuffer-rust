use serde_json::json;
use turbopuffer::Client;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn per_request_authorization_survives_async_polling() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/dest"))
        .and(header("authorization", "Bearer scoped-key"))
        .and(header("x-proxy-token", "request-token"))
        .respond_with(
            ResponseTemplate::new(202)
                .insert_header("preference-applied", "respond-async")
                .insert_header("location", "/v1/namespaces/dest/operations/job-1")
                .set_body_json(json!({"token":"job-1"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/namespaces/dest/operations/job-1"))
        .and(header("authorization", "Bearer scoped-key"))
        .and(header("x-proxy-token", "request-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"status":"finished","result":{"success":{"status":"OK","message":"copied","rows_affected":0,"billing":{"billable_logical_bytes_written":0}}}}),
        ))
        .mount(&server)
        .await;
    let client = Client::builder()
        .api_key("default-key")
        .base_url(server.uri())
        .max_retries(0)
        .build()
        .unwrap();
    let result = client
        .namespace("dest")
        .copy_from("source")
        .header("authorization", "Bearer scoped-key")
        .header("x-proxy-token", "request-token")
        .header("prefer", "respond-async")
        .send()
        .await;
    assert!(
        result.is_ok(),
        "copy started with scoped key but polling lost authorization: {result:?}"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(!requests[1].headers.contains_key("prefer"));
    assert!(requests[1].headers.contains_key("x-stainless-lang"));
}
