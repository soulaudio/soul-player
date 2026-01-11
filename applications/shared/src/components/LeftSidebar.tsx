'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
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
  Upload,
  FolderOpen,
  Settings,
  Music,
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
  { id: 'library', labelKey: 'nav.library', path: '/library' },
  { id: 'discovery', labelKey: 'nav.discovery', path: '/discovery' },
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

interface LeftSidebarProps {
  onImport?: () => void;
  onOpenSources?: () => void;
}

// Unified track item component for consistent structure
interface TrackItemProps {
  id: string | number;
  title: string;
  artist: string;
  coverArtPath?: string;
  album?: string;
  size: 'small' | 'large';
  isPlaying?: boolean;
  showPlayingIndicator?: boolean;
  onClick?: () => void;
}

function TrackItem({ id, title, artist, coverArtPath, album, size, isPlaying, showPlayingIndicator, onClick }: TrackItemProps) {
  const artworkSize = size === 'large' ? 'w-12 h-12' : 'w-8 h-8';
  const gap = size === 'large' ? 'gap-3' : 'gap-2';

  return (
    <motion.div
      layoutId={`track-${id}`}
      layout
      className={cn("flex items-center", gap, onClick && "cursor-pointer")}
      onClick={onClick}
      transition={{
        layout: { duration: 0.4, ease: [0.4, 0, 0.2, 1] }
      }}
    >
      <motion.div
        layoutId={`artwork-${id}`}
        className={cn(artworkSize, "bg-muted rounded overflow-hidden flex-shrink-0 relative")}
      >
        <ArtworkImage
          trackId={id}
          coverArtPath={coverArtPath}
          alt={album || 'Album art'}
          className="w-full h-full object-cover"
          fallbackClassName="w-full h-full flex items-center justify-center"
        />
        {showPlayingIndicator && isPlaying && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/30">
            <div className="flex items-end gap-[2px] h-3">
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite]" />
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite_0.2s]" />
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite_0.4s]" />
            </div>
          </div>
        )}
      </motion.div>
      <div className="flex-1 min-w-0">
        <motion.div layoutId={`title-${id}`} className="text-sm truncate">
          {title}
        </motion.div>
        <motion.div layoutId={`artist-${id}`} className="text-xs text-muted-foreground truncate">
          {artist}
        </motion.div>
      </div>
    </motion.div>
  );
}

