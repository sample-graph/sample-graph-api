use std::{env::var, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    error_handling::HandleErrorLayer, http::StatusCode, routing::get, BoxError, Router, Server,
};
use genius_rust::Genius;
use redis::Client;
use tower::{buffer::BufferLayer, limit::rate::RateLimitLayer, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::fmt;

use sample_graph_api::{
    routes::{graph, search, version},
    structs::AppState,
};

static GENIUS_API_KEY_NAME: &str = "GENIUS_KEY";
static REDIS_ADDR_NAME: &str = "REDIS_ADDR";
static HOST_ADDR: &str = "0.0.0.0:8000";
static REDIS_LOCAL_ADDR: &str = "redis://127.0.0.1:6379";

#[tokio::main]
async fn main() -> Result<()> {
    fmt::init();

    let genius_client =
        Genius::new(var(GENIUS_API_KEY_NAME).context("Failed to fetch Genius API key")?);
    let redis_client = Client::open(var(REDIS_ADDR_NAME).unwrap_or(REDIS_LOCAL_ADDR.to_string()))
        .context("Failed to find Redis client")?;
    let shared_state = Arc::new(AppState::new(genius_client, redis_client));

    let route_layers = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|err: BoxError| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled error: {}", err),
            )
        }))
        .layer(BufferLayer::new(1024))
        .layer(RateLimitLayer::new(20, Duration::from_secs(60)))
        .layer(TraceLayer::new_for_http());
    let router = Router::new()
        .route("/search", get(search))
        .route("/graph/:song_id", get(graph))
        .route("/version", get(version))
        .layer(route_layers)
        .with_state(shared_state);
    Server::bind(&HOST_ADDR.parse()?)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
