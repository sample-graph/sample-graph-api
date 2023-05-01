//! Functions for API routes.

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State as AxumState},
    response::Json,
};
use http::StatusCode;
use redis::ConnectionLike;
use semver::Version;
use serde_json::{json, Value};

use crate::State;

const VERSION: &str = env!("CARGO_PKG_VERSION");
static DEGREE: u8 = 2;

/// Get the current version of the API.
///
/// # Returns
///
/// The API version.
pub async fn version() -> Result<Json<Value>, (StatusCode, String)> {
    Version::parse(VERSION)
        .map(|v| Json(json!(v.major)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Handler for the search route.
///
/// # Args
///
/// * `params` - The query parameters.
/// * `state` - The shared application state.
///
/// # Returns
///
/// A server response.
#[cfg(not(tarpaulin_include))]
pub async fn search<C: ConnectionLike + Send>(
    Query(params): Query<HashMap<String, String>>,
    AxumState(state): AxumState<Arc<impl State<C> + Sync>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    Ok(Json(json!(state.search(query).await?)))
}

/// Handler for the graph route.
///
/// # Args
///
/// * `params` - The query parameters.
/// * `song_id` - Genius song ID from the URL path.
/// * `state` - The shared application state.
///
/// # Returns
///
/// A server response.
#[cfg(not(tarpaulin_include))]
pub async fn graph<C: ConnectionLike + Send>(
    Query(params): Query<HashMap<String, String>>,
    Path(song_id): Path<u32>,
    AxumState(state): AxumState<Arc<impl State<C> + Sync>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let degree: u8 = params
        .get("degree")
        .map(|d| d.parse().unwrap_or(DEGREE))
        .unwrap_or(DEGREE);
    let graph = state.graph(song_id, degree).await?;
    Ok(Json(json!(graph)))
}
