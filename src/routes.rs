//! Functions for API routes.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::{Error, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use petgraph::graph::{DiGraph, NodeIndex};
use semver::Version;
use serde_json::json;

use crate::structs::{AppState, GraphNode, QueueItem, RelationshipType, SongData};

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
) -> Result<DiGraph<GraphNode, RelationshipType>> {
    let mut graph = DiGraph::new();
    let mut visited: HashMap<u32, NodeIndex> = HashMap::new();
    let mut queue = VecDeque::new();

    let center_id = center.id;
    let center_idx = graph.add_node(GraphNode::new(0, center));
    visited.insert(center_id, center_idx);
    queue.push_back(QueueItem::new(0, center_id, center_idx));

    while let Some(current) = queue.pop_front() {
        visited.insert(current.song_id, current.index);
        if current.degree < degree {
            let next_degree = current.degree + 1;
            for relationship in state.relationships(current.song_id).await? {
                let song_id = relationship.song.id;
                if !visited.contains_key(&song_id) {
                    let next_idx = *visited
                        .get(&relationship.song.id)
                        .unwrap_or(&graph.add_node(GraphNode::new(next_degree, relationship.song)));
                    graph.add_edge(current.index, next_idx, relationship.relationship_type);
                    if next_degree < degree {
                        queue.push_back(QueueItem::new(next_degree, song_id, next_idx));
                    }
                }
            }
        }
    }

    Ok(graph)
}
