//! Generate song graphs.

use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    sync::Arc,
};

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

use crate::{
    models::{RelationshipType, SongData},
    AppState,
};

/// An item in a graph search queue.
#[derive(Debug, Copy, Clone)]
pub struct QueueItem {
    /// The degree of separation from the graph center.
    pub degree: u8,
    /// The Genius ID of the queued song.
    pub song_id: u32,
    /// The graph node index of the queued song.
    pub index: NodeIndex,
}

impl QueueItem {
    /// Create a queue item.
    ///
    /// # Args
    ///
    /// * `degree` - The degree of separation from the graph center.
    /// * `song_id` - The Genius ID of the queued song.
    /// * `index` - The graph node index of the queued song.
    ///
    /// # Returns
    ///
    /// The queue item.
    pub fn new(degree: u8, song_id: u32, index: NodeIndex) -> Self {
        Self {
            degree,
            index,
            song_id,
        }
    }
}

/// Node data in a graph.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphNode {
    /// Degree of separation from the center.
    pub degree: u8,
    /// Genius song data.
    pub song: SongData,
}

impl GraphNode {
    /// Create a new graph node.
    /// 
    /// # Args
    /// 
    /// * `degree` - Degree of separation from the center.
    /// * `song` - Genius song data.
    /// 
    /// # Returns
    /// 
    /// The graph node.
    pub fn new(degree: u8, song: SongData) -> Self {
        Self { degree, song }
    }
}

/// Construct a graph of song relationships usingc breadth-first search.
///
/// # Args
///
/// * `state` - The shared application state.
/// * `center` - The center of the graph.
/// * `degree` - The maximum degree of separation from the center.
/// 
/// # Returns
/// 
/// The final graph.
pub async fn build_graph(
    state: Arc<AppState>,
    center: SongData,
    degree: u8,
) -> Result<DiGraph<GraphNode, RelationshipType>, Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
}
