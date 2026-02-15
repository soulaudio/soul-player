// Types
export * from './types';

// i18n - internationalization
export { initI18n, useTranslation, Trans, I18nextProvider } from './i18n';

// Stores
export {
  usePlayerStore,
  // Optimized selector hooks (CRITICAL for performance)
  useCurrentTrack,
  useIsPlaying,
  useProgress,
  useDuration,
  useVolume,
  useShuffleMode,
  useRepeatMode,
  useQueue,
  useQueueIndex,
  // Composite selectors
  usePlayerPlayback,
  usePlayerProgress,
  usePlayerModes,
} from './stores/player';
export { useLibraryStore } from './stores/library';

// Hooks
// export { usePlatform } from './hooks/usePlatform'; // Temporarily disabled due to missing Tauri deps
export { useAddTrackToPlaylist, useRemoveTrackFromPlaylist, useCreatePlaylist, useDeletePlaylist } from './hooks/queries/usePlaylistMutations';
export { useDeleteTrack } from './hooks/queries/useTrackMutations';

// Contexts
export { PlayerCommandsProvider, usePlayerCommands, usePlaybackEvents } from './contexts/PlayerCommandsContext';
export type { PlayerCommandsInterface, PlaybackEventsInterface, PlayerContextValue, PlaybackCapabilities, QueueTrack, Source, QueueContext } from './contexts/PlayerCommandsContext';

export { LibraryDataProvider, useLibraryData } from './contexts/LibraryDataContext';
export type { LibraryDataInterface, Album, Artist, Playlist, Genre, LibraryTrack } from './contexts/LibraryDataContext';

export { PlatformProvider, usePlatform, useIsDesktop, useFeatures, DesktopOnly, WebOnly, FeatureGate } from './contexts/PlatformContext';
export type { PlatformType, PlatformContextValue } from './contexts/PlatformContext';

export { BackendProvider, useBackend } from './contexts/BackendContext';
export type {
  BackendInterface,
  BackendTrack,
  BackendAlbum,
  BackendArtist,
  BackendPlaylist,
  BackendGenre,
  DatabaseHealth,
  PlaybackContext,
  PlaybackContext as BackendPlaybackContext,
  SetArtworkParams,
  EffectSlot,
  EffectType,
  EqBand,
  CompressorSettings,
  LimiterSettings,
  CrossfeedSettings,
  StereoSettings,
  GraphicEqSettings,
  ConvolutionSettings,
  HeadroomSettings,
  LatencyInfo,
  ExclusiveConfig,
  AnalysisQueueStats,
  AnalysisWorkerStatus,
  AudioBackend,
  AudioDevice,
} from './contexts/BackendContext';

export { ScrollVisibilityProvider, useScrollVisibility } from './contexts/ScrollVisibilityContext';

export { PlaybackSessionProvider, usePlaybackSession } from './contexts/PlaybackSessionContext';
export type { PlaybackSession, PlaybackSessionContextValue } from './contexts/PlaybackSessionContext';

// Backend Providers
export { MockBackendProvider } from './providers/MockBackendProvider';
export { ServerBackendProvider } from './providers/ServerBackendProvider';

// Web Playback Provider (for marketing demo and future web player)
export { WebPlaybackProvider } from './providers/WebPlaybackProvider';

// Demo Storage
export { DemoStorage } from './lib/demo-storage';
export type { DemoTrack, DemoAlbum, DemoPlaylist, DemoData } from './lib/demo-storage';

// Storage interfaces
export type { PlaybackDataStorage } from './types/storage';

// Utils
export { cn, formatDuration } from './lib/utils';
export { getPlatform, isMac, getModifierKey, getModifierKeyName, formatShortcut } from './lib/platform';
export type { Platform } from './lib/platform';
export { removeConsecutiveDuplicates } from './utils/queue';
export { groupTracks, getDeduplicatedTracks, getFormatQualityScore } from './utils/trackGrouping';
export type { TrackForGrouping, GroupedTrack } from './utils/trackGrouping';
export { debug } from './utils/debug';
// export { commands, playerCommands, libraryCommands, playlistCommands } from './lib/tauri'; // Temporarily disabled

// Theme system
export { themeManager, ThemeManager } from './theme/ThemeManager';
export { builtInThemes, defaultTheme, lightTheme, darkTheme, oceanTheme } from './theme/themes';
export { ThemeProvider } from './theme/ThemeProvider';
export { ThemePicker } from './theme/components/ThemePicker';
export { useTheme } from './theme/useTheme';
export * from './theme/types';

