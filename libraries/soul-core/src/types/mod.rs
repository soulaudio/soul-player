mod audio;
mod user;
mod legacy;

// Multi-source types (primary types)
mod source;
mod artist;
mod album;
mod multisource_track;
mod multisource_playlist;

pub use audio::{AudioBuffer, AudioFormat, SampleRate};
pub use user::User;
pub use legacy::{Permission, PlaylistShare, TrackMetadata};

// Multi-source exports (these are the primary types)
pub use source::{Source, SourceType, SourceConfig, CreateSource, SourceId};
pub use artist::{Artist, CreateArtist, ArtistId};
pub use album::{Album, CreateAlbum, AlbumId};
pub use multisource_track::{
    Track, TrackId, CreateTrack, UpdateTrack, TrackAvailability,
    AvailabilityStatus, MetadataSource
};
pub use multisource_playlist::{
    Playlist, CreatePlaylist, PlaylistTrack,
    PlaylistId, UserId
};
