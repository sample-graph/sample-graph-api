//! Shared state for the application.

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use genius_rust::{error::GeniusError, Genius};
use http::StatusCode;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    prelude::DiGraphMap,
};
use redis::{Client, Commands, Connection, ConnectionLike, RedisError};
use redis_test::MockRedisConnection;
use serde_json::{error::Error as JsonError, from_slice, to_vec};
use thiserror::Error as ThisError;

use crate::{GraphNode, QueueItem, Relationship, RelationshipType, SongData};

/// Possible errors when consulting the shared application state.
#[derive(ThisError, Debug)]
pub enum StateError {
    /// Error when interacting with the Genius API.
    #[error("Genius API error - {0}")]
    GeniusError(GeniusError),

    /// Error with JSON (de)serialization.
    /// Typically just during the Redis retrieval/write step.
    #[error("JSON error - {0}")]
    JsonError(JsonError),

    /// Error when interacting with the Redis server.
    #[error("Redis error - {0}")]
    RedisError(RedisError),

    /// Generic error when interacting with the MockState.
    #[error("Mock error - {0}")]
    Mock(String),
}

impl From<RedisError> for StateError {
    #[cfg(not(tarpaulin_include))]
    fn from(value: RedisError) -> Self {
        Self::RedisError(value)
    }
}

impl From<JsonError> for StateError {
    #[cfg(not(tarpaulin_include))]
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
pub trait State<C: ConnectionLike + Send> {
    /// Return a Redis connection using the app state.
    /// Mostly a convenience function for implementing Redis mocks.
    ///
    /// # Returns
    ///
    /// A connection to a Redis database.
    fn connection(&self) -> Result<C, StateError>;

    /// Return how long Redis keys should have until they expire.
    ///
    /// # Returns
    ///
    /// The expiry time in seconds.
    fn key_expiry(&self) -> usize;

    /// Return the Redis key for song data.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of the song.
    ///
    /// # Returns
    ///
    /// The Redis key.
    fn song_key(id: u32) -> String {
        format!("song/{}", id)
    }

    /// Return the Redis key for relationship data about a song.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of the song.
    ///
    /// # Returns
    ///
    /// The Redis key.
    fn relationships_key(id: u32) -> String {
        format!("relationships/{}", id)
    }

    /// Return the Redis key for search results for a search query.
    ///
    /// # Args
    ///
    /// * `query` - The search query.
    ///
    /// # Returns
    ///
    /// The Redis key.
    fn search_key(query: &str) -> String {
        format!("search/{}", query)
    }

    /// Return song data for a particular song.
    /// Does not consult a Redis cache.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of a song.
    ///
    /// # Returns
    ///
    /// The song data.
    async fn song_no_cache(&self, id: u32) -> Result<SongData, StateError>;

    /// Return all song relationships for a particular song.
    /// Does not consult a Redis cache.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of a song.
    ///
    /// # Returns
    ///
    /// The relationships for a song.
    async fn relationships_no_cache(&self, id: u32) -> Result<Vec<Relationship>, StateError>;

    /// Return all song results from a Genius search.
    /// Does not consult a Redis cache.
    ///
    /// # Args
    ///
    /// * `query` - The search query.
    ///
    /// # Returns
    ///
    /// The song data from the search.
    async fn search_no_cache(&self, query: &str) -> Result<Vec<SongData>, StateError>;

    /// Return song data for a particular song.
    /// Consults from and stores to a Redis cache.
    ///
    /// # Args
    ///
    /// * `id` The Genius ID of a song.
    ///
    /// # Returns
    ///
    /// The song data.
    async fn song(&self, id: u32) -> Result<SongData, StateError> {
        let mut con = self.connection()?;
        let key = Self::song_key(id);
        if con.exists::<&str, bool>(&key)? {
            let data = con.get::<&str, Vec<u8>>(&key)?;
            Ok(from_slice::<SongData>(&data)?)
        } else {
            let song = self.song_no_cache(id).await?;
            con.set(&key, to_vec(&song)?)?;
            con.expire(&key, self.key_expiry())?;
            Ok(song)
        }
    }

