//! Various helper structs for organizing data.

use std::fmt::Debug;

use genius_rust::{search::Hit, song::Song as GeniusSong};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

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
}

/// Relevant song data.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use genius_rust::{
        search::Hit,
        song::{Artist, Song, SongStatus},
    };
    use rstest::*;
    use serde_json::{from_value, json, to_value};

    use super::*;

    #[fixture]
    fn song() -> Song {
        Song {
            annotation_count: 0,
            api_path: "".into(),
            apple_music_id: None,
            apple_music_player_url: None,
            comment_count: None,
            custom_header_image_url: None,
            custom_song_art_image_url: None,
            description: None,
            description_preview: None,
            embed_content: None,
            facebook_share_message_without_url: None,
            featured_video: None,
            full_title: "".into(),
            has_instagram_reel_annotations: None,
            header_image_thumbnail_url: "".into(),
            header_image_url: "".into(),
            hidden: None,
            id: 12345,
            instrumental: None,
            is_music: None,
            lyrics: None,
            lyrics_owner_id: 0,
            lyrics_state: "".into(),
            lyrics_updated_at: None,
            path: "".into(),
            pending_lyrics_edits_count: None,
            published: None,
            pusher_channel: None,
            release_date_components: None,
            pyongs_count: None,
            recording_location: None,
            release_date: None,
            release_date_for_display: None,
            share_url: None,
            song_art_image_thumbnail_url: "".into(),
            song_art_image_url: "".into(),
            soundcloud_url: None,
            spotify_uuid: None,
            stats: SongStatus {
                accepted_annotations: None,
                contributors: None,
                iq_earners: None,
                transcribers: None,
                verified_annotations: None,
                unreviewed_annotations: 0,
                hot: false,
                pageviews: None,
            },
            title: "".into(),
            title_with_featured: "Foobar".into(),
            tracking_paths: None,
            twitter_share_message: None,
            twitter_share_message_without_url: None,
            updated_by_human_at: None,
            url: "".into(),
            viewable_by_roles: None,
            youtube_start: None,
            youtube_url: None,
            current_user_metadata: None,
            primary_artist: Artist {
                api_path: "".into(),
                header_image_url: "".into(),
                id: 0,
                image_url: "".into(),
                index_character: None,
                is_meme_verified: false,
                is_verified: false,
                name: "Barfoo".into(),
                slug: None,
                url: "".into(),
                iq: None,
            },
            album: None,
            albums: None,
            custom_performances: None,
            description_annotation: None,
            featured_artists: None,
            media: None,
            producer_artists: None,
            song_relationships: None,
            verified_annotations_by: None,
            verified_contributors: None,
            verified_lyrics_by: None,
            writer_artists: None,
        }
    }

    #[fixture]
    fn hit(song: Song) -> Hit {
        Hit {
            hit_type: "".into(),
            index: "".into(),
            result: song,
        }
    }

    #[rstest]
    #[case("samples", RelationshipType::Samples)]
    #[case("sampled_in", RelationshipType::SampledIn)]
    #[case("interpolates", RelationshipType::Interpolates)]
    #[case("interpolated_by", RelationshipType::InterpolatedBy)]
    #[case("cover_of", RelationshipType::CoverOf)]
    #[case("covered_by", RelationshipType::CoveredBy)]
    #[case("remix_of", RelationshipType::RemixOf)]
    #[case("remixed_by", RelationshipType::RemixedBy)]
    #[case("live_version_of", RelationshipType::LiveVersionOf)]
    #[case("performed_live_as", RelationshipType::PerformedLiveAs)]
    #[case("translation_of", RelationshipType::TranslationOf)]
    #[case("translations", RelationshipType::Translations)]
    #[case("foobar", RelationshipType::Unknown)]
    fn test_relationship_type_from_str(#[case] input: &str, #[case] expected: RelationshipType) {
        assert_eq!(RelationshipType::from(input), expected);
    }

    #[rstest]
    #[case("samples", RelationshipType::Samples)]
    #[case("sampled_in", RelationshipType::SampledIn)]
    #[case("interpolates", RelationshipType::Interpolates)]
    #[case("interpolated_by", RelationshipType::InterpolatedBy)]
    #[case("cover_of", RelationshipType::CoverOf)]
    #[case("covered_by", RelationshipType::CoveredBy)]
    #[case("remix_of", RelationshipType::RemixOf)]
    #[case("remixed_by", RelationshipType::RemixedBy)]
    #[case("live_version_of", RelationshipType::LiveVersionOf)]
    #[case("performed_live_as", RelationshipType::PerformedLiveAs)]
    #[case("translation_of", RelationshipType::TranslationOf)]
    #[case("translations", RelationshipType::Translations)]
    #[case("unknown", RelationshipType::Unknown)]
    fn test_relationship_type_serialize(#[case] expected: &str, #[case] input: RelationshipType) {
        assert_eq!(to_value(&input).unwrap(), json!(expected));
    }

    #[rstest]
    #[case("samples", RelationshipType::Samples)]
    #[case("sampled_in", RelationshipType::SampledIn)]
    #[case("interpolates", RelationshipType::Interpolates)]
    #[case("interpolated_by", RelationshipType::InterpolatedBy)]
    #[case("cover_of", RelationshipType::CoverOf)]
    #[case("covered_by", RelationshipType::CoveredBy)]
    #[case("remix_of", RelationshipType::RemixOf)]
    #[case("remixed_by", RelationshipType::RemixedBy)]
    #[case("live_version_of", RelationshipType::LiveVersionOf)]
    #[case("performed_live_as", RelationshipType::PerformedLiveAs)]
    #[case("translation_of", RelationshipType::TranslationOf)]
    #[case("translations", RelationshipType::Translations)]
    #[case("unknown", RelationshipType::Unknown)]
    fn test_relationship_type_deserialize(#[case] input: &str, #[case] expected: RelationshipType) {
        assert_eq!(
            from_value::<RelationshipType>(json!(input)).unwrap(),
            expected
        );
    }

    #[rstest]
    #[case(true, RelationshipType::Samples)]
    #[case(true, RelationshipType::SampledIn)]
    #[case(true, RelationshipType::Interpolates)]
    #[case(true, RelationshipType::InterpolatedBy)]
    #[case(false, RelationshipType::CoverOf)]
    #[case(false, RelationshipType::CoveredBy)]
    #[case(false, RelationshipType::RemixOf)]
    #[case(false, RelationshipType::RemixedBy)]
    #[case(false, RelationshipType::LiveVersionOf)]
    #[case(false, RelationshipType::PerformedLiveAs)]
    #[case(false, RelationshipType::TranslationOf)]
    #[case(false, RelationshipType::Translations)]
    #[case(false, RelationshipType::Unknown)]
    fn test_relationship_type_is_relevant(#[case] expected: bool, #[case] input: RelationshipType) {
        assert_eq!(input.is_relevant(), expected);
    }

    #[rstest]
    fn test_song_data_new(
        #[values(u32::MIN, u32::MAX, 0, 2539091)] id: u32,
        #[values("foobar", "Sillyman!", "")] title: String,
        #[values("foofighters", "The sillys", "")] artist_name: String,
    ) {
        let result = SongData::new(id, title.clone(), artist_name.clone());
        assert_eq!(result.id, id);
        assert_eq!(result.title, title);
        assert_eq!(result.artist_name, artist_name);
    }

    #[rstest]
    fn test_song_data_from_song(song: Song) {
        let result = SongData::from(song);
        assert_eq!(result.id, 12345);
        assert_eq!(result.title, "Foobar");
        assert_eq!(result.artist_name, "Barfoo");
    }

    #[rstest]
    fn test_song_data_from_hit(hit: Hit) {
        let result = SongData::from(hit);
        assert_eq!(result.id, 12345);
        assert_eq!(result.title, "Foobar");
        assert_eq!(result.artist_name, "Barfoo");
    }

    #[rstest]
    fn test_relationship_new(
        #[values(u32::MIN, u32::MAX, 0, 2539091)] id: u32,
        #[values("foobar", "Sillyman!", "")] title: String,
        #[values("foofighters", "The sillys", "")] artist_name: String,
        #[values(
            RelationshipType::Samples,
            RelationshipType::InterpolatedBy,
            RelationshipType::Unknown
        )]
        relationship_type: RelationshipType,
    ) {
        let song = SongData::new(id, title, artist_name);
        let result = Relationship::new(relationship_type, song.clone());
        assert_eq!(result.relationship_type, relationship_type);
        assert_eq!(result.song, song);
    }

    #[rstest]
    fn test_queue_item_new() {
        let result = QueueItem::new(255, 12345, NodeIndex::default());
        assert_eq!(result.degree, 255);
        assert_eq!(result.song_id, 12345);
        assert_eq!(result.index, NodeIndex::default());
    }

    #[rstest]
    fn test_graph_node_new() {
        let result = GraphNode::new(255, SongData::new(12345, "Foobar".into(), "Barfoo".into()));
        assert_eq!(result.degree, 255);
        assert_eq!(
            result.song,
            SongData::new(12345, "Foobar".into(), "Barfoo".into())
        );
    }
}
