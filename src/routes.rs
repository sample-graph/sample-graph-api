//! Functions for API routes.

use std::{collections::HashMap, error::Error, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use http::StatusCode;
use semver::Version;
use serde_json::{json, Value};

use crate::{graph::build_graph, models::SongData, AppState};

const VERSION: &str = env!("CARGO_PKG_VERSION");
static DEGREE: u8 = 2;

/// Convert an error into a simple response.
///
/// # Args
///
/// * `e` - The error the handle.
///
/// # Returns
///
/// * INTERNAL_SERVER_ERROR status code + error as a string.
pub fn simple_err_handler(e: Box<dyn Error>) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// Get the current version of the API.
///
/// # Returns
///
/// The API version.
pub async fn version() -> Result<Json<Value>, (StatusCode, String)> {
    Version::parse(VERSION)
        .map(|v| Json(json!(v.major)))
        .map_err(|e| simple_err_handler(Box::new(e)))
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
pub async fn search(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let results = state
        .genius
        .search(query)
        .await
        .map_err(|e| simple_err_handler(Box::new(e)))?;
    Ok(Json(json!(results
        .into_iter()
        .map(SongData::from)
        .collect::<Vec<SongData>>())))
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
pub async fn graph(
    Query(params): Query<HashMap<String, String>>,
    Path(song_id): Path<u32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let degree: u8 = params
        .get("degree")
        .map(|d| d.parse().unwrap_or(DEGREE))
        .unwrap_or(DEGREE);
    let song = state.song(song_id).await.map_err(simple_err_handler)?;
    let graph = build_graph(state, song, degree)
        .await
        .map_err(simple_err_handler)?;
    Ok(Json(json!(graph)))
}