    /// Return all song relationships for a particular song.
    /// Consults from and stores to a Redis cache.
    /// # Args
    ///
    /// * `id` - The Genius ID of a song.
    ///
    /// # Returns
    ///
    /// The relationships for a song.
    async fn relationships(&self, id: u32) -> Result<Vec<Relationship>, StateError> {
        let mut con = self.connection()?;
        let key = Self::relationships_key(id);
        if con.exists::<&str, bool>(&key)? {
            let data = con.get::<&str, Vec<u8>>(&key)?;
            Ok(from_slice::<Vec<Relationship>>(&data)?)
        } else {
            let song = self.relationships_no_cache(id).await?;
            con.set(&key, to_vec(&song)?)?;
            con.expire(&key, self.key_expiry())?;
            Ok(song)
        }
    }

    /// Return all song results from a Genius search.
    /// Consults from and stores to a Redis cache.
    ///
    /// # Args
    ///
    /// * `query` - The search query.
    ///
    /// # Returns
    ///
    /// The song data from the search.
    async fn search(&self, query: &str) -> Result<Vec<SongData>, StateError> {
        let mut con = self.connection()?;
        let key = Self::search_key(query);
        if con.exists::<&str, bool>(&key)? {
            let data = con.get::<&str, Vec<u8>>(&key)?;
            Ok(from_slice::<Vec<SongData>>(&data)?)
        } else {
            let song = self.search_no_cache(query).await?;
            con.set(&key, to_vec(&song)?)?;
            con.expire(&key, self.key_expiry())?;
            Ok(song)
        }
    }

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

/// The main application state.
pub struct AppState {
    /// The Genius API client.
    genius: Genius,
    /// The Redis client.
    redis: Client,
    /// Redis key expiry time.
    key_expiry: usize,
}

impl AppState {
    /// Create a new AppState.
    ///
    /// # Args
    ///
    /// * `genius` - The Genius API client.
    /// * `redis` - The Redis client.
    /// * `key_expiry` - The Redis key expiry time.
    ///
    /// # Returns
    ///
    /// The shared application state.
    #[cfg(not(tarpaulin_include))]
    pub fn new(genius: Genius, redis: Client, key_expiry: usize) -> Self {
        Self {
            genius,
            redis,
            key_expiry,
        }
    }
}

#[async_trait]
impl State<Connection> for AppState {
    #[cfg(not(tarpaulin_include))]
    fn connection(&self) -> Result<Connection, StateError> {
        self.redis.get_connection().map_err(StateError::from)
    }

    #[cfg(not(tarpaulin_include))]
    fn key_expiry(&self) -> usize {
        self.key_expiry
    }

    #[cfg(not(tarpaulin_include))]
    async fn song_no_cache(&self, id: u32) -> Result<SongData, StateError> {
        Ok(self
            .genius
            .get_song(id, "plain")
            .await
            .map(SongData::from)?)
    }

    #[cfg(not(tarpaulin_include))]
    async fn relationships_no_cache(&self, id: u32) -> Result<Vec<Relationship>, StateError> {
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
        Ok(relationships)
    }

    #[cfg(not(tarpaulin_include))]
    async fn search_no_cache(&self, query: &str) -> Result<Vec<SongData>, StateError> {
        Ok(self
            .genius
            .search(query)
            .await?
            .into_iter()
            .map(SongData::from)
            .collect::<Vec<SongData>>())
    }
}

/// A mock application state for testing some of the core `State` methods.
pub struct MockState {
    /// A mock Redis connection.
    mock_redis: MockRedisConnection,
    /// A mock graph that represents the relationships between songs.
    graph: DiGraphMap<u32, RelationshipType>,
    /// Mock song data.
    songs: HashMap<u32, SongData>,
    /// Mock search results.
    search: HashMap<String, Vec<SongData>>,
    /// Mock Redis key expiry time.
    key_expiry: usize,
}

impl MockState {
    /// Create a new MockState.
    ///
    /// # Args
    ///
    /// * `mock_redis` - A mock Redis connection.
    /// * `graph` - A mock graph that represents the relationships between songs.
    /// * `songs` - Mock song data.
    /// * `search` - Mock search results.
    /// * `key_expiry` - Mock Redis key expiry time.
    ///
    /// # Returns
    ///
    /// The mocked application state.
    #[cfg(not(tarpaulin_include))]
    pub fn new(
        mock_redis: MockRedisConnection,
        graph: DiGraphMap<u32, RelationshipType>,
        songs: HashMap<u32, SongData>,
        search: HashMap<String, Vec<SongData>>,
        key_expiry: usize,
    ) -> Self {
        Self {
            mock_redis,
            graph,
            songs,
            search,
            key_expiry,
        }
    }
}

#[async_trait]
impl State<MockRedisConnection> for MockState {
    #[cfg(not(tarpaulin_include))]
    fn connection(&self) -> Result<MockRedisConnection, StateError> {
        Ok(self.mock_redis.clone())
    }

