'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion, LayoutGroup } from 'framer-motion';
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Shuffle,
  Repeat,
  Repeat1,
  Volume2,
  VolumeX,
  Speaker,
  Check,
  Settings,
  Music,
  Heart,
} from 'lucide-react';
import { usePlayerStore } from '../stores/player';
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext';
import { ArtworkImage } from './ArtworkImage';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';
import { cn } from '../lib/utils';
import { usePlatform } from '../contexts/PlatformContext';

interface NavItem {
  id: string;
  labelKey: string;
  path: string;
}

interface AudioDevice {
  name: string;
  backend: string;
  isDefault: boolean;
  sampleRate?: number;
  channels?: number;
  isRunning: boolean;
}

interface AudioBackend {
  backend: string;
  name: string;
  description: string;
  available: boolean;
  isDefault: boolean;
  deviceCount: number;
}

const navigationItems: NavItem[] = [
  { id: 'home', labelKey: 'nav.home', path: '/' },
  { id: 'albums', labelKey: 'library.tab.albums', path: '/albums' },
  { id: 'artists', labelKey: 'library.tab.artists', path: '/artists' },
  { id: 'playlists', labelKey: 'library.tab.playlists', path: '/playlists' },
  { id: 'tracks', labelKey: 'library.tab.tracks', path: '/tracks' },
];

const MOCK_DEVICES: { backend: string; name: string; devices: AudioDevice[] }[] = [
  {
    backend: 'System',
    name: 'System Default',
    devices: [
      { name: 'System Default', backend: 'System', isDefault: true, sampleRate: 48000, channels: 2, isRunning: true },
    ],
  },
];

// Shared track item component with layoutId for animations
interface TrackItemProps {
  trackId: string | number;
  title: string;
  artist: string;
  coverArtPath?: string;
  album?: string;
  isLarge?: boolean;
  isPlaying?: boolean;
  showEqualizer?: boolean;
  onClick?: () => void;
}

function TrackItem({ trackId, title, artist, coverArtPath, album, isLarge, isPlaying, showEqualizer, onClick }: TrackItemProps) {
  // Ensure consistent string conversion for layoutId matching
  const layoutId = `sidebar-track-${String(trackId)}`;

  return (
    <motion.div
      layoutId={layoutId}
      layout="position"
      className={cn(
        "flex items-center group/track",
        isLarge ? "gap-3" : "gap-2",
        onClick && "cursor-pointer"
      )}
      onClick={onClick}
      transition={{
        layout: {
          type: "tween",
          duration: 0.25,
          ease: [0.4, 0, 0.2, 1], // Material Design standard easing
        },
      }}
    >
      <div
        className={cn(
          "bg-muted rounded overflow-hidden flex-shrink-0 relative",
          isLarge ? "w-12 h-12" : "w-8 h-8"
        )}
      >
        <ArtworkImage
          trackId={trackId}
          coverArtPath={coverArtPath}
          alt={album || 'Album art'}
          className="w-full h-full object-cover"
          fallbackClassName="w-full h-full flex items-center justify-center"
        />
        {showEqualizer && isPlaying && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/30">
            <div className="flex items-end gap-[2px] h-3">
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite]" />
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite_0.2s]" />
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite_0.4s]" />
            </div>
          </div>
        )}
      </div>
      <div
        className={cn(
          "flex-1 min-w-0",
          onClick && "group-hover/track:text-foreground"
        )}
      >
        <div className="text-sm truncate">{title}</div>
        <div className={cn(
          "text-xs truncate transition-colors",
          onClick ? "text-muted-foreground/70 group-hover/track:text-muted-foreground" : "text-muted-foreground"
        )}>{artist}</div>
      </div>
    </motion.div>
  );
}

interface LeftSidebarProps {
  /** Callback when the "Add to Playlist" button is clicked */
  onAddToPlaylist?: () => void;
}

