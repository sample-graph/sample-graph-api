use std::{env::var, error::Error, sync::Arc, time::Duration};

use axum::{error_handling::HandleErrorLayer, routing::get, BoxError, Router, Server};
use genius_rust::Genius;
use http::{Method, StatusCode};
use redis::Client;
use tower::{buffer::BufferLayer, limit::rate::RateLimitLayer, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::fmt;

use sample_graph_api::{
    routes::{graph, search, version},
    AppState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    fmt::init();

    let genius_client = Genius::new(var("GENIUS_KEY")?);
    let redis_client = Client::open(var("DATABASE_URL")?)?;
    let shared_state = Arc::new(AppState::new(
        genius_client,
        redis_client,
        var("REDIS_KEY_EXPIRY")?.parse::<usize>()?,
    ));

    let cors = CorsLayer::new()
        .allow_methods(Method::GET)
        .allow_origin(Any);
    let route_layers = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|err: BoxError| async move {
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
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
