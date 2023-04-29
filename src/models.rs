//! Various helper structs for organizing data.

use std::fmt::Debug;

use genius_rust::{search::Hit, song::Song as GeniusSong};
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

#[cfg(test)]
mod tests {
    use rstest::*;
    use serde_json::{from_value, json, to_value};

    use super::*;

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
}
