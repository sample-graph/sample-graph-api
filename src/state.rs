//! Shared state for the application.

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use genius_rust::{error::GeniusError, Genius};
use http::StatusCode;
use petgraph::graph::{DiGraph, NodeIndex};
use redis::{Client, Commands, Connection, ConnectionLike, RedisError};
use serde_json::Error as JsonError;
use serde_json::{from_slice, to_vec};
use thiserror::Error as ThisError;

use crate::{GraphNode, QueueItem, Relationship, RelationshipType, SongData};

/// Possible errors when consulting the shared application state.
#[derive(ThisError, Debug)]
pub enum StateError {
    ///
    #[error("Genius API error: {0}")]
    GeniusError(GeniusError),

    ///
    #[error("JSON error: {0}")]
    JsonError(JsonError),

    ///
    #[error("Redis error: {0}")]
    RedisError(RedisError),
}

impl From<RedisError> for StateError {
    fn from(value: RedisError) -> Self {
        Self::RedisError(value)
    }
}

impl From<JsonError> for StateError {
    fn from(value: JsonError) -> Self {
        Self::JsonError(value)
    }
}

impl From<GeniusError> for StateError {
    fn from(value: GeniusError) -> Self {
        Self::GeniusError(value)
    }
}

impl From<StateError> for (StatusCode, String) {
    fn from(value: StateError) -> Self {
        (StatusCode::INTERNAL_SERVER_ERROR, value.to_string())
    }
}

/// Required methods for the shared application state.
#[async_trait]
pub trait State<C: ConnectionLike> {
    /// Return a Redis connection using the app state.
    /// Mostly a convenience function for implementing Redis mocks.
    ///
    /// # Returns
    ///
    /// A connection to a Redis database.
    fn connection(&self) -> Result<C, StateError>;

    /// Return song data for a particular song.
    ///
    /// # Args
    ///
    /// * `id` The Genius ID of a song.
    ///
    /// # Returns
    ///
    /// The song data.
    async fn song(&self, id: u32) -> Result<SongData, StateError>;

    /// Return all song relationships for a particular song.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of a song.
    ///
    /// # Returns
    ///
    /// The relationships for a song.
    async fn relationships(&self, id: u32) -> Result<Vec<Relationship>, StateError>;

    /// Return all song results from a Genius search.
    ///
    /// # Args
    ///
    /// * `query` - The search query.
    ///
    /// # Returns
    ///
    /// The song data from the search.
    async fn search(&self, query: &str) -> Result<Vec<SongData>, StateError>;

    /// Return a graph of song relationships using the app state.
    ///
    /// # Args
    ///
    /// * `start_id` - The Genius ID of the starting node.
    /// * `degree` - The maximum degree of separation between any node and the start node.
    ///
    /// # Returns
    ///
    /// A graph of all of the musical relationships from the start song.
    async fn graph(
        &self,
        start_id: u32,
        degree: u8,
    ) -> Result<DiGraph<GraphNode, RelationshipType>, StateError> {
        let mut graph = DiGraph::new();
        let mut visited: HashMap<u32, NodeIndex> = HashMap::new();
        let mut queue = VecDeque::new();

        let start_idx = graph.add_node(GraphNode::new(0, self.song(start_id).await?));
        visited.insert(start_id, start_idx);
        queue.push_back(QueueItem::new(0, start_id, start_idx));

        while let Some(current) = queue.pop_front() {
            visited.insert(current.song_id, current.index);
            if current.degree < degree {
                let next_degree = current.degree + 1;
                for relationship in self.relationships(current.song_id).await? {
                    let song_id = relationship.song.id;
                    if !visited.contains_key(&song_id) {
                        let next_idx = *visited.get(&relationship.song.id).unwrap_or(
                            &graph.add_node(GraphNode::new(next_degree, relationship.song)),
                        );
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
}

///
pub struct AppState {
    genius: Genius,
    redis: Client,
    key_expiry: usize,
}

impl AppState {
    ///
    pub fn new(genius: Genius, redis: Client, key_expiry: usize) -> Self {
        Self {
            genius,
            redis,
            key_expiry,
        }
    }

    fn song_key(id: u32) -> String {
        format!("song/{}", id)
    }

    fn relationships_key(id: u32) -> String {
        format!("relationships/{}", id)
    }

    fn search_key(query: &str) -> String {
        format!("search/{}", query)
    }
}

#[async_trait]
impl State<Connection> for AppState {
    fn connection(&self) -> Result<Connection, StateError> {
        self.redis.get_connection().map_err(StateError::from)
    }

    async fn song(&self, id: u32) -> Result<SongData, StateError> {
        let mut con = self.connection()?;
        let key = AppState::song_key(id);
        if con.exists::<&str, bool>(&key)? {
            let data = con.get::<&str, Vec<u8>>(&key)?;
            Ok(from_slice::<SongData>(&data)?)
        } else {
            let song = self
                .genius
                .get_song(id, "plain")
                .await
                .map(SongData::from)?;
            con.set(&key, to_vec(&song)?)?;
            con.expire(&key, self.key_expiry)?;
            Ok(song)
        }
    }

    async fn relationships(&self, id: u32) -> Result<Vec<Relationship>, StateError> {
        let mut con = self.connection()?;
        let key = AppState::relationships_key(id);
        if con.exists::<&str, bool>(&key)? {
            let data = con.get::<&str, Vec<u8>>(&key)?;
            Ok(from_slice::<Vec<Relationship>>(&data)?)
        } else {
            let mut relationships = Vec::new();
            if let Some(gr) = self.genius.get_song(id, "plain").await?.song_relationships {
                for r in gr {
                    let rt = RelationshipType::from(r.relationship_type);
                    if rt.is_relevant() {
                        for s in r.songs.into_iter().flatten() {
                            relationships.push(Relationship::new(rt, SongData::from(s)));
                        }
                    }
                }
            }
            con.set(&key, to_vec(&relationships)?)?;
            con.expire(&key, self.key_expiry)?;
            Ok(relationships)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<SongData>, StateError> {
        let mut con = self.connection()?;
        let key = AppState::search_key(query);
        if con.exists::<&str, bool>(&key)? {
            let data = con.get::<&str, Vec<u8>>(&key)?;
            Ok(from_slice::<Vec<SongData>>(&data)?)
        } else {
            let hits = self
                .genius
                .search(query)
                .await?
                .into_iter()
                .map(SongData::from)
                .collect::<Vec<SongData>>();
            con.set(&key, to_vec(&hits)?)?;
            con.expire(&key, self.key_expiry)?;
            Ok(hits)
        }
    }
}