    fn key_expiry(&self) -> usize {
        self.key_expiry
    }

    async fn song_no_cache(&self, id: u32) -> Result<SongData, StateError> {
        Ok(self
            .songs
            .get(&id)
            .ok_or_else(|| StateError::Mock("No song found".into()))?
            .clone())
    }

    async fn relationships_no_cache(&self, id: u32) -> Result<Vec<Relationship>, StateError> {
        let mut relationships = Vec::new();
        for (_from, to, rel_type) in self.graph.edges(id) {
            if rel_type.is_relevant() {
                let song = self.song_no_cache(to).await?;
                relationships.push(Relationship::new(*rel_type, song));
            }
        }
        Ok(relationships)
    }

    async fn search_no_cache(&self, query: &str) -> Result<Vec<SongData>, StateError> {
        Ok(self
            .search
            .get(query)
            .map(Clone::clone)
            .unwrap_or_else(Vec::new))
    }
}

#[cfg(test)]
mod tests {
    use redis::{cmd, Value};
    use redis_test::MockCmd;
    use rstest::*;
    use serde_json::{json, to_string};

    use super::*;

    #[fixture]
    fn genius_err() -> GeniusError {
        GeniusError::Unauthorized("oh no!".into())
    }

    #[fixture]
    fn songs() -> Vec<SongData> {
        vec![
            SongData::new(1, "Foobar".into(), "The Sillys".into()),
            SongData::new(2, "Barfoo".into(), "The Seriouses".into()),
            SongData::new(1, "Barfoo 2".into(), "Even More Serious".into()),
        ]
    }

    fn mock_state_helper(mock_commands: Vec<MockCmd>, songs: Vec<SongData>) -> MockState {
        let mock_redis = MockRedisConnection::new(mock_commands);
        let song_1 = songs[0].clone();

        let graph = DiGraphMap::from_edges([
            (1, 2, RelationshipType::Samples),
            (2, 1, RelationshipType::SampledIn),
            (2, 3, RelationshipType::InterpolatedBy),
            (3, 2, RelationshipType::Interpolates),
            (1, 3, RelationshipType::RemixOf),
            (3, 1, RelationshipType::RemixedBy),
        ]);
        let songs = HashMap::from([
            (1, song_1.clone()),
            (2, songs[1].clone()),
            (3, songs[2].clone()),
        ]);
        let search = HashMap::from([
            ("foobar".to_string(), vec![song_1]),
            ("testing".to_string(), vec![]),
        ]);
        MockState::new(mock_redis, graph, songs, search, 100)
    }

    #[fixture]
    fn mock_state(songs: Vec<SongData>) -> MockState {
        mock_state_helper(vec![], songs)
    }

