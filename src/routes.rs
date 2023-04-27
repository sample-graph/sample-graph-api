//! Functions for API routes.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use anyhow::{Error, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use petgraph::graph::DiGraph;
use semver::Version;
use serde_json::json;

use crate::structs::{AppState, QueueItem, RelationshipType, SongData};

const VERSION: &str = env!("CARGO_PKG_VERSION");
static DEGREE: u8 = 2;

/// Get the current version of the API.
///
/// # Returns
///
/// The API version.
pub async fn version() -> Response {
    match Version::parse(VERSION) {
        Ok(v) => Json(json!(v.major)).into_response(),
        Err(e) => handle_error(e.into()),
    }
}

/// Convert an error into a server response.
///
/// # Args
///
/// * `err` - The error from the `anyhow` crate.
///
/// # Returns
///
/// The server response.
fn handle_error(err: Error) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
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
) -> Response {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let results = state.genius.search(query).await;
    match results {
        Ok(hits) => Json(
            hits.into_iter()
                .map(SongData::from)
                .collect::<Vec<SongData>>(),
        )
        .into_response(),
        Err(err) => handle_error(err.into()),
    }
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
) -> Response {
    let degree: u8 = params
        .get("degree")
        .map(|d| d.parse().unwrap_or(DEGREE))
        .unwrap_or(DEGREE);
    let song: Result<SongData> = state.song(song_id).await;
    match song {
        Ok(song) => match build_graph(state, song, degree).await {
            Ok(graph) => Json(graph).into_response(),
            Err(err) => handle_error(err),
        },
        Err(err) => handle_error(err),
    }
}

/// Construct a graph of song relationships usingc breadth-first search.
///
/// # Args
///
/// * `state` - The shared application state.
/// * `center` - The center of the graph.
/// * `degree` - The maximum degree of separation from the center.
async fn build_graph(
    state: Arc<AppState>,
    center: SongData,
    degree: u8,
) -> Result<DiGraph<SongData, RelationshipType>> {
    let mut graph = DiGraph::new();
    let mut visited: HashSet<u32> = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(center.id);
    queue.push_back(QueueItem::new(0, center.id, graph.add_node(center)));

    while let Some(current) = queue.pop_front() {
        if current.degree < degree {
            let next_degree = current.degree + 1;
            for relationship in state.relationships(current.song_id).await? {
                if !visited.contains(&relationship.song.id) {
                    let song_id = relationship.song.id;
                    let next_index = graph.add_node(relationship.song);
                    graph.add_edge(current.index, next_index, relationship.relationship_type);
                    graph.add_edge(
                        next_index,
                        current.index,
                        relationship.relationship_type.invert(),
                    );
                    if next_degree < degree {
                        queue.push_back(QueueItem::new(next_degree, song_id, next_index));
                    }
                }
            }
        }
    }

    Ok(graph)
}
