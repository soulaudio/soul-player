'use client';

import { Volume2, VolumeX } from 'lucide-react';

export interface VolumeControlProps {
  volume: number;
  isMuted: boolean;
  onVolumeChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onMuteToggle: () => void;
  onWheel?: (e: React.WheelEvent) => void;
}

export function VolumeControl({
  volume,
  isMuted,
  onVolumeChange,
  onMuteToggle,
  onWheel,
}: VolumeControlProps) {
  const displayVolume = isMuted ? 0 : volume;

  return (
    <div className="flex items-center gap-2" onWheel={onWheel}>
      <button
        onClick={onMuteToggle}
        className="p-1 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity"
      >
        {isMuted || volume === 0 ? (
          <VolumeX className="w-4 h-4" />
        ) : (
          <Volume2 className="w-4 h-4" />
        )}
      </button>
      <div className="flex-1 relative h-4 flex items-center cursor-pointer group">
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={displayVolume}
          onChange={onVolumeChange}
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
    </div>
  );
}