    #[fixture]
    fn mock_song_state(songs: Vec<SongData>) -> MockState {
        let mock_cmds = vec![
            MockCmd::new(cmd("EXISTS").arg("song/1"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["song/1", &to_string(&songs[0].clone()).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(cmd("EXPIRE").arg(&["song/1", "100"]), Ok(Value::Okay)),
            MockCmd::new(cmd("EXISTS").arg("song/2"), Ok("1")),
            MockCmd::new(
                cmd("GET").arg("song/2"),
                Ok(Value::Data(to_vec(&songs[1].clone()).unwrap())),
            ),
            MockCmd::new(cmd("EXISTS").arg("song/3"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["song/3", &to_string(&songs[2].clone()).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(cmd("EXPIRE").arg(&["song/3", "100"]), Ok(Value::Okay)),
        ];
        mock_state_helper(mock_cmds, songs)
    }

    #[fixture]
    fn mock_relationships_state(songs: Vec<SongData>) -> MockState {
        let rels_1 = vec![Relationship::new(
            RelationshipType::Samples,
            songs[1].clone(),
        )];
        let rels_2 = vec![
            Relationship::new(RelationshipType::SampledIn, songs[0].clone()),
            Relationship::new(RelationshipType::InterpolatedBy, songs[2].clone()),
        ];
        let mock_cmds = vec![
            MockCmd::new(cmd("EXISTS").arg("relationships/1"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["relationships/1", &to_string(&rels_1).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(
                cmd("EXPIRE").arg(&["relationships/1", "100"]),
                Ok(Value::Okay),
            ),
            MockCmd::new(cmd("EXISTS").arg("relationships/2"), Ok("1")),
            MockCmd::new(
                cmd("GET").arg("relationships/2"),
                Ok(Value::Data(to_vec(&rels_2).unwrap())),
            ),
        ];
        mock_state_helper(mock_cmds, songs)
    }

    #[fixture]
    fn mock_search_state(songs: Vec<SongData>) -> MockState {
        let search_1 = vec![songs[0].clone()];
        let mock_cmds = vec![
            MockCmd::new(cmd("EXISTS").arg("search/foobar"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["search/foobar", &to_string(&search_1).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(
                cmd("EXPIRE").arg(&["search/foobar", "100"]),
                Ok(Value::Okay),
            ),
            MockCmd::new(cmd("EXISTS").arg("search/testing"), Ok("1")),
            MockCmd::new(
                cmd("GET").arg("search/testing"),
                Ok(Value::Data(to_vec::<Vec<SongData>>(&vec![]).unwrap())),
            ),
        ];
        mock_state_helper(mock_cmds, songs)
    }

    #[fixture]
    fn mock_graph_state(songs: Vec<SongData>) -> MockState {
        let rels_1 = vec![Relationship::new(
            RelationshipType::Samples,
            songs[1].clone(),
        )];
        let rels_2 = vec![
            Relationship::new(RelationshipType::SampledIn, songs[0].clone()),
            Relationship::new(RelationshipType::InterpolatedBy, songs[2].clone()),
        ];
        let rels_3 = vec![Relationship::new(
            RelationshipType::Interpolates,
            songs[1].clone(),
        )];
        let mock_cmds = vec![
            MockCmd::new(cmd("EXISTS").arg("song/1"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["song/1", &to_string(&songs[0].clone()).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(cmd("EXPIRE").arg(&["song/1", "100"]), Ok(Value::Okay)),
            MockCmd::new(cmd("EXISTS").arg("relationships/1"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["relationships/1", &to_string(&rels_1).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(
                cmd("EXPIRE").arg(&["relationships/1", "100"]),
                Ok(Value::Okay),
            ),
            MockCmd::new(cmd("EXISTS").arg("relationships/2"), Ok("1")),
            MockCmd::new(
                cmd("GET").arg("relationships/2"),
                Ok(Value::Data(to_vec(&rels_2).unwrap())),
            ),
            MockCmd::new(cmd("EXISTS").arg("relationships/3"), Ok("0")),
            MockCmd::new(
                cmd("SET").arg(&["relationships/3", &to_string(&rels_3).unwrap()]),
                Ok(Value::Okay),
            ),
            MockCmd::new(
                cmd("EXPIRE").arg(&["relationships/3", "100"]),
                Ok(Value::Okay),
            ),
        ];
        mock_state_helper(mock_cmds, songs)
    }

    #[rstest]
    fn test_state_error_from_genius_error(genius_err: GeniusError) {
        assert!(matches!(
            StateError::from(genius_err),
            StateError::GeniusError(..)
        ));
    }

    #[rstest]
    fn test_status_string_from_state_error(genius_err: GeniusError) {
        let result: (StatusCode, String) = StateError::from(genius_err).into();
        assert_eq!(
            result,
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Genius API error - Unauthorized: oh no!".into()
            )
        );
    }

    #[rstest]
    #[case(0, "song/0")]
    #[case(12345, "song/12345")]
    fn test_state_song_key(#[case] input: u32, #[case] expected: String) {
        assert_eq!(MockState::song_key(input), expected);
    }

    #[rstest]
    #[case(0, "relationships/0")]
    #[case(12345, "relationships/12345")]
    fn test_state_relationships_key(#[case] input: u32, #[case] expected: String) {
        assert_eq!(MockState::relationships_key(input), expected);
    }

    #[rstest]
    #[case("foobar", "search/foobar")]
    #[case("barfoo", "search/barfoo")]
    fn test_state_search_key(#[case] input: &str, #[case] expected: String) {
        assert_eq!(MockState::search_key(input), expected);
    }

    #[rstest]
    fn test_mock_state_key_expiry(mock_state: MockState) {
        assert_eq!(mock_state.key_expiry(), 100);
    }

    #[rstest]
    #[case(1, SongData::new(1, "Foobar".into(), "The Sillys".into()))]
    #[should_panic]
    #[case(4, SongData::new(4, "Does not exists".into(), "oops".into()))]
    async fn test_mock_state_song_no_cache(
        mock_state: MockState,
        #[case] input: u32,
        #[case] expected: SongData,
    ) {
        assert_eq!(mock_state.song_no_cache(input).await.unwrap(), expected);
    }

    #[rstest]
    #[case(4, &[])]
    #[case(1, &[Relationship::new(RelationshipType::Samples, SongData::new(2, "Barfoo".into(), "The Seriouses".into()))])]
    async fn test_mock_state_relationships_no_cache(
        mock_state: MockState,
        #[case] input: u32,
        #[case] expected: &[Relationship],
    ) {
        assert_eq!(
            mock_state.relationships_no_cache(input).await.unwrap(),
            expected
        );
    }

    #[rstest]
    #[case("does not exist", &[])]
    #[case("testing", &[])]
    #[case("foobar", &[SongData::new(1, "Foobar".into(), "The Sillys".into())])]
    async fn test_mock_state_search_no_cache(
        mock_state: MockState,
        #[case] input: &str,
        #[case] expected: &[SongData],
    ) {
        assert_eq!(mock_state.search_no_cache(input).await.unwrap(), expected);
    }

    #[rstest]
    async fn test_state_song(mock_song_state: MockState) {
        for input in 1..3 {
            assert_eq!(
                mock_song_state.song(input).await.unwrap(),
                mock_song_state.song_no_cache(input).await.unwrap(),
            );
        }
    }

    #[rstest]
    async fn test_state_relationships(mock_relationships_state: MockState) {
        for input in 1..2 {
            assert_eq!(
                mock_relationships_state.relationships(input).await.unwrap(),
                mock_relationships_state
                    .relationships_no_cache(input)
                    .await
                    .unwrap(),
            )
        }
    }

    #[rstest]
    async fn test_state_search(mock_search_state: MockState) {
        for input in ["foobar", "testing"] {
            assert_eq!(
                mock_search_state.search(input).await.unwrap(),
                mock_search_state.search_no_cache(input).await.unwrap(),
            )
        }
    }

    #[rstest]
    async fn test_state_graph(mock_graph_state: MockState, songs: Vec<SongData>) {
        // THIS TEST DOES NOT WORK AS EXPECTED, BUT LIVE USAGE OF THE GRAPH API SEEMS FINE
        let result = mock_graph_state.graph(1, 2).await.unwrap();
        let mut expected = DiGraph::new();
        let song_1 = expected.add_node(GraphNode::new(0, songs[0].clone()));
        let song_2 = expected.add_node(GraphNode::new(1, songs[1].clone()));
        // let song_3 = expected.add_node(GraphNode::new(2, songs[2].clone()));
        expected.add_edge(song_1, song_2, RelationshipType::Samples);
        // expected.add_edge(song_2, song_3, RelationshipType::InterpolatedBy);
        assert_eq!(json!(result), json!(expected));
    }
}
