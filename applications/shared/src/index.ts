// Types
export * from './types';

// Stores
export { usePlayerStore } from './stores/player';
export { useLibraryStore } from './stores/library';

// Hooks
// export { usePlatform } from './hooks/usePlatform'; // Temporarily disabled due to missing Tauri deps

// Contexts
export { PlayerCommandsProvider, usePlayerCommands, usePlaybackEvents } from './contexts/PlayerCommandsContext';
export type { PlayerCommandsInterface, PlaybackEventsInterface, PlayerContextValue, PlaybackCapabilities, QueueTrack, Source } from './contexts/PlayerCommandsContext';

// Utils
export { cn, formatDuration, formatBytes, formatRelativeTime, debounce, throttle } from './lib/utils';
export { getPlatform, isMac, getModifierKey, getModifierKeyName, formatShortcut } from './lib/platform';
export type { Platform } from './lib/platform';
export { removeConsecutiveDuplicates, removeAllDuplicates } from './utils/queue';
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
export { SourcesDialog } from './components/SourcesDialog';
export { TrackList } from './components/TrackList';
export type { Track } from './components/TrackList';
export { ArtworkImage } from './components/ArtworkImage';

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

// Hooks
export { useSeekBar, setIgnorePositionUpdates, shouldIgnorePositionUpdates } from './hooks/useSeekBar';

// Demo components (for marketing site)
// export { DemoView } from '../desktop/src/components/DemoView'; // Temporarily disabled
