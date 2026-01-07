mod audio;
mod ids;
mod legacy;
mod user;

// Multi-source types (primary types)
mod album;
mod artist;
mod genre;
mod multisource_playlist;
mod multisource_track;
mod source;

pub use audio::{AudioBuffer, AudioFormat, SampleRate};
pub use ids::{PlaylistId, TrackId, UserId};
pub use legacy::{Permission, PlaylistShare, TrackMetadata};
pub use user::User;

// Multi-source exports (these are the primary types)
pub use album::{Album, AlbumId, CreateAlbum};
pub use artist::{Artist, ArtistId, CreateArtist};
pub use genre::{CreateGenre, Genre, GenreId};
pub use multisource_playlist::{CreatePlaylist, Playlist, PlaylistTrack};
pub use multisource_track::{
    AvailabilityStatus, CreateTrack, MetadataSource, Track, TrackAvailability, UpdateTrack,
};
pub use source::{CreateSource, Source, SourceConfig, SourceId, SourceType};
