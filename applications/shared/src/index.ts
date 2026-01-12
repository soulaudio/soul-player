// Types
export * from './types';

// i18n - internationalization
export { initI18n, useTranslation, Trans, I18nextProvider } from './i18n';

// Stores
export { usePlayerStore } from './stores/player';
export { useLibraryStore } from './stores/library';

// Hooks
// export { usePlatform } from './hooks/usePlatform'; // Temporarily disabled due to missing Tauri deps

// Contexts
export { PlayerCommandsProvider, usePlayerCommands, usePlaybackEvents } from './contexts/PlayerCommandsContext';
export type { PlayerCommandsInterface, PlaybackEventsInterface, PlayerContextValue, PlaybackCapabilities, QueueTrack, Source } from './contexts/PlayerCommandsContext';

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
  PlaybackContext as BackendPlaybackContext,
  SetArtworkParams,
} from './contexts/BackendContext';

export { ScrollVisibilityProvider, useScrollVisibility } from './contexts/ScrollVisibilityContext';

// Utils
export { cn, formatDuration, formatBytes, formatRelativeTime, debounce, throttle } from './lib/utils';
export { getPlatform, isMac, getModifierKey, getModifierKeyName, formatShortcut } from './lib/platform';
export type { Platform } from './lib/platform';
export { removeConsecutiveDuplicates, removeAllDuplicates } from './utils/queue';
export { groupTracks, getDeduplicatedTracks, getFormatQualityScore } from './utils/trackGrouping';
export type { TrackForGrouping, GroupedTrack } from './utils/trackGrouping';
// export { commands, playerCommands, libraryCommands, playlistCommands } from './lib/tauri'; // Temporarily disabled

// Theme system
export { themeManager, ThemeManager } from './theme/ThemeManager';
export { builtInThemes, defaultTheme, lightTheme, darkTheme, oceanTheme } from './theme/themes';
export { ThemeProvider } from './theme/ThemeProvider';
export { useTheme } from './theme/useTheme';
export * from './theme/types';

// Player components
export { PlayerFooter } from './components/player/PlayerFooter';
export { PlayerControls } from './components/player/PlayerControls';
export { TrackInfo } from './components/player/TrackInfo';
export { ProgressBar } from './components/player/ProgressBar';
export { VolumeControl } from './components/player/VolumeControl';
export { ShuffleRepeatControls } from './components/player/ShuffleRepeatControls';
export { DeviceSelector } from './components/player/DeviceSelector';

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
export { LibraryPage } from './pages/LibraryPage';
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

// Shared components
export { AlbumCard, type AlbumCardAlbum } from './components/AlbumCard';
export { PlaylistCard } from './components/PlaylistCard';
export { LibraryPageLayout } from './components/LibraryPageLayout';
export { SkeletonCard } from './components/SkeletonCard';
export { ProgressiveImage } from './components/ProgressiveImage';
export { VirtualizedGrid } from './components/VirtualizedGrid';

// Hooks
export { useSeekBar, setIgnorePositionUpdates, shouldIgnorePositionUpdates } from './hooks/useSeekBar';
export { useGridScale } from './hooks/useGridScale';
export { useInfiniteLibrary } from './hooks/useInfiniteLibrary';

// TanStack Query exports
export { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// Demo components (for marketing site)
// export { DemoView } from '../desktop/src/components/DemoView'; // Temporarily disabled
