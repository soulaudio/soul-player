mod audio;
mod device;
mod external_file_settings;
mod ids;
mod legacy;
mod library_source;
mod managed_library;
mod playback_state;
mod user;

// Multi-source types (primary types)
mod album;
mod artist;
mod genre;
mod multisource_playlist;
mod multisource_track;
mod source;

pub use audio::{AudioBuffer, AudioFormat, SampleRate};
pub use device::{Device, DeviceType, RegisterDevice};
pub use external_file_settings::{
    ExternalFileAction, ExternalFileSettings, ImportDestination, UpdateExternalFileSettings,
};
pub use ids::{PlaylistId, TrackId, UserId};
pub use library_source::{
    CreateLibrarySource, LibrarySource, ScanProgress, ScanProgressStatus, ScanStatus,
    UpdateLibrarySource,
};
pub use managed_library::{
    ImportAction, ManagedLibrarySettings, PathTemplatePreset, UpdateManagedLibrarySettings,
};
pub use legacy::{Permission, PlaylistShare, TrackMetadata};
pub use playback_state::{PlaybackState, RepeatMode, TransferPlayback, UpdatePlaybackState};
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
