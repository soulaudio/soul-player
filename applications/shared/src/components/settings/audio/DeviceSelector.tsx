// Audio output device selection component

import { AudioDevice } from '../AudioSettingsPage';
import { Speaker, Check } from 'lucide-react';

interface DeviceSelectorProps {
  devices: AudioDevice[];
  currentDevice: string | null;
  onDeviceChange: (deviceName: string) => void;
  loading?: boolean;
}

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
  devices,
  currentDevice,
  onDeviceChange,
  loading = false,
}: DeviceSelectorProps) {
  if (loading) {
    return (
      <div className="space-y-3">
        <label className="text-sm font-medium">Output Device</label>
        <div className="space-y-2">
          <DeviceSkeleton />
          <DeviceSkeleton />
          <DeviceSkeleton />
        </div>
      </div>
    );
  }
  // If no device selected, use default
  const activeDevice = currentDevice || devices.find(d => d.isDefault)?.name;

  return (
    <div className="space-y-3">
      <label className="text-sm font-medium">Output Device</label>

      {devices.length === 0 ? (
        <div className="p-4 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
          No audio devices found for selected backend
        </div>
      ) : (
        <div className="space-y-2">
          {devices.map((device) => {
            const isSelected = device.name === activeDevice;

            return (
              <button
                key={device.name}
                onClick={() => onDeviceChange(device.name)}
                className={`
                  w-full text-left p-3 rounded-lg border transition-all
                  ${
                    isSelected
                      ? 'border-primary bg-primary/5 shadow-sm'
                      : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  }
                `}
              >
                <div className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    <Speaker className="w-4 h-4 flex-shrink-0 text-muted-foreground" />

                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{device.name}</div>
                      <div className="text-xs text-muted-foreground mt-0.5">
                        {device.sampleRate.toLocaleString()} Hz • {device.channels} channels
                        {device.isDefault && ' • System Default'}
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
