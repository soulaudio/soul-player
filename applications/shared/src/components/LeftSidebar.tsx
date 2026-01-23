'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  useCurrentTrack,
  useIsPlaying,
  usePlayerProgress,
  useVolume,
  usePlayerModes,
} from '../stores/player';
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext';
import { cn } from '../lib/utils';
import { usePlatform } from '../contexts/PlatformContext';
import { useBackend } from '../contexts/BackendContext';
import { useResizableSidebar } from '../hooks/useResizableSidebar';
import { debug } from '../utils/debug';
import { useTranslation } from 'react-i18next';
import {
  NavBar,
  QueueSection,
  PlayerPanel,
  SettingsFooter,
  type ShuffleMode,
  type RepeatMode,
} from './sidebar';

interface LeftSidebarProps {
  /** Callback when the "Add to Playlist" button is clicked */
  onAddToPlaylist?: () => void;
}

export function LeftSidebar({ onAddToPlaylist }: LeftSidebarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { features } = usePlatform();
  const backend = useBackend();
  const [queue, setQueue] = useState<QueueTrack[]>([]);
  const [homeEnabled, setHomeEnabled] = useState(true);
  const [version, setVersion] = useState<string>('');
  const currentTrack = useCurrentTrack();
  const isPlaying = useIsPlaying();
  const { progress, duration } = usePlayerProgress();
  const volume = useVolume();
  const { shuffleMode, repeatMode, setShuffleMode, setRepeatMode } = usePlayerModes();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();

  // Resizable sidebar hook
  const { width, isResizing, handleMouseDown, resizableRef } = useResizableSidebar({
    minWidth: 240,
    maxWidth: 480,
    defaultWidth: 288,
  });

  const queueScrollRef = useRef<HTMLDivElement>(null);

  // Scroll queue to bottom (with requestAnimationFrame to ensure DOM is updated)
  const scrollQueueToBottom = useCallback(() => {
    requestAnimationFrame(() => {
      if (queueScrollRef.current) {
        queueScrollRef.current.scrollTop = queueScrollRef.current.scrollHeight;
      }
    });
  }, []);

  useEffect(() => {
    loadQueue();
    const unsubscribe = events.onQueueUpdate(() => {
      loadQueue();
    });
    return unsubscribe;
  }, [commands, events]);

  // Scroll queue to bottom when track changes
  useEffect(() => {
    let scrollTimeout: NodeJS.Timeout | null = null;

    const unsubscribe = events.onTrackChange(() => {
      // Clear any pending scroll timeout
      if (scrollTimeout) {
        clearTimeout(scrollTimeout);
      }

      // Small delay to allow queue to update first
      scrollTimeout = setTimeout(() => {
        scrollQueueToBottom();
        scrollTimeout = null;
      }, 50);
    });

    return () => {
      // Cleanup: clear pending timeout and unsubscribe from events
      if (scrollTimeout) {
        clearTimeout(scrollTimeout);
      }
      unsubscribe();
    };
  }, [events, scrollQueueToBottom]);

  // Load home page enabled setting
  useEffect(() => {
    const loadHomeEnabled = () => {
      backend
        .getUserSetting('home.enabled')
        .then((value) => {
          setHomeEnabled(value ?? true);
        })
        .catch((err) => debug.error('Failed to load home.enabled setting:', err));
    };

    // Load on mount
    loadHomeEnabled();

    // Listen for changes from settings page
    const handleHomeEnabledChanged = (event: Event) => {
      const customEvent = event as CustomEvent<{ enabled: boolean }>;
      setHomeEnabled(customEvent.detail.enabled);
    };

    window.addEventListener('home-enabled-changed', handleHomeEnabledChanged);

    return () => {
      window.removeEventListener('home-enabled-changed', handleHomeEnabledChanged);
    };
  }, [backend]);

  // Load version
  useEffect(() => {
    backend
      .getVersion()
      .then((v) => setVersion(v))
      .catch((err) => debug.error('Failed to load version:', err));
  }, [backend]);

  const loadQueue = async () => {
    try {
      const queueData = await commands.getQueue();
      setQueue(queueData);
    } catch (error) {
      debug.error('[LeftSidebar] Failed to load queue:', error);
    }
  };

  const handleQueueItemClick = async (originalIndex: number) => {
    try {
      await commands.skipToQueueIndex(originalIndex);
    } catch (error) {
      debug.error('[LeftSidebar] Failed to skip to queue index:', error);
    }
  };

  const handleShuffleModeChange = (mode: ShuffleMode) => {
    setShuffleMode(mode);
  };

  const handleRepeatModeChange = (mode: RepeatMode) => {
    setRepeatMode(mode);
  };

  return (
    <div
      ref={resizableRef}
      className="bg-card border-r border-border flex flex-col h-full relative"
      style={{ width: `${width}px` }}
    >
      {/* Resize Handle */}
      <div
        className={cn(
          'absolute top-0 right-0 w-1 h-full cursor-ew-resize group z-50',
          isResizing && 'bg-primary/50'
        )}
        onMouseDown={handleMouseDown}
        title={t('sidebar.resize', 'Resize sidebar')}
      >
        <div
          className={cn(
            'absolute inset-y-0 right-0 w-[3px] bg-primary/0 group-hover:bg-primary/30 transition-colors',
            isResizing && 'bg-primary/50'
          )}
        />
      </div>

      {/* Navigation - fixed at top */}
      <NavBar homeEnabled={homeEnabled} />

      {/* Queue Section - flexible, fills available space */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <QueueSection
          queue={queue}
          currentTrackId={currentTrack?.id}
          scrollRef={queueScrollRef}
          onTrackClick={handleQueueItemClick}
        />
      </div>

      {/* Player Panel - fixed at bottom */}
      <PlayerPanel
        currentTrack={
          currentTrack
            ? {
                id: currentTrack.id,
                title: currentTrack.title,
                artist: currentTrack.artist,
                album: currentTrack.album,
                coverArtPath: currentTrack.coverArtPath,
              }
            : null
        }
        isPlaying={isPlaying}
        progress={progress}
        duration={duration}
        volume={volume}
        shuffleMode={shuffleMode}
        repeatMode={repeatMode}
        hasRealDevices={features.hasRealAudioDevices}
        canCreatePlaylists={features.canCreatePlaylists}
        onTrackClick={() => navigate('/now-playing')}
        onAddToPlaylist={onAddToPlaylist}
        onShuffleModeChange={handleShuffleModeChange}
        onRepeatModeChange={handleRepeatModeChange}
      />

      {/* Settings Footer - bottom of sidebar */}
      <SettingsFooter version={version} />
    </div>
  );
}
