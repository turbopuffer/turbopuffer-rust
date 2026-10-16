//! Tests that each API method sends the request the API expects and decodes its response, using sample values
//! for every parameter.

use serde_json::json;
use turbopuffer::Client;
use turbopuffer::types::{
    AggregateBy, Columns, DistanceMetric, EncryptionCustomerManaged, Expr, Filter, GroupBy,
    PatchByFilter, Query, QueryConfigConsistency, RankBy, RerankBy, RerankModelParams, Row,
    ShardingConfig, VectorEncoding,
};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A client for the mock server, which doesn't retry so that failures surface immediately.
fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .max_retries(0)
        .build()
        .unwrap()
}

/// The requests a mock server received, for failure messages.
async fn received(server: &MockServer) -> String {
    let requests = server.received_requests().await.unwrap_or_default();
    requests
        .iter()
        .map(|r| {
            format!(
                "{} {} {}",
                r.method,
                r.url,
                String::from_utf8_lossy(&r.body)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn client_namespaces() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/namespaces"))
        .and(query_param("cursor", "string"))
        .and(query_param("page_size", "3"))
        .and(query_param("prefix", "string"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"namespaces":[{"id":"string"}],"next_cursor":null})),
        )
        .mount(&server)
        .await;

    let result = client(&server)
        .namespaces()
        .cursor("string")
        .page_size(3)
        .prefix("string")
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    assert_eq!(response.items().len(), 1);
    assert!(!response.has_next_page());
}

#[tokio::test]
async fn namespaces_branch_from() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace"))
        .and(body_json(json!({"branch_from_namespace":{"source_namespace":"string"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"billing":{"billable_logical_bytes_written":3,"query":{"billable_logical_bytes_queried":3,"billable_logical_bytes_returned":3}},"message":"string","rows_affected":3,"deleted_ids":[3],"patched_ids":[3],"performance":{"server_total_ms":3,"embedding_ms":3,"embedding_tokens":3},"rows_deleted":3,"rows_patched":3,"rows_remaining":true,"rows_upserted":3,"upserted_ids":[3]})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .branch_from("string")
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_copy_from() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace"))
        .and(body_json(json!({"copy_from_namespace":{"source_namespace":"string","dest_encryption":{"mode":"customer-managed","key_name":"string"},"source_api_key":"string","source_region":"string"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"billing":{"billable_logical_bytes_written":3,"query":{"billable_logical_bytes_queried":3,"billable_logical_bytes_returned":3}},"message":"string","rows_affected":3,"deleted_ids":[3],"patched_ids":[3],"performance":{"server_total_ms":3,"embedding_ms":3,"embedding_tokens":3},"rows_deleted":3,"rows_patched":3,"rows_remaining":true,"rows_upserted":3,"upserted_ids":[3]})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .copy_from("string")
        .dest_encryption(EncryptionCustomerManaged::new("string"))
        .source_api_key("string")
        .source_region("string")
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_delete_all() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v2/namespaces/namespace"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .delete_all()
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_explain_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace/explain_query"))
        .and(body_json(json!({"aggregate_by":{"key":["Count"]},"compute_attributes":{"key":["Embed","string"]},"consistency":{},"distance_metric":"cosine_distance","exclude_attributes":["string"],"filters":["string","Eq","value"],"group_by":["string"],"include_attributes":true,"limit":3,"offset":3,"rank_by":["string","ANN",[0.5]],"search_effort":0.5,"top_k":3,"vector_encoding":"float"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"plan_text":"string"})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .explain_query()
        .aggregate_by([("key", AggregateBy::Count)])
        .compute_attributes([("key", Expr::embed("string"))])
        .consistency(QueryConfigConsistency::new())
        .distance_metric(DistanceMetric::CosineDistance)
        .exclude_attributes(["string"])
        .filters(Filter::eq("string", "value"))
        .group_by([GroupBy::attr("string")])
        .include_attributes(true)
        .limit(3)
        .offset(3)
        .rank_by(RankBy::ann("string", [0.5]))
        .search_effort(0.5)
        .top_k(3)
        .vector_encoding(VectorEncoding::Float)
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_hint_cache_warm() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/namespaces/namespace/hint_cache_warm"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"message":"string"})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .hint_cache_warm()
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/namespaces/namespace/metadata"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"approx_logical_bytes":3,"approx_row_count":3,"created_at":"2024-01-01T00:00:00Z","encryption":{"mode":"customer-managed","key_name":"string"},"index":{"status":"up-to-date"},"schema":{"key":{"type":"string"}},"updated_at":"2024-01-01T00:00:00Z","pinning":{"replicas":3,"status":{"ready_replicas":3,"replicas":3,"updated_at":"2024-01-01T00:00:00Z","utilization":0.5}},"read_only":true,"sharding":{"num_shards":3}})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .metadata()
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_multi_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace/query"))
        .and(body_json(json!({"queries":[{}],"consistency":{},"limit":3,"offset":3,"rerank_by":["RRF"],"vector_encoding":"float"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"billing":{"billable_logical_bytes_queried":3,"billable_logical_bytes_returned":3},"performance":{"approx_namespace_size":3,"cache_hit_ratio":0.5,"cache_temperature":"string","exhaustive_search_count":3,"query_execution_ms":3,"server_total_ms":3,"embedding_ms":3,"embedding_tokens":3},"results":[{}]})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .multi_query([Query::new()])
        .consistency(QueryConfigConsistency::new())
        .limit(3)
        .offset(3)
        .rerank_by(RerankBy::Rrf)
        .vector_encoding(VectorEncoding::Float)
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_poll_copy_from() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/namespaces/namespace/operations/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"status":"running","start_time":"2024-01-01T00:00:00Z","progress":"string"}),
        ))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .poll_copy_from("token")
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace/query"))
        .and(body_json(json!({"aggregate_by":{"key":["Count"]},"compute_attributes":{"key":["Embed","string"]},"consistency":{},"distance_metric":"cosine_distance","exclude_attributes":["string"],"filters":["string","Eq","value"],"group_by":["string"],"include_attributes":true,"limit":3,"offset":3,"rank_by":["string","ANN",[0.5]],"rerank_by":{"attribute":"string","model":"string","query":"string"},"search_effort":0.5,"top_k":3,"vector_encoding":"float"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"billing":{"billable_logical_bytes_queried":3,"billable_logical_bytes_returned":3},"performance":{"approx_namespace_size":3,"cache_hit_ratio":0.5,"cache_temperature":"string","exhaustive_search_count":3,"query_execution_ms":3,"server_total_ms":3,"embedding_ms":3,"embedding_tokens":3},"aggregation_groups":[{"key":"value"}],"aggregations":{"key":"value"},"rows":[{"id":3}]})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .query()
        .aggregate_by([("key", AggregateBy::Count)])
        .compute_attributes([("key", Expr::embed("string"))])
        .consistency(QueryConfigConsistency::new())
        .distance_metric(DistanceMetric::CosineDistance)
        .exclude_attributes(["string"])
        .filters(Filter::eq("string", "value"))
        .group_by([GroupBy::attr("string")])
        .include_attributes(true)
        .limit(3)
        .offset(3)
        .rank_by(RankBy::ann("string", [0.5]))
        .rerank_by(RerankModelParams::new("string", "string", "string"))
        .search_effort(0.5)
        .top_k(3)
        .vector_encoding(VectorEncoding::Float)
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_recall() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/namespaces/namespace/_debug/recall"))
        .and(body_json(json!({"filters":["string","Eq","value"],"include_ground_truth":true,"num":3,"rank_by":["string","ANN",[0.5]],"top_k":3})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"avg_ann_count":0.5,"avg_exhaustive_count":0.5,"avg_recall":0.5})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .recall()
        .filters(Filter::eq("string", "value"))
        .include_ground_truth(true)
        .num(3)
        .rank_by(RankBy::ann("string", [0.5]))
        .top_k(3)
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_schema() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/namespaces/namespace/schema"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"key":{"type":"string","ann":true,"embed":"string","filterable":true,"full_text_search":true,"fuzzy":true,"glob":true,"regex":true,"sparse_knn":{"distance_metric":"dot_product"}}})))
        .mount(&server)
        .await;

    let result = client(&server).namespace("namespace").schema().send().await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_start_copy_from() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace/async"))
        .and(body_json(json!({"copy_from_namespace":{"source_namespace":"string","dest_encryption":{"mode":"customer-managed","key_name":"string"},"source_api_key":"string","source_region":"string"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token":"string"})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .start_copy_from("string")
        .dest_encryption(EncryptionCustomerManaged::new("string"))
        .source_api_key("string")
        .source_region("string")
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_update_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/namespaces/namespace/metadata"))
        .and(body_json(json!({"pinning":true,"read_only":true})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"approx_logical_bytes":3,"approx_row_count":3,"created_at":"2024-01-01T00:00:00Z","encryption":{"mode":"customer-managed","key_name":"string"},"index":{"status":"up-to-date"},"schema":{"key":{"type":"string"}},"updated_at":"2024-01-01T00:00:00Z","pinning":{"replicas":3,"status":{"ready_replicas":3,"replicas":3,"updated_at":"2024-01-01T00:00:00Z","utilization":0.5}},"read_only":true,"sharding":{"num_shards":3}})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .update_metadata()
        .pinning(true)
        .read_only(true)
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_update_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/namespaces/namespace/schema"))
        .and(body_json(json!({"key":"string"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"key":{"type":"string","ann":true,"embed":"string","filterable":true,"full_text_search":true,"fuzzy":true,"glob":true,"regex":true,"sparse_knn":{"distance_metric":"dot_product"}}})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .update_schema([("key", "string")])
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}

#[tokio::test]
async fn namespaces_write() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/namespaces/namespace"))
        .and(body_json(json!({"branch_from_namespace":"string","copy_from_namespace":"string","create_namespace":true,"delete_by_filter":["string","Eq","value"],"delete_by_filter_allow_partial":true,"delete_condition":["string","Eq","value"],"deletes":[3],"disable_backpressure":true,"distance_metric":"cosine_distance","encryption":{"mode":"customer-managed","key_name":"string"},"patch_by_filter":{"filters":["string","Eq","value"],"patch":{"key":"value"}},"patch_by_filter_allow_partial":true,"patch_columns":{"id":[3]},"patch_condition":["string","Eq","value"],"patch_rows":[{"id":3}],"return_affected_ids":true,"schema":{"key":"string"},"sharding":{"num_shards":3},"upsert_columns":{"id":[3]},"upsert_condition":["string","Eq","value"],"upsert_rows":[{"id":3}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"billing":{"billable_logical_bytes_written":3,"query":{"billable_logical_bytes_queried":3,"billable_logical_bytes_returned":3}},"message":"string","rows_affected":3,"deleted_ids":[3],"patched_ids":[3],"performance":{"server_total_ms":3,"embedding_ms":3,"embedding_tokens":3},"rows_deleted":3,"rows_patched":3,"rows_remaining":true,"rows_upserted":3,"upserted_ids":[3]})))
        .mount(&server)
        .await;

    let result = client(&server)
        .namespace("namespace")
        .write()
        .branch_from_namespace("string")
        .copy_from_namespace("string")
        .create_namespace(true)
        .delete_by_filter(Filter::eq("string", "value"))
        .delete_by_filter_allow_partial(true)
        .delete_condition(Filter::eq("string", "value"))
        .deletes([3])
        .disable_backpressure(true)
        .distance_metric(DistanceMetric::CosineDistance)
        .encryption(EncryptionCustomerManaged::new("string"))
        .patch_by_filter(PatchByFilter::new(
            Filter::eq("string", "value"),
            [("key", "value")],
        ))
        .patch_by_filter_allow_partial(true)
        .patch_columns(Columns::new([3]))
        .patch_condition(Filter::eq("string", "value"))
        .patch_rows([Row::new(3)])
        .return_affected_ids(true)
        .schema([("key", "string")])
        .sharding(ShardingConfig::new(3))
        .upsert_columns(Columns::new([3]))
        .upsert_condition(Filter::eq("string", "value"))
        .upsert_rows([Row::new(3)])
        .send()
        .await;
    let response = match result {
        Ok(response) => response,
        Err(err) => panic!("{err}\nrequests received:\n{}", received(&server).await),
    };
    let _ = response;
}
