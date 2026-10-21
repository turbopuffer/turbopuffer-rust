# turbopuffer Rust API library

<a href="https://crates.io/crates/turbopuffer"><img src="https://img.shields.io/crates/v/turbopuffer.svg" alt="crates.io"></a>
<a href="https://docs.rs/turbopuffer"><img src="https://docs.rs/turbopuffer/badge.svg" alt="docs.rs"></a>

The turbopuffer Rust library provides typed, async access to the [turbopuffer REST API](https://turbopuffer.com/docs/api-overview) from Rust applications. It's built on [tokio](https://tokio.rs) and [reqwest](https://docs.rs/reqwest).

## Documentation

The REST API documentation can be found at [turbopuffer.com/docs/api-overview](https://turbopuffer.com/docs/api-overview). The full API of this library can be found on [docs.rs](https://docs.rs/turbopuffer).

## Installation

```sh
cargo add turbopuffer
```

You'll also need an async runtime. Most applications use [tokio](https://tokio.rs):

```sh
cargo add tokio --features full
```

## Usage

```rust no_run
use turbopuffer::Client;
use turbopuffer::types::{DistanceMetric, Row};

#[tokio::main]
async fn main() -> Result<(), turbopuffer::Error> {
    let client = Client::builder()
        .region("gcp-us-central1")
        .build()?;

    let response = client
        .namespace("products")
        .write()
        .distance_metric(DistanceMetric::CosineDistance)
        .upsert_rows([Row::new("2108ed60-6851-49a0-9016-8325434f3845")
            .vector([0.1, 0.2])
            .insert("name", "Red boots")
            .insert("price", 34.99)])
        .send()
        .await?;
    println!("{:?}", response.rows_affected);

    Ok(())
}
```

The API key is read from the `TURBOPUFFER_API_KEY` environment variable by default. You can also set it with `.api_key(...)` on the builder, but we recommend keeping it out of source control by using an environment variable (or a `.env` file and a crate like [dotenvy](https://docs.rs/dotenvy)).

## Configuring the client

Settings that aren't set on the [`ClientBuilder`](https://docs.rs/turbopuffer/latest/turbopuffer/struct.ClientBuilder.html) are read from environment variables:

- `TURBOPUFFER_API_KEY`: API key used for authentication (`.api_key(...)`)
- `TURBOPUFFER_REGION`: The turbopuffer region to use (`.region(...)`)
- `TURBOPUFFER_BASE_URL`: overrides the base URL (`.base_url(...)`)

`Client::from_env()` creates a client using only environment variables. Clients are cheap to clone and share a connection pool, so create one and reuse it throughout your application.

## Making requests

Each API method returns a request builder. Required parameters are arguments to the method, and optional parameters are set with builder methods. Send the request with `.send().await`, or just `.await` it, since request builders implement [`IntoFuture`](https://doc.rust-lang.org/std/future/trait.IntoFuture.html).

Methods that operate on a single namespace are called on a handle returned by `client.namespace(...)`. Handles are cheap to create and don't make any requests on their own.

Request parameters accept anything convertible into the parameter type, so you can pass `&str` for strings, arrays or iterators for lists, and the variants' inner values for union types.

Response types are plain structs with public fields. Structs and enums are marked `#[non_exhaustive]`, so that new fields and variants can be added in minor versions: construct request types with their `new` function and setters, and include a wildcard arm when matching on enums. Enums representing a fixed set of strings have an `Other` variant holding values introduced to the API after this version of the library.

## Typed parameters

Some parameters take structured values, like [`AggregateBy`](https://docs.rs/turbopuffer/latest/turbopuffer/types/enum.AggregateBy.html), [`Expr`](https://docs.rs/turbopuffer/latest/turbopuffer/types/enum.Expr.html), [`RankBy`](https://docs.rs/turbopuffer/latest/turbopuffer/types/enum.RankBy.html), [`Filter`](https://docs.rs/turbopuffer/latest/turbopuffer/types/enum.Filter.html), [`GroupBy`](https://docs.rs/turbopuffer/latest/turbopuffer/types/enum.GroupBy.html), [`RerankBy`](https://docs.rs/turbopuffer/latest/turbopuffer/types/enum.RerankBy.html). Build them with their associated functions, which accept anything convertible into each argument:

```rust no_run
use turbopuffer::Client;
use turbopuffer::types::{Filter, RankBy};

#[tokio::main]
async fn main() -> Result<(), turbopuffer::Error> {
    let client = Client::builder()
        .region("gcp-us-central1")
        .build()?;

    let response = client
        .namespace("example")
        .query()
        .filters(Filter::eq("attr", "value"))
        .rank_by(RankBy::ann("attr", [0.1, 0.2, 0.3]))
        .send()
        .await?;
    println!("{response:?}");

    Ok(())
}
```

They serialize to the JSON the API expects, and can be deserialized from it too.

## Pagination

List methods are paginated. Use `into_stream` to iterate over every item, fetching more pages as needed:

```rust no_run
use futures::TryStreamExt;
use turbopuffer::Client;

#[tokio::main]
async fn main() -> Result<(), turbopuffer::Error> {
    let client = Client::builder()
        .region("gcp-us-central1")
        .build()?;

    let mut stream = client.namespaces().prefix("products").into_stream();
    while let Some(item) = stream.try_next().await? {
        println!("{item:?}");
    }

    Ok(())
}
```

Or fetch one page at a time with `send`, then `next_page`:

```rust no_run
use turbopuffer::Client;

#[tokio::main]
async fn main() -> Result<(), turbopuffer::Error> {
    let client = Client::builder()
        .region("gcp-us-central1")
        .build()?;

    let page = client.namespaces().prefix("products").send().await?;
    for item in page.items() {
        println!("{item:?}");
    }
    if let Some(next) = page.next_page().await? {
        println!("next page has {} items", next.items().len());
    }

    Ok(())
}
```

## Handling errors

Methods return [`turbopuffer::Result`](https://docs.rs/turbopuffer/latest/turbopuffer/type.Result.html). When the API responds with a non-success status code, the error is `Error::Api`, with the status code, message, and response body. Other variants cover timeouts, connection errors, responses that couldn't be decoded, and `Error::Json` for your own conversions to or from JSON, which `serde_json::Error` converts into with `?`.

```rust no_run
use turbopuffer::ApiErrorKind;
use turbopuffer::Client;
use turbopuffer::Error;

#[tokio::main]
async fn main() -> Result<(), turbopuffer::Error> {
    let client = Client::builder()
        .region("gcp-us-central1")
        .build()?;

    let result = client.namespaces().prefix("foo").send().await;
    match result {
        Ok(response) => println!("{response:?}"),
        Err(Error::Api(err)) if err.kind() == ApiErrorKind::NotFound => {
            println!("not found: {}", err.message)
        }
        Err(Error::Api(err)) => println!("API error {}: {}", err.status, err.message),
        Err(err) => return Err(err),
    }

    Ok(())
}
```

## Retries

Connection errors, timeouts, and responses with status 408, 409, 429, or 5xx are retried 4 times by default, with exponential backoff that honors the server's `Retry-After` header. Configure this for every request with `ClientBuilder::max_retries`, or for a single request with the request builder's `max_retries` method:

```rust no_run
use std::time::Duration;

use turbopuffer::Client;

fn configure() -> Result<Client, turbopuffer::Error> {
    Client::builder()
        .max_retries(0) // disable retries
        .timeout(Duration::from_secs(20))
        .build()
}
```

## Timeouts

Requests time out after 60 seconds by default. Each retry gets a fresh timeout. Configure it with `ClientBuilder::timeout`, or per request with the request builder's `timeout` method.

## TLS

TLS is provided by [rustls](https://docs.rs/rustls) by default. To use your platform's native TLS implementation instead, disable default features and enable `native-tls`:

```sh
cargo add turbopuffer --no-default-features --features native-tls
```

## Client telemetry

The client can report its own request timings to turbopuffer to help diagnose latency between your application and the API. Telemetry is disabled by default. To enable it, set the `TURBOPUFFER_TELEMETRY` environment variable to `1`, or use `ClientBuilder::telemetry`:

```rust no_run
use turbopuffer::Client;

fn configure() -> Result<Client, Box<dyn std::error::Error>> {
    Ok(Client::builder().telemetry(true).build()?)
}
```

`.telemetry(false)` disables telemetry even when the environment variable is set.

When enabled, timings for completed requests are attached to subsequent requests in the `X-turbopuffer-Client-Telemetry` header, so no additional requests are made. Each record contains the request's method and path with the namespace name removed, its status code, retry count, whether it reused an existing connection, and client-side durations (serialization, waiting for a free connection, connecting, sending, waiting for the response, reading the response body, and deserialization). Connections and sending are only timed with the client's own HTTP client, not one passed to `ClientBuilder::http_client`. Request and response bodies and API keys are never included. turbopuffer may ask the client to record only a sample of requests.

## Requirements

Rust 1.85 or later.
