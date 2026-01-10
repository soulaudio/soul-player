import { useState, useEffect, useCallback, useRef } from 'react';
import { Speaker, Check } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../../stores/player';

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

// Mock devices for browser demo
const MOCK_DEVICES: { backend: string; name: string; devices: AudioDevice[] }[] = [
  {
    backend: 'System',
    name: 'System Default',
    devices: [
      { name: 'System Default', backend: 'System', isDefault: true, sampleRate: 48000, channels: 2, isRunning: true },
    ],
  },
  {
    backend: 'WASAPI',
    name: 'WASAPI (Desktop only)',
    devices: [
      { name: 'Speakers (Realtek Audio)', backend: 'WASAPI', isDefault: false, sampleRate: 48000, channels: 2, isRunning: false },
      { name: 'Headphones (USB Audio)', backend: 'WASAPI', isDefault: false, sampleRate: 96000, channels: 2, isRunning: false },
    ],
  },
  {
    backend: 'ASIO',
    name: 'ASIO (Desktop only)',
    devices: [
      { name: 'Focusrite USB ASIO', backend: 'ASIO', isDefault: false, sampleRate: 192000, channels: 2, isRunning: false },
    ],
  },
];

/**
 * Device selector for choosing audio output device
 *
 * Features:
 * - Shows current device with sample rate
 * - Dropdown menu with available devices
 * - Grouped by backend (Default, ASIO, JACK)
 * - Spotify-style design
 * - Shows mock devices with "(Desktop only)" in browser demo
 * - Auto-updates when device sample rate changes
 */
