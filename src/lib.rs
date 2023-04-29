//! Library code for the SampleGraph backend API.
#![deny(
    missing_docs,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

pub mod routes;
use std::error::Error;

pub use routes::*;
pub mod graph;
pub use graph::*;
pub mod models;
pub use models::*;

use genius_rust::Genius;
use mockall::automock;
use redis::{Client, Commands};
use serde_json::{from_slice, to_vec};

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
    pub async fn song(&self, id: u32) -> Result<SongData, Box<dyn Error>> {
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
    pub async fn relationships(&self, id: u32) -> Result<Vec<Relationship>, Box<dyn Error>> {
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
