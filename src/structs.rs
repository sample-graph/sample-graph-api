//! Various helper structs for organizing data.

use std::fmt::Debug;

use anyhow::Result;
use genius_rust::{search::Hit, song::Song as GeniusSong, Genius};
use mockall::automock;
use petgraph::graph::NodeIndex;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};

/// Possible relationships between songs.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Samples another song.
    Samples,
    /// Sampled in another song.
    SampledIn,
    /// Interpolates another song.
    Interpolates,
    /// Interpolated by another song.
    InterpolatedBy,
    /// Cover of another song.
    CoverOf,
    /// Covered by another song.
    CoveredBy,
    /// Remix of another song.
    RemixOf,
    /// Remixed by another song.
    RemixedBy,
    /// Live version of another song.
    LiveVersionOf,
    /// Performed live as another song.
    PerformedLiveAs,
    /// Translation of another song.
    TranslationOf,
    /// Translated by another song.
    Translations,
    /// Unknown relationship.
    Unknown,
}

impl<S: AsRef<str>> From<S> for RelationshipType {
    fn from(value: S) -> Self {
        match value.as_ref() {
            "samples" => Self::Samples,
            "sampled_in" => Self::SampledIn,
            "interpolates" => Self::Interpolates,
            "interpolated_by" => Self::InterpolatedBy,
            "cover_of" => Self::CoverOf,
            "covered_by" => Self::CoveredBy,
            "remix_of" => Self::RemixOf,
            "remixed_by" => Self::RemixedBy,
            "live_version_of" => Self::LiveVersionOf,
            "performed_live_as" => Self::PerformedLiveAs,
            "translation_of" => Self::TranslationOf,
            "translations" => Self::Translations,
            _ => Self::Unknown,
        }
    }
}

impl RelationshipType {
    /// Determines if a relationship is relevant to the web API.
    /// Currently just samples (both ways).
    ///
    /// # Returns
    ///
    /// Whether the relationship type is relevant.
    pub fn is_relevant(&self) -> bool {
        matches!(
            self,
            Self::SampledIn | Self::Samples | Self::Interpolates | Self::InterpolatedBy
        )
    }

    /// Inverts the relationship type.
    /// Unknown relationships stay unknown.
    ///
    /// # Returns
    ///
    /// The inverted relationship type.
    pub fn invert(&self) -> Self {
        match self {
            Self::SampledIn => Self::Samples,
            Self::Samples => Self::SampledIn,
            Self::Interpolates => Self::InterpolatedBy,
            Self::InterpolatedBy => Self::Interpolates,
            Self::CoverOf => Self::CoveredBy,
            Self::CoveredBy => Self::CoverOf,
            Self::RemixOf => Self::RemixedBy,
            Self::RemixedBy => Self::RemixOf,
            Self::LiveVersionOf => Self::PerformedLiveAs,
            Self::PerformedLiveAs => Self::LiveVersionOf,
            Self::TranslationOf => Self::Translations,
            Self::Translations => Self::TranslationOf,
            Self::Unknown => Self::Unknown,
        }
    }
}

/// Relevant song data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongData {
    /// Genius ID of the song.
    pub id: u32,
    /// Title of the song.
    pub title: String,
    /// Artist's name who made the song.
    pub artist_name: String,
}

impl SongData {
    /// Create new song data.
    ///
    /// # Args
    ///
    /// * `id` - Genius ID of the song.
    /// * `title` - Title of the song.
    /// * `artist_name` - Artist's name who made the song.
    ///
    /// # Returns
    ///
    /// The song data.
    pub fn new(id: u32, title: String, artist_name: String) -> Self {
        Self {
            id,
            title,
            artist_name,
        }
    }
}

impl From<Hit> for SongData {
    fn from(value: Hit) -> Self {
        Self::new(
            value.result.id,
            value.result.title_with_featured,
            value.result.primary_artist.name,
        )
    }
}

impl From<GeniusSong> for SongData {
    fn from(value: GeniusSong) -> Self {
        Self::new(
            value.id,
            value.title_with_featured,
            value.primary_artist.name,
        )
    }
}

/// A relationship to another song.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Relationship {
    /// The type of relationship.
    pub relationship_type: RelationshipType,
    /// The song that the relationship applies to.
    pub song: SongData,
}

impl Relationship {
    /// Create a new relationship.
    ///
    /// # Args
    ///
    /// * `relationship_type` - The type of relationship.
    /// * `song` - The song that the relationship applies to.
    ///
    /// # Returns
    ///
    /// The relationship.
    pub fn new(relationship_type: RelationshipType, song: SongData) -> Self {
        Self {
            relationship_type,
            song,
        }
    }
}

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
    ///
    pub fn new(degree: u8, song: SongData) -> Self {
        Self { degree, song }
    }
}

/// Shared application state.
pub struct AppState {
    /// Genius API client.
    pub genius: Genius,
    /// Redis client.
    pub redis: Client,
    /// Redis key expiry time.
    pub key_expiry: usize,
}

#[automock]
impl AppState {
    /// Create a new shared application state.
    ///
    /// # Args
    ///
    /// * `genius` - Genius API client.
    /// * `redis` - Redis client.
    /// * `key_expiry` - Redis key expiry time.
    ///
    /// # Returns
    ///
    /// The application state.
    pub fn new(genius: Genius, redis: Client, key_expiry: usize) -> Self {
        Self {
            genius,
            redis,
            key_expiry,
        }
    }

    /// Pull song data associated with a Genius ID.
    /// Tries to pull from Redis first, then from the Genius API.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of the song.
    ///
    /// # Returns
    ///
    /// The song data, or an error if things go wrong.
    pub async fn song(&self, id: u32) -> Result<SongData> {
        let mut con = self.redis.get_connection()?;
        let key = format!("song/{}", id);
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

    /// Pull relevant song relationships associated with a Genius ID.
    /// Tries to pull from Redis first, then from the Genius API.
    ///
    /// # Args
    ///
    /// * `id` - The Genius ID of the song.
    ///
    /// # Returns
    ///
    /// The song relationships, or an error if things go wrong.
    pub async fn relationships(&self, id: u32) -> Result<Vec<Relationship>> {
        let mut con = self.redis.get_connection()?;
        let key = format!("relationships/{}", id);
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
}