export function DeviceSelector() {
  const commands = usePlayerCommands();
  const [currentDevice, setCurrentDevice] = useState<AudioDevice | null>(null);
  const [backends, setBackends] = useState<AudioBackend[]>([]);
  const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map());
  const [isLoading, setIsLoading] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  // Use ref to track isOpen state for event handler (avoids stale closure)
  const isOpenRef = useRef(false);
  isOpenRef.current = isOpen;

  // Check if we're in browser demo mode (no real device commands)
  const isBrowserDemo = !commands?.getCurrentAudioDevice;

  const loadCurrentDevice = useCallback(async () => {
    try {
      if (!commands?.getCurrentAudioDevice) return;
      const device = await commands.getCurrentAudioDevice();
      console.log('[DeviceSelector] Loaded current device:', device?.name, 'at', device?.sampleRate, 'Hz');
      setCurrentDevice(device);
    } catch (error) {
      console.error('[DeviceSelector] Failed to load current device:', error);
    }
  }, [commands]);

  // Memoize loadDevices to avoid recreating it
  const loadDevicesCallback = useCallback(async () => {
    if (isBrowserDemo) {
      const deviceMap = new Map<string, AudioDevice[]>();
      MOCK_DEVICES.forEach(mock => {
        deviceMap.set(mock.backend, mock.devices);
      });
      setDevices(deviceMap);
      return;
    }

    if (isLoading) return;

    setIsLoading(true);
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
              console.error(`[DeviceSelector] Failed to load devices for ${backend.backend}:`, error);
            }
          }
        }

        setDevices(deviceMap);
      }
    } catch (error) {
      console.error('[DeviceSelector] Failed to load devices:', error);
    } finally {
      setIsLoading(false);
    }
  }, [isBrowserDemo, commands, isLoading]);

  // Load current device on mount and listen for sample rate changes
  useEffect(() => {
    if (isBrowserDemo) {
      // Set default mock device
      setCurrentDevice(MOCK_DEVICES[0].devices[0]);
      return;
    }

    loadCurrentDevice();

    // Listen for sample rate changes from the backend
    // This fires when the device sample rate changes externally
    // (e.g., via ASIO control panel or Windows sound settings)
    let unlistenFn: (() => void) | undefined;
    let mounted = true;

    const setupListener = async () => {
      try {
        // Dynamic import to avoid issues in browser demo mode
        const { listen } = await import('@tauri-apps/api/event');

        const unlisten = await listen<{ from: number; to: number }>('playback:sample-rate-changed', (event) => {
          if (!mounted) return;
          console.log('[DeviceSelector] Sample rate changed:', event.payload.from, 'Hz ->', event.payload.to, 'Hz');
          // Refresh current device to get updated sample rate
          loadCurrentDevice();
          // Also refresh device list if dropdown is open (using ref to get current value)
          if (isOpenRef.current) {
            console.log('[DeviceSelector] Dropdown is open, refreshing device list');
            loadDevicesCallback();
          }
        });

        unlistenFn = unlisten;
      } catch (error) {
        // Tauri not available (browser mode), ignore
        console.log('[DeviceSelector] Tauri event listener not available');
      }
    };

    setupListener();

    return () => {
      mounted = false;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [isBrowserDemo, loadCurrentDevice, loadDevicesCallback]);

  const switchDevice = async (backend: string, deviceName: string) => {
    // In browser demo, only allow selecting the System default
    if (isBrowserDemo) {
      if (backend === 'System') {
        setCurrentDevice(MOCK_DEVICES[0].devices[0]);
      }
      return;
    }

    try {
      if (!commands?.setAudioDevice) return;

      await commands.setAudioDevice(backend, deviceName);
      await loadCurrentDevice(); // Refresh current device

      // Explicitly sync playback state after device switch
      // This ensures the play/pause button reflects the actual state
      // Belt-and-suspenders: backend also emits StateChanged event, but we sync explicitly too
      if (commands?.getPlaybackState) {
        const state = await commands.getPlaybackState();
        const isPlaying = state === 'Playing';
        console.log('[DeviceSelector] Syncing playback state after device switch:', state, '-> isPlaying:', isPlaying);
        usePlayerStore.setState({ isPlaying });
      }

      console.log('[DeviceSelector] Switched to:', backend, deviceName);
    } catch (error) {
      console.error('[DeviceSelector] Failed to switch device:', error);
    }
  };

  return (
    <DropdownMenu onOpenChange={(open) => {
      setIsOpen(open);
      if (open) {
        loadDevicesCallback();
      }
    }}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          title={currentDevice ? `${currentDevice.name}${currentDevice.sampleRate ? ` (${currentDevice.sampleRate}Hz)` : ''}` : 'Select audio device'}
        >
          <Speaker className={cn(
            "h-4 w-4",
            currentDevice?.isRunning ? "text-primary" : ""
          )} />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        align="end"
        className="w-[320px] max-h-[400px] overflow-y-auto"
      >
        <DropdownMenuLabel className="flex items-center justify-between">
          <span>Audio Output Device</span>
          {currentDevice?.sampleRate && (
            <span className="text-xs font-normal text-muted-foreground">
              {currentDevice.sampleRate}Hz
            </span>
          )}
        </DropdownMenuLabel>

        <DropdownMenuSeparator />

        {isLoading ? (
          <div className="p-4 text-center text-sm text-muted-foreground min-h-[200px] flex items-center justify-center">
            Loading devices...
          </div>
        ) : isBrowserDemo ? (
          // Browser demo mode - show mock devices
          <>
            {MOCK_DEVICES.map((mockBackend, index) => {
              const isDesktopOnly = mockBackend.backend !== 'System';

              return (
                <div key={mockBackend.backend}>
                  <DropdownMenuLabel className="text-xs uppercase text-muted-foreground">
                    {mockBackend.name}
                  </DropdownMenuLabel>

                  {mockBackend.devices.map((device) => {
                    const isSelected = currentDevice?.name === device.name &&
                                      currentDevice?.backend === device.backend;

                    return (
                      <DropdownMenuItem
                        key={`${device.backend}-${device.name}`}
                        onClick={() => switchDevice(device.backend, device.name)}
                        disabled={isDesktopOnly}
                        className={cn(
                          "flex items-center justify-between",
                          isDesktopOnly ? "cursor-not-allowed" : "cursor-pointer"
                        )}
                      >
                        <div className="flex flex-col min-w-0 flex-1">
                          <span className={cn("text-sm truncate", isDesktopOnly && "text-muted-foreground")}>
                            {device.name}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {device.sampleRate}Hz
                            {device.channels && ` • ${device.channels}ch`}
                          </span>
                        </div>
                        {isSelected && (
                          <Check className="h-4 w-4 text-primary ml-2 flex-shrink-0" />
                        )}
                      </DropdownMenuItem>
                    );
                  })}

                  {index < MOCK_DEVICES.length - 1 && (
                    <DropdownMenuSeparator />
                  )}
                </div>
              );
            })}
          </>
        ) : backends.length === 0 ? (
          <div className="p-4 text-center text-sm text-muted-foreground">
            No audio devices found
          </div>
        ) : (
          // Desktop mode - show real devices
          <>
            {backends.map((backend, index) => {
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

                  {backendDevices.map((device) => {
                    const isSelected = currentDevice?.name === device.name &&
                                      currentDevice?.backend === device.backend;

                    return (
                      <DropdownMenuItem
                        key={`${device.backend}-${device.name}`}
                        onClick={() => switchDevice(device.backend, device.name)}
                        className="flex items-center justify-between cursor-pointer"
                      >
                        <div className="flex flex-col min-w-0 flex-1">
                          <span className="text-sm truncate">{device.name}</span>
                          {device.sampleRate && (
                            <span className="text-xs text-muted-foreground">
                              {device.sampleRate}Hz
                              {device.channels && ` • ${device.channels}ch`}
                            </span>
                          )}
                        </div>
                        {isSelected && (
                          <Check className="h-4 w-4 text-primary ml-2 flex-shrink-0" />
                        )}
                      </DropdownMenuItem>
                    );
                  })}

                  {index < backends.filter(b => b.available).length - 1 && (
                    <DropdownMenuSeparator />
                  )}
                </div>
              );
            })}
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