export function LeftSidebar({ onAddToPlaylist }: LeftSidebarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const { features } = usePlatform();
  const [queue, setQueue] = useState<QueueTrack[]>([]);
  const {
    currentTrack,
    isPlaying,
    progress,
    duration,
    volume,
    shuffleEnabled,
    repeatMode,
    toggleShuffle,
    setRepeatMode,
  } = usePlayerStore();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();

  const [isMuted, setIsMuted] = useState(false);
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const queueScrollRef = useRef<HTMLDivElement>(null);

  // Scroll queue to bottom (with requestAnimationFrame to ensure DOM is updated)
  const scrollQueueToBottom = useCallback(() => {
    requestAnimationFrame(() => {
      if (queueScrollRef.current) {
        queueScrollRef.current.scrollTop = queueScrollRef.current.scrollHeight;
      }
    });
  }, []);

  // Device selector state
  const [currentDevice, setCurrentDevice] = useState<AudioDevice | null>(null);
  const [backends, setBackends] = useState<AudioBackend[]>([]);
  const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map());
  const [isLoadingDevices, setIsLoadingDevices] = useState(false);

  // Use feature flags to determine if real audio devices are available
  const hasRealDevices = features.hasRealAudioDevices;

  useEffect(() => {
    loadQueue();
    const unsubscribe = events.onQueueUpdate(() => {
      loadQueue();
    });
    return unsubscribe;
  }, [commands, events]);

  // Scroll queue to bottom when track changes
  useEffect(() => {
    const unsubscribe = events.onTrackChange(() => {
      // Small delay to allow queue to update first
      setTimeout(() => {
        scrollQueueToBottom();
      }, 50);
    });
    return unsubscribe;
  }, [events, scrollQueueToBottom]);

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume);
    }
  }, [volume, isMuted]);

  useEffect(() => {
    if (!hasRealDevices) {
      setCurrentDevice(MOCK_DEVICES[0].devices[0]);
    } else {
      loadCurrentDevice();
    }
  }, [hasRealDevices]);

  const loadCurrentDevice = async () => {
    try {
      if (!commands?.getCurrentAudioDevice) return;
      const device = await commands.getCurrentAudioDevice();
      setCurrentDevice(device);
    } catch (error) {
      console.error('[LeftSidebar] Failed to load current device:', error);
    }
  };

  const loadDevices = async () => {
    if (!hasRealDevices) {
      const deviceMap = new Map<string, AudioDevice[]>();
      MOCK_DEVICES.forEach(mock => {
        deviceMap.set(mock.backend, mock.devices);
      });
      setDevices(deviceMap);
      return;
    }

    if (isLoadingDevices) return;
    setIsLoadingDevices(true);

    try {
      if (commands?.getAudioBackends) {
        const backendList = await commands.getAudioBackends();
        setBackends(backendList);

        const deviceMap = new Map<string, AudioDevice[]>();
        for (const backend of backendList) {
          if (backend.available && commands?.getAudioDevices) {
            try {
              const backendDevices = await commands.getAudioDevices(backend.backend);
              deviceMap.set(backend.backend, backendDevices);
            } catch (error) {
              console.error(`[LeftSidebar] Failed to load devices for ${backend.backend}:`, error);
            }
          }
        }
        setDevices(deviceMap);
      }
    } catch (error) {
      console.error('[LeftSidebar] Failed to load devices:', error);
    } finally {
      setIsLoadingDevices(false);
    }
  };

  const switchDevice = async (backend: string, deviceName: string) => {
    if (!hasRealDevices) {
      if (backend === 'System') {
        setCurrentDevice(MOCK_DEVICES[0].devices[0]);
      }
      return;
    }

    try {
      if (!commands?.setAudioDevice) return;
      await commands.setAudioDevice(backend, deviceName);
      await loadCurrentDevice();
    } catch (error) {
      console.error('[LeftSidebar] Failed to switch device:', error);
    }
  };

  const loadQueue = async () => {
    try {
      const queueData = await commands.getQueue();
      setQueue(queueData);
    } catch (error) {
      console.error('[LeftSidebar] Failed to load queue:', error);
    }
  };

  const handleQueueItemClick = async (originalIndex: number) => {
    try {
      await commands.skipToQueueIndex(originalIndex);
    } catch (error) {
      console.error('[LeftSidebar] Failed to skip to queue index:', error);
    }
  };

  const handlePlayPause = useCallback(async () => {
    try {
      if (isPlaying) {
        await commands.pausePlayback();
      } else {
        await commands.resumePlayback();
      }
    } catch (error) {
      console.error('[LeftSidebar] Failed to toggle playback:', error);
    }
  }, [isPlaying, commands]);

  const handlePrevious = useCallback(async () => {
    try {
      await commands.skipPrevious();
    } catch (error) {
      console.error('[LeftSidebar] Failed to skip previous:', error);
    }
  }, [commands]);

  const handleNext = useCallback(async () => {
    try {
      await commands.skipNext();
    } catch (error) {
      console.error('[LeftSidebar] Failed to skip next:', error);
    }
  }, [commands]);

  const handleSeek = useCallback(
    async (e: React.MouseEvent<HTMLDivElement>) => {
      if (!duration) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const percentage = x / rect.width;
      const newPosition = percentage * duration;
      try {
        await commands.seek(newPosition);
      } catch (error) {
        console.error('[LeftSidebar] Failed to seek:', error);
      }
    },
    [duration, commands]
  );

  const handleShuffleToggle = async () => {
    const newValue = !shuffleEnabled;
    toggleShuffle();
    try {
      await commands.setShuffle(newValue);
    } catch (error) {
      console.error('[LeftSidebar] Set shuffle failed:', error);
      toggleShuffle();
    }
  };

  const handleRepeatToggle = async () => {
    const nextMode = repeatMode === 'off' ? 'all' : repeatMode === 'all' ? 'one' : 'off';
    setRepeatMode(nextMode);
    try {
      await commands.setRepeatMode(nextMode);
    } catch (error) {
      console.error('[LeftSidebar] Set repeat mode failed:', error);
      const prevMode = nextMode === 'off' ? 'one' : nextMode === 'all' ? 'off' : 'all';
      setRepeatMode(prevMode);
    }
  };

  const applyVolumeChange = useCallback(
    (newVolume: number) => {
      const clampedVolume = Math.max(0, Math.min(1, newVolume));
      usePlayerStore.getState().setVolume(clampedVolume);

      if (clampedVolume > 0 && isMuted) {
        setIsMuted(false);
      }

      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }

      debounceTimerRef.current = setTimeout(() => {
        commands.setVolume(clampedVolume).catch((error) => {
          console.error('[LeftSidebar] Set volume failed:', error);
        });
      }, 150);
    },
    [commands, isMuted]
  );

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    applyVolumeChange(parseFloat(e.target.value));
  };

  const handleMuteToggle = async () => {
    try {
      if (isMuted) {
        await commands.setVolume(volumeBeforeMute);
        usePlayerStore.getState().setVolume(volumeBeforeMute);
        setIsMuted(false);
      } else {
        setVolumeBeforeMute(volume);
        await commands.setVolume(0);
        usePlayerStore.getState().setVolume(0);
        setIsMuted(true);
      }
    } catch (error) {
      console.error('[LeftSidebar] Mute toggle failed:', error);
    }
  };

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/';
    }
    return location.pathname === path || location.pathname.startsWith(path + '/');
  };

  // Get current track ID for filtering
  const currentTrackId = currentTrack?.id;

  // Filter out current track from queue, reverse so items closest to now playing are at bottom
  const displayQueue = queue
    .filter(t => String(t.trackId) !== String(currentTrackId))
    .reverse();

  // progress is already a percentage (0-100) from the store
  const progressPercentage = progress;
  // Calculate current position in seconds for time display
  const currentPositionSeconds = duration > 0 ? (progress / 100) * duration : 0;
  const displayVolume = isMuted ? 0 : volume;

  const formatTime = (seconds: number) => {
    if (!seconds || !isFinite(seconds)) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  // Handle mouse wheel for volume control
  const handleVolumeWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -0.05 : 0.05; // Scroll down = decrease, scroll up = increase
    applyVolumeChange(volume + delta);
  }, [volume, applyVolumeChange]);

  return (
    <div className="w-72 bg-card border-r border-border flex flex-col h-full">
      {/* Navigation */}
      <nav className="p-4 pt-6">
        <ul className="space-y-1">
          {navigationItems.map((item) => (
            <li key={item.id}>
              <button
                onClick={() => navigate(item.path)}
                className={cn(
                  "w-full text-left px-3 py-2 text-xl font-bold transition-colors",
                  isActive(item.path) ? 'text-primary' : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {t(item.labelKey)}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      {/* Queue Section - can expand/contract */}
      <div className="mt-auto flex flex-col">
        <LayoutGroup>
          {displayQueue.length > 0 && (
            <div className="flex flex-col max-h-[40vh] group/queue">
              <div className="px-4 py-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                {t('sidebar.queue')}
              </div>
              {/* Scrollable queue */}
              <motion.div
                ref={queueScrollRef}
                layoutScroll
                className="flex-1 overflow-y-auto px-4 pb-2 queue-scrollbar"
              >
                <div className="flex flex-col justify-end min-h-full gap-1">
                  {displayQueue.map((track) => {
                    const originalIndex = queue.findIndex(q => q.trackId === track.trackId);
                    return (
                      <TrackItem
                        key={String(track.trackId)}
                        trackId={track.trackId}
                        title={track.title}
                        artist={track.artist}
                        coverArtPath={track.coverArtPath}
                        album={track.album ?? undefined}
                        onClick={() => handleQueueItemClick(originalIndex)}
                      />
                    );
                  })}
                </div>
              </motion.div>
            </div>
          )}

          {/* Now Playing Section - completely static, fixed at bottom */}
          <div className="p-4">
            <div className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
              {t('sidebar.nowPlaying')}
            </div>

            {/* Fixed height container for track item */}
            <div className="h-12 flex items-center gap-2">
              <div className="flex-1 min-w-0">
                {currentTrack ? (
                  <TrackItem
                    key={String(currentTrack.id)}
                    trackId={currentTrack.id}
                    title={currentTrack.title}
                    artist={currentTrack.artist}
                    coverArtPath={currentTrack.coverArtPath}
                    album={currentTrack.album}
                    isLarge
                    isPlaying={isPlaying}
                    showEqualizer
                    onClick={() => navigate('/now-playing')}
                  />
                ) : (
                  <div className="flex items-center gap-3 text-muted-foreground h-12">
                    <div className="w-12 h-12 bg-muted rounded flex items-center justify-center">
                      <Music className="w-6 h-6 opacity-50" />
                    </div>
                    <span className="text-sm">{t('sidebar.noTrackPlaying')}</span>
                  </div>
                )}
              </div>
              <button
                onClick={onAddToPlaylist}
                disabled={!currentTrack || !features.canCreatePlaylists}
                className={cn(
                  "p-1.5 transition-colors text-muted-foreground flex-shrink-0",
                  currentTrack && features.canCreatePlaylists ? "hover:text-foreground" : "opacity-50 cursor-not-allowed"
                )}
                title={features.canCreatePlaylists
                  ? t('playlist.addToPlaylist', 'Add to Playlist')
                  : t('settings.demoDisabled', 'Available in desktop app')
                }
              >
                <Heart className="w-4 h-4" />
              </button>
            </div>

            {/* Controls */}
            <div className="mt-4 space-y-3">
              {/* Progress */}
              <div>
                <div
                  className={cn(
                    "py-2 -my-2",
                    currentTrack ? "cursor-pointer" : "cursor-default"
                  )}
                  onClick={currentTrack ? handleSeek : undefined}
                >
                  <div
                    className={cn(
                      "h-1.5 bg-muted rounded-full overflow-hidden",
                      !currentTrack && "opacity-50"
                    )}
                  >
                    <div
                      className="h-full bg-primary rounded-full transition-[width] duration-150"
                      style={{ width: `${progressPercentage}%` }}
                    />
                  </div>
                </div>
                <div className="flex justify-between mt-1 text-[10px] text-muted-foreground font-mono">
                  <span>{formatTime(currentPositionSeconds)}</span>
                  <span>{formatTime(duration)}</span>
                </div>
              </div>

              {/* Playback Controls - Grid layout to keep play/pause centered */}
              <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-1">
                {/* Left group */}
                <div className="flex items-center justify-end gap-1">
                  <button
                    onClick={handleShuffleToggle}
                    disabled={!currentTrack}
                    className={cn(
                      "p-1.5 transition-colors",
                      !currentTrack && "opacity-50 cursor-not-allowed",
                      shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground disabled:hover:text-muted-foreground'
                    )}
                  >
                    <Shuffle className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={handlePrevious}
                    disabled={!currentTrack}
                    className={cn(
                      "p-1.5 text-muted-foreground transition-colors",
                      currentTrack ? "hover:text-foreground" : "opacity-50 cursor-not-allowed"
                    )}
                  >
                    <SkipBack className="w-4 h-4" />
                  </button>
                </div>

                {/* Center - Play/Pause */}
                <button
                  onClick={handlePlayPause}
                  disabled={!currentTrack}
                  className={cn(
                    "w-8 h-8 bg-primary text-primary-foreground rounded-full transition-colors flex items-center justify-center",
                    currentTrack ? "hover:bg-primary/90" : "opacity-50 cursor-not-allowed"
                  )}
                >
                  {isPlaying ? (
                    <Pause className="w-4 h-4" />
                  ) : (
                    <Play className="w-4 h-4 translate-x-[1px]" />
                  )}
                </button>

                {/* Right group */}
                <div className="flex items-center justify-start gap-1">
                  <button
                    onClick={handleNext}
                    disabled={!currentTrack}
                    className={cn(
                      "p-1.5 text-muted-foreground transition-colors",
                      currentTrack ? "hover:text-foreground" : "opacity-50 cursor-not-allowed"
                    )}
                  >
                    <SkipForward className="w-4 h-4" />
                  </button>
                  <button
                    onClick={handleRepeatToggle}
                    disabled={!currentTrack}
                    className={cn(
                      "p-1.5 transition-colors",
                      !currentTrack && "opacity-50 cursor-not-allowed",
                      repeatMode !== 'off' ? 'text-primary' : 'text-muted-foreground hover:text-foreground disabled:hover:text-muted-foreground'
                    )}
                  >
                    {repeatMode === 'one' ? <Repeat1 className="w-3.5 h-3.5" /> : <Repeat className="w-3.5 h-3.5" />}
                  </button>
                </div>
              </div>

              {/* Volume + Device */}
              <div className="flex items-center gap-2" onWheel={handleVolumeWheel}>
                <button
                  onClick={handleMuteToggle}
                  className="p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  {isMuted || volume === 0 ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
                </button>
                <div className="flex-1 relative h-4 flex items-center cursor-pointer group">
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={displayVolume}
                    onChange={handleVolumeChange}
                    className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
                  />
                  <div className="absolute inset-x-0 h-1 bg-muted rounded-full" />
                  <div
                    className="absolute left-0 h-1 bg-primary rounded-full"
                    style={{ width: `${displayVolume * 100}%` }}
                  />
                </div>
                <span className="text-[10px] text-muted-foreground font-mono w-6 text-right">
                  {Math.round(displayVolume * 100)}
                </span>
                <DropdownMenu onOpenChange={(open) => { if (open) loadDevices(); }}>
                  <DropdownMenuTrigger asChild>
                    <button
                      className="p-1 text-muted-foreground hover:text-foreground transition-colors ml-1"
                      title={currentDevice?.name || 'Select audio device'}
                    >
                      <Speaker className={cn("w-4 h-4", currentDevice?.isRunning && "text-primary")} />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-[280px] max-h-[300px] overflow-y-auto">
                    <DropdownMenuLabel className="flex items-center justify-between">
                      <span>Audio Output</span>
                      {currentDevice?.sampleRate && (
                        <span className="text-xs font-normal text-muted-foreground">
                          {currentDevice.sampleRate}Hz
                        </span>
                      )}
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    {isLoadingDevices ? (
                      <div className="p-4 text-center text-sm text-muted-foreground">Loading...</div>
                    ) : !hasRealDevices ? (
                      MOCK_DEVICES.map((mockBackend) => (
                        <div key={mockBackend.backend}>
                          <DropdownMenuLabel className="text-xs uppercase text-muted-foreground">
                            {mockBackend.name}
                          </DropdownMenuLabel>
                          {mockBackend.devices.map((device) => (
                            <DropdownMenuItem
                              key={`${device.backend}-${device.name}`}
                              onClick={() => switchDevice(device.backend, device.name)}
                              className="flex items-center justify-between cursor-pointer"
                            >
                              <div className="flex flex-col min-w-0 flex-1">
                                <span className="text-sm truncate">{device.name}</span>
                                <span className="text-xs text-muted-foreground">{device.sampleRate}Hz</span>
                              </div>
                              {currentDevice?.name === device.name && <Check className="h-4 w-4 text-primary ml-2" />}
                            </DropdownMenuItem>
                          ))}
                        </div>
                      ))
                    ) : backends.length === 0 ? (
                      <div className="p-4 text-center text-sm text-muted-foreground">No audio devices found</div>
                    ) : (
                      backends.map((backend, index) => {
                        if (!backend.available) return null;
                        const backendDevices = devices.get(backend.backend) || [];
                        if (backendDevices.length === 0) return null;
                        return (
                          <div key={backend.backend}>
                            {backends.length > 1 && (
                              <DropdownMenuLabel className="text-xs uppercase text-muted-foreground">
                                {backend.name}
                              </DropdownMenuLabel>
                            )}
                            {backendDevices.map((device) => (
                              <DropdownMenuItem
                                key={`${device.backend}-${device.name}`}
                                onClick={() => switchDevice(device.backend, device.name)}
                                className="flex items-center justify-between cursor-pointer"
                              >
                                <div className="flex flex-col min-w-0 flex-1">
                                  <span className="text-sm truncate">{device.name}</span>
                                  {device.sampleRate && (
                                    <span className="text-xs text-muted-foreground">{device.sampleRate}Hz</span>
                                  )}
                                </div>
                                {currentDevice?.name === device.name && currentDevice?.backend === device.backend && (
                                  <Check className="h-4 w-4 text-primary ml-2" />
                                )}
                              </DropdownMenuItem>
                            ))}
                            {index < backends.filter(b => b.available).length - 1 && <DropdownMenuSeparator />}
                          </div>
                        );
                      })
                    )}
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          </div>
        </LayoutGroup>

        {/* Settings - bottom of sidebar */}
        <div className="border-t border-border">
          <button
            onClick={() => navigate('/settings')}
            className={cn(
              "w-full flex items-center gap-3 px-4 py-3 text-sm transition-colors",
              location.pathname === '/settings'
                ? 'text-primary bg-accent/20'
                : 'text-muted-foreground hover:text-foreground hover:bg-accent/10'
            )}
          >
            <Settings className="w-4 h-4" />
            <span>{t('nav.settings')}</span>
          </button>
        </div>
      </div>
    </div>
  );
}
