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
  isRunning: boolean;
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

export function DeviceSelector({
  currentDevice,
  backends,
  devices,
  isLoadingDevices,
  hasRealDevices,
  onLoadDevices,
  onSwitchDevice,
}: DeviceSelectorProps) {
  const { t } = useTranslation();

  return (
    <DropdownMenu onOpenChange={(open) => { if (open) onLoadDevices(); }}>
      <DropdownMenuTrigger asChild>
        <button
          className="p-1 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity ml-1"
          title={currentDevice?.name || t('audio.selectDevice', 'Select audio device')}
        >
          <Speaker className={cn('w-4 h-4', currentDevice?.isRunning && 'text-primary')} />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-[280px] max-h-[300px] overflow-y-auto">
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
          MOCK_DEVICES.map((mockBackend) => (
            <div key={mockBackend.backend}>
              <DropdownMenuLabel className="text-xs uppercase text-muted-foreground">
                {mockBackend.name}
              </DropdownMenuLabel>
              {mockBackend.devices.map((device) => (
                <DropdownMenuItem
                  key={`${device.backend}-${device.name}`}
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
                    onClick={() => onSwitchDevice(device.backend, device.name)}
                    className="flex items-center justify-between cursor-pointer"
                  >
                    <div className="flex flex-col min-w-0 flex-1">
                      <span className="text-sm truncate">{device.name}</span>
                      {device.sampleRate && (
                        <span className="text-xs text-muted-foreground">{device.sampleRate}Hz</span>
                      )}
                    </div>
                    {currentDevice?.name === device.name &&
                      currentDevice?.backend === device.backend && (
                        <Check className="h-4 w-4 text-primary ml-2" />
                      )}
                  </DropdownMenuItem>
                ))}
                {index < backends.filter((b) => b.available).length - 1 && <DropdownMenuSeparator />}
              </div>
            );
          })
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
