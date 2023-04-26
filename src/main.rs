use std::{env::var, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    error_handling::HandleErrorLayer, http::StatusCode, routing::get, BoxError, Router, Server,
};
use genius_rust::Genius;
use http::Method;
use redis::Client;
use tower::{buffer::BufferLayer, limit::rate::RateLimitLayer, ServiceBuilder};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use tracing_subscriber::fmt;

use sample_graph_api::{
    routes::{graph, search, version},
    structs::AppState,
};

#[tokio::main]
async fn main() -> Result<()> {
    fmt::init();

    let genius_client =
        Genius::new(var("GENIUS_KEY").context("Failed to fetch Genius API key")?);
    let redis_client = Client::open(var("DATABASE_URL")?)
        .context("Failed to find Redis client")?;
    let shared_state = Arc::new(AppState::new(genius_client, redis_client));
    
    let cors = CorsLayer::new().allow_methods(Method::GET).allow_origin(Any);
    let route_layers = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|err: BoxError| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled error: {}", err),
            )
        }))
        .layer(BufferLayer::new(1024))
        .layer(RateLimitLayer::new(20, Duration::from_secs(60)))
        .layer(TraceLayer::new_for_http())
        .layer(cors);
    let router = Router::new()
        .route("/search", get(search))
        .route("/graph/:song_id", get(graph))
        .route("/version", get(version))
        .layer(route_layers)
        .with_state(shared_state);
    Server::bind(&"0.0.0.0:8000".parse()?)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