export function LeftSidebar({ onImport, onOpenSources }: LeftSidebarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
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

  // Device selector state
  const [currentDevice, setCurrentDevice] = useState<AudioDevice | null>(null);
  const [backends, setBackends] = useState<AudioBackend[]>([]);
  const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map());
  const [isLoadingDevices, setIsLoadingDevices] = useState(false);

  const isBrowserDemo = !commands?.getCurrentAudioDevice;

  useEffect(() => {
    loadQueue();
    const unsubscribe = events.onQueueUpdate(() => {
      loadQueue();
    });
    return unsubscribe;
  }, [commands, events]);

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume);
    }
  }, [volume, isMuted]);

  useEffect(() => {
    if (isBrowserDemo) {
      setCurrentDevice(MOCK_DEVICES[0].devices[0]);
    } else {
      loadCurrentDevice();
    }
  }, [isBrowserDemo]);

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
    if (isBrowserDemo) {
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
    if (isBrowserDemo) {
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
    if (queueScrollRef.current) {
      queueScrollRef.current.scrollTo({
        top: queueScrollRef.current.scrollHeight,
        behavior: 'smooth',
      });
    }
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
    return location.pathname.startsWith(path);
  };

  // Filter out current track from queue and reverse
  const currentTrackId = currentTrack?.id;
  const filteredQueue = queue.filter(t => String(t.trackId) !== String(currentTrackId));
  const reversedQueue = [...filteredQueue].reverse();

  const progressPercentage = duration ? (progress / duration) * 100 : 0;
  const displayVolume = isMuted ? 0 : volume;

  const formatTime = (seconds: number) => {
    if (!seconds || !isFinite(seconds)) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="w-72 bg-card border-r border-border flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-end gap-1 px-3 py-2 border-b border-border">
        <button
          onClick={onImport}
          disabled={!onImport}
          className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          aria-label="Import Music"
          title="Import Music"
        >
          <Upload className="w-4 h-4" />
        </button>
        <button
          onClick={onOpenSources}
          className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
          aria-label="Manage Sources"
          title="Manage Sources"
        >
          <FolderOpen className="w-4 h-4" />
        </button>
        <button
          onClick={() => navigate('/settings')}
          className={cn(
            "p-1.5 rounded-lg transition-colors",
            location.pathname === '/settings'
              ? 'text-primary bg-accent'
              : 'text-muted-foreground hover:text-foreground hover:bg-accent'
          )}
          aria-label="Settings"
          title="Settings"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>

      {/* Navigation */}
      <nav className="p-4">
        <ul className="space-y-2">
          {navigationItems.map((item) => (
            <li key={item.id}>
              <button
                onClick={() => navigate(item.path)}
                className={cn(
                  "w-full text-left px-3 py-2 text-xl font-bold",
                  isActive(item.path) ? 'text-primary' : 'text-muted-foreground'
                )}
              >
                {t(item.labelKey)}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Queue + Now Playing */}
      <div className="flex flex-col">
        {/* Queue */}
        <AnimatePresence mode="popLayout">
          {reversedQueue.length > 0 && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="flex flex-col max-h-[40%] group/queue"
            >
              <div className="px-4 py-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                {t('sidebar.queue')}
              </div>
              <div
                ref={queueScrollRef}
                className="overflow-y-auto px-4 queue-scrollbar"
              >
                {reversedQueue.map((track, reversedIndex) => {
                  const originalIndex = queue.findIndex(q => q.trackId === track.trackId);
                  return (
                    <div key={track.trackId} className="py-1">
                      <TrackItem
                        id={track.trackId}
                        title={track.title}
                        artist={track.artist}
                        coverArtPath={track.coverArtPath}
                        album={track.album}
                        size="small"
                        onClick={() => handleQueueItemClick(originalIndex)}
                      />
                    </div>
                  );
                })}
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Now Playing */}
        <div className="p-4">
          <div className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
            {t('sidebar.nowPlaying')}
          </div>

          <div className="min-h-[48px]">
            <AnimatePresence mode="wait">
              {currentTrack ? (
                <motion.div
                  key={currentTrack.id}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.2 }}
                >
                  <TrackItem
                    id={currentTrack.id}
                    title={currentTrack.title}
                    artist={currentTrack.artist}
                    coverArtPath={currentTrack.coverArtPath}
                    album={currentTrack.album}
                    size="large"
                    isPlaying={isPlaying}
                    showPlayingIndicator
                  />
                </motion.div>
              ) : (
                <motion.div
                  key="empty"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="flex items-center gap-3 text-muted-foreground"
                >
                  <div className="w-12 h-12 bg-muted rounded flex items-center justify-center">
                    <Music className="w-6 h-6 opacity-50" />
                  </div>
                  <span className="text-sm">{t('sidebar.noTrackPlaying')}</span>
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Controls */}
          {currentTrack && (
            <div className="mt-4 space-y-3">
              {/* Progress */}
              <div>
                <div
                  className="h-1 bg-muted rounded-full cursor-pointer overflow-hidden"
                  onClick={handleSeek}
                >
                  <div
                    className="h-full bg-primary rounded-full transition-[width] duration-150"
                    style={{ width: `${progressPercentage}%` }}
                  />
                </div>
                <div className="flex justify-between mt-1 text-[10px] text-muted-foreground font-mono">
                  <span>{formatTime(progress)}</span>
                  <span>{formatTime(duration)}</span>
                </div>
              </div>

              {/* Playback Controls */}
              <div className="flex items-center justify-center gap-3">
                <button
                  onClick={handleShuffleToggle}
                  className={cn(
                    "p-1.5 transition-colors",
                    shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'
                  )}
                >
                  <Shuffle className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={handlePrevious}
                  className="p-1.5 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <SkipBack className="w-4 h-4" />
                </button>
                <button
                  onClick={handlePlayPause}
                  className="w-8 h-8 bg-primary text-primary-foreground rounded-full hover:bg-primary/90 transition-colors flex items-center justify-center"
                >
                  {isPlaying ? (
                    <Pause className="w-4 h-4" />
                  ) : (
                    <Play className="w-4 h-4 translate-x-[1px]" />
                  )}
                </button>
                <button
                  onClick={handleNext}
                  className="p-1.5 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <SkipForward className="w-4 h-4" />
                </button>
                <button
                  onClick={handleRepeatToggle}
                  className={cn(
                    "p-1.5 transition-colors",
                    repeatMode !== 'off' ? 'text-primary' : 'text-muted-foreground hover:text-foreground'
                  )}
                >
                  {repeatMode === 'one' ? <Repeat1 className="w-3.5 h-3.5" /> : <Repeat className="w-3.5 h-3.5" />}
                </button>
              </div>

              {/* Volume + Device */}
              <div className="flex items-center gap-2">
                <button
                  onClick={handleMuteToggle}
                  className="p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  {isMuted || volume === 0 ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
                </button>
                <div className="flex-1 relative h-1">
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={displayVolume}
                    onChange={handleVolumeChange}
                    className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
                  />
                  <div className="absolute inset-0 bg-muted rounded-full" />
                  <div
                    className="absolute inset-y-0 left-0 bg-primary rounded-full"
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
                    ) : isBrowserDemo ? (
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
          )}
        </div>
      </div>
    </div>
  );
}
