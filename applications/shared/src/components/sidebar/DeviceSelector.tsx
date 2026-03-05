'use client';

import { Speaker, Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { cn } from '../../lib/utils';

export interface AudioDevice {
  name: string;
  backend: string;
  isDefault: boolean;
  sampleRate?: number;
  channels?: number;
  isRunning?: boolean;
  sampleRateRange?: [number, number];
}

export interface AudioBackend {
  backend: string;
  name: string;
  description: string;
  available: boolean;
  isDefault: boolean;
  deviceCount: number;
}

export interface DeviceSelectorProps {
  currentDevice: AudioDevice | null;
  backends: AudioBackend[];
  devices: Map<string, AudioDevice[]>;
  isLoadingDevices: boolean;
  hasRealDevices: boolean;
  onLoadDevices: () => void;
  onSwitchDevice: (backend: string, deviceName: string) => void;
  variant?: 'dropdown' | 'list';
  showLabel?: boolean;
}

const MOCK_DEVICES: { backend: string; name: string; devices: AudioDevice[] }[] = [
  {
    backend: 'System',
    name: 'System Default',
    devices: [
      {
        name: 'System Default',
        backend: 'System',
        isDefault: true,
        sampleRate: 48000,
        channels: 2,
        isRunning: true,
      },
    ],
  },
];

function DeviceSkeleton() {
  return (
    <div className="w-full p-3 rounded-lg border border-border animate-pulse">
      <div className="flex items-center gap-3">
        <div className="w-4 h-4 bg-muted rounded" />
        <div className="flex-1 space-y-1.5">
          <div className="h-4 bg-muted rounded w-40" />
          <div className="h-3 bg-muted rounded w-32" />
        </div>
      </div>
    </div>
  );
}

export function DeviceSelector({
  currentDevice,
  backends,
  devices,
  isLoadingDevices,
  hasRealDevices,
  onLoadDevices,
  onSwitchDevice,
  variant = 'dropdown',
  showLabel = true,
}: DeviceSelectorProps) {
  const { t } = useTranslation();

  // List variant for settings page
  if (variant === 'list') {
    // Get all devices from the map
    const allDevices: AudioDevice[] = [];
    devices.forEach((deviceList) => {
      allDevices.push(...deviceList);
    });

    // Use mock devices if no real devices
    const displayDevices = !hasRealDevices || allDevices.length === 0
      ? MOCK_DEVICES.flatMap(mock => mock.devices)
      : allDevices;

    // If no device selected, use default
    const activeDevice = currentDevice?.name || displayDevices.find(d => d.isDefault)?.name;

    if (isLoadingDevices) {
      return (
        <div className="space-y-3">
          {showLabel && <label className="text-sm font-medium">{t('audio.outputDevice', 'Output Device')}</label>}
          <div className="space-y-2">
            <DeviceSkeleton />
            <DeviceSkeleton />
            <DeviceSkeleton />
          </div>
        </div>
      );
    }

    return (
      <div className="space-y-3" data-testid="audio-device-section">
        {showLabel && <label className="text-sm font-medium">{t('audio.outputDevice', 'Output Device')}</label>}

        {displayDevices.length === 0 ? (
          <div className="p-4 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
            {t('audio.noDevicesFound', 'No audio devices found')}
          </div>
        ) : (
          <div className="space-y-2" data-testid="audio-device-list">
            {displayDevices.map((device) => {
              const isSelected = device.name === activeDevice && device.backend === currentDevice?.backend;

              return (
                <button
                  key={`${device.backend}-${device.name}`}
                  onClick={() => onSwitchDevice(device.backend, device.name)}
                  data-testid={`audio-device-${device.name.replace(/\s+/g, '-').toLowerCase()}`}
                  className={cn(
                    'w-full text-left p-3 rounded-lg border transition-all',
                    isSelected
                      ? 'border-primary bg-primary/5 shadow-sm'
                      : 'border-border hover:border-primary/50 hover:bg-foreground/[var(--hover-bg-opacity)]'
                  )}
                >
                  <div className="flex items-center justify-between gap-4">
                    <div className="flex items-center gap-3 flex-1 min-w-0">
                      <Speaker className="w-4 h-4 flex-shrink-0 text-muted-foreground" />

                      <div className="flex-1 min-w-0">
                        <div className="font-medium truncate">{device.name}</div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          {device.sampleRate?.toLocaleString()} Hz
                          {device.channels && ` • ${device.channels} channels`}
                          {device.isDefault && ` • ${t('audio.systemDefault', 'System Default')}`}
                        </div>
                      </div>
                    </div>

                    {isSelected && (
                      <div className="flex-shrink-0">
                        <div className="w-5 h-5 rounded-full bg-primary flex items-center justify-center">
                          <Check className="w-3 h-3 text-primary-foreground" />
                        </div>
                      </div>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>
    );
  }

  // Dropdown variant for player panel
  return (
    <DropdownMenu onOpenChange={(open) => { if (open) onLoadDevices(); }}>
      <DropdownMenuTrigger asChild>
        <button
          data-testid="device-selector-button"
          className="p-1 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity ml-1"
          title={currentDevice?.name || t('audio.selectDevice', 'Select audio device')}
        >
          <Speaker className={cn('w-4 h-4', currentDevice?.isRunning && 'text-primary')} />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent data-testid="device-selector-content" align="end" className="w-[280px] max-h-[300px] overflow-y-auto">
        <DropdownMenuLabel className="flex items-center justify-between">
          <span>{t('audio.output', 'Audio Output')}</span>
          {currentDevice?.sampleRate && (
            <span className="text-xs font-normal text-muted-foreground">
              {currentDevice.sampleRate}Hz
            </span>
          )}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {isLoadingDevices ? (
          <div className="p-4 text-center text-sm text-muted-foreground">{t('common.loading', 'Loading...')}</div>
        ) : !hasRealDevices ? (
          MOCK_DEVICES.map((mockBackend, mockIndex) => (
            <div key={`mock-${mockBackend.backend}-${mockIndex}`}>
              <DropdownMenuLabel className="text-xs uppercase text-muted-foreground">
                {mockBackend.name}
              </DropdownMenuLabel>
              {mockBackend.devices.map((device, deviceIndex) => (
                <DropdownMenuItem
                  key={`${mockBackend.backend}-${device.name}-${deviceIndex}`}
                  data-testid={`device-dropdown-item-${device.name.replace(/\s+/g, '-').toLowerCase()}`}
                  onClick={() => onSwitchDevice(device.backend, device.name)}
                  className="flex items-center justify-between cursor-pointer"
                >
                  <div className="flex flex-col min-w-0 flex-1">
                    <span className="text-sm truncate">{device.name}</span>
                    <span className="text-xs text-muted-foreground">{device.sampleRate}Hz</span>
                  </div>
                  {currentDevice?.name === device.name && (
                    <Check className="h-4 w-4 text-primary ml-2" />
                  )}
                </DropdownMenuItem>
              ))}
            </div>
          ))
        ) : backends.length === 0 ? (
          <div className="p-4 text-center text-sm text-muted-foreground">{t('audio.noDevicesFound', 'No audio devices found')}</div>
        ) : (
          backends.map((backend, index) => {
            if (!backend.available) return null;
            const backendDevices = devices.get(backend.backend) || [];
            // Hide only if both the live enumeration AND the startup count report zero devices
            if (backendDevices.length === 0 && backend.deviceCount === 0) return null;
            return (
              <div key={`${backend.backend}-${index}`}>
                {backends.length > 1 && (
                  <DropdownMenuLabel className="text-xs uppercase text-muted-foreground">
                    {backend.name}
                  </DropdownMenuLabel>
                )}
                {backendDevices.length === 0 ? (
                  <DropdownMenuItem disabled>
                    <span className="text-xs text-muted-foreground">{t('audio.noDevicesAvailable', 'No devices available')}</span>
                  </DropdownMenuItem>
                ) : (
                  backendDevices.map((device, deviceIndex) => (
                    <DropdownMenuItem
                      key={`${backend.backend}-${device.name}-${deviceIndex}`}
                      data-testid={`device-dropdown-item-${device.name.replace(/\s+/g, '-').toLowerCase()}`}
                      onClick={() => onSwitchDevice(backend.backend, device.name)}
                      className="flex items-center justify-between cursor-pointer"
                    >
                      <div className="flex flex-col min-w-0 flex-1">
                        <span className="text-sm truncate">{device.name}</span>
                        {device.sampleRate && (
                          <span className="text-xs text-muted-foreground">{device.sampleRate}Hz</span>
                        )}
                      </div>
                      {currentDevice?.name === device.name &&
                        currentDevice?.backend === backend.backend && (
                          <Check className="h-4 w-4 text-primary ml-2" />
                        )}
                    </DropdownMenuItem>
                  ))
                )}
                {index < backends.filter((b) => b.available).length - 1 && <DropdownMenuSeparator />}
              </div>
            );
          })
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
