// Audio backend selection component

import { AudioBackend } from '../AudioSettingsPage';
import { Check, Info } from 'lucide-react';

interface BackendSelectorProps {
  backends: AudioBackend[];
  currentBackend: 'default' | 'asio' | 'jack';
  onBackendChange: (backend: 'default' | 'asio' | 'jack') => void;
  loading?: boolean;
}

function BackendSkeleton() {
  return (
    <div className="w-full p-4 rounded-lg border-2 border-border animate-pulse">
      <div className="flex items-start justify-between gap-4">
        <div className="flex-1 space-y-2">
          <div className="h-5 bg-muted rounded w-32" />
          <div className="h-4 bg-muted rounded w-48" />
          <div className="h-3 bg-muted rounded w-24 mt-2" />
        </div>
      </div>
    </div>
  );
}

export function BackendSelector({
  backends,
  currentBackend,
  onBackendChange,
  loading = false,
}: BackendSelectorProps) {
  if (loading) {
    return (
      <div className="space-y-3">
        <label className="text-sm font-medium">Audio Backend</label>
        <div className="space-y-2">
          <BackendSkeleton />
          <BackendSkeleton />
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <label className="text-sm font-medium">Audio Backend</label>

      <div className="space-y-2">
        {backends.map((backend) => {
          const isSelected = backend.backend === currentBackend;
          const isAvailable = backend.available;

          return (
            <button
              key={backend.backend}
              onClick={() => isAvailable && onBackendChange(backend.backend)}
              disabled={!isAvailable}
              className={`
                w-full text-left p-4 rounded-lg border-2 transition-all
                ${
                  isSelected
                    ? 'border-primary bg-primary/5 shadow-sm'
                    : 'border-border hover:border-primary/50 hover:bg-muted/30'
                }
                ${!isAvailable ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
              `}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-semibold">{backend.name}</span>
                    {backend.is_default && (
                      <span className="text-xs px-2 py-0.5 bg-muted rounded-full text-muted-foreground">
                        System Default
                      </span>
                    )}
                    {!isAvailable && (
                      <span className="text-xs px-2 py-0.5 bg-destructive/10 text-destructive rounded-full">
                        Unavailable
                      </span>
                    )}
                  </div>

                  <p className="text-sm text-muted-foreground">
                    {backend.description}
                  </p>

                  {isAvailable && (
                    <p className="text-xs text-muted-foreground mt-2">
                      {backend.device_count} {backend.device_count === 1 ? 'device' : 'devices'} available
                    </p>
                  )}

                  {!isAvailable && backend.backend === 'asio' && (
                    <div className="mt-2 flex items-start gap-2 text-xs text-muted-foreground bg-muted/50 p-2 rounded">
                      <Info className="w-4 h-4 flex-shrink-0 mt-0.5" />
                      <div>
                        <p className="font-medium mb-1">ASIO Setup Required:</p>
                        <ul className="list-disc list-inside space-y-0.5 text-xs">
                          <li>Install ASIO-compatible audio drivers</li>
                          <li>Or install ASIO4ALL (free universal driver)</li>
                        </ul>
                      </div>
                    </div>
                  )}
                </div>

                {isSelected && (
                  <div className="flex-shrink-0">
                    <div className="w-6 h-6 rounded-full bg-primary flex items-center justify-center">
                      <Check className="w-4 h-4 text-primary-foreground" />
                    </div>
                  </div>
                )}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