// Player components (UNUSED - kept for potential future use)
// export { PlayerFooter } from './components/player/PlayerFooter'; // DELETED - dead code
export { PlayerControls } from './components/player/PlayerControls';
export { TrackInfo } from './components/player/TrackInfo';
// NOTE: player/ProgressBar - Full-featured drag-to-seek variant (actively used in PlayerPanel).
// Self-contained with hooks, provides advanced seeking with AbortController cleanup.
export { ProgressBar } from './components/player/ProgressBar';
// export { VolumeControl } from './components/player/VolumeControl'; // DELETED - dead code
export { ShuffleRepeatControls } from './components/player/ShuffleRepeatControls';
export { DeviceSelector } from './components/sidebar/DeviceSelector';
export type { AudioDevice as DeviceSelectorAudioDevice, AudioBackend as DeviceSelectorAudioBackend, DeviceSelectorProps } from './components/sidebar/DeviceSelector';

// Other components
export { QueueSidebar } from './components/QueueSidebar';
export { TrackList } from './components/TrackList';
export type { Track, SourceType } from './components/TrackList';
export { TrackMenu } from './components/TrackMenu';
export { ArtworkImage, clearArtworkCache, clearAllArtworkCache } from './components/ArtworkImage';
export { TrackQualityBadge } from './components/TrackQualityBadge';
export { SourceIndicator } from './components/SourceIndicator';
export { EditArtworkDialog, type ArtworkEntityType } from './components/EditArtworkDialog';
export { AddToPlaylistDialog } from './components/AddToPlaylistDialog';
export { ImageCropper } from './components/ImageCropper';
export { ArtistLink } from './components/ArtistLink';
export { AlbumLink } from './components/AlbumLink';

// UI components
export { Kbd, KbdGroup } from './components/ui/Kbd';
export { Tooltip, TooltipButton } from './components/ui/Tooltip';
export { Button } from './components/ui/button';
export {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from './components/ui/dropdown-menu';
export {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogBody,
  DialogFooter,
  ConfirmDialog,
} from './components/ui/Dialog';

// Layouts
export { MainLayout } from './layouts/MainLayout';

// Pages (shared between platforms)
export { HomePage } from './pages/HomePage';
export { AlbumsPage } from './pages/AlbumsPage';
export { ArtistsPage } from './pages/ArtistsPage';
export { PlaylistsPage } from './pages/PlaylistsPage';
export { TracksPage } from './pages/TracksPage';
export { AlbumPage } from './pages/AlbumPage';
export { ArtistPage } from './pages/ArtistPage';
export { PlaylistPage } from './pages/PlaylistPage';
export { NowPlayingPage } from './pages/NowPlayingPage';
export { SettingsPage } from './pages/SettingsPage';
export type { SettingsHandlers, ShortcutsSettingsProps } from './pages/SettingsPage';
export { SettingsLayout } from './components/settings/SettingsLayout';
export { AudioSettingsPage } from './components/settings/AudioSettingsPage';
export { LibrarySettingsPage } from './components/settings/LibrarySettingsPage';
export { ReportBugSettingsPage } from './components/settings/ReportBugSettingsPage';
export { DataManagementSettingsPage } from './components/settings/DataManagementSettingsPage';

// Shared components
export { AlbumCard, type AlbumCardAlbum } from './components/AlbumCard';
export { PlaylistCard } from './components/PlaylistCard';
export { LibraryPageLayout } from './components/LibraryPageLayout';
export { SkeletonCard } from './components/SkeletonCard';
export { ProgressiveImage } from './components/ProgressiveImage';
export { VirtualizedGrid } from './components/VirtualizedGrid';
export { ScanProgressToast } from './components/ScanProgressToast';

// Hooks
export { useSeekBar } from './hooks/useSeekBar';
export { useGridScale } from './hooks/useGridScale';
export { useResponsiveColumns } from './hooks/useResponsiveColumns';
export { useInfiniteLibrary } from './hooks/useInfiniteLibrary';
export { useNavigateWithHistory } from './hooks/useNavigateWithHistory';

// TanStack Query exports
export { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// Demo components (for marketing site)
// export { DemoView } from '../desktop/src/components/DemoView'; // Temporarily disabled
