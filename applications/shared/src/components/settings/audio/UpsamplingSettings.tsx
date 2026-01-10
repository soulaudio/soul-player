// Upsampling/Resampling settings component

import { Info, CheckCircle2 } from 'lucide-react';

interface UpsamplingSettingsProps {
  quality: 'disabled' | 'fast' | 'balanced' | 'high' | 'maximum';
  targetRate: 'auto' | number;
  onQualityChange: (quality: 'disabled' | 'fast' | 'balanced' | 'high' | 'maximum') => void;
  onTargetRateChange: (rate: 'auto' | number) => void;
}

export function UpsamplingSettings({}: UpsamplingSettingsProps) {

  return (
    <div className="bg-card border border-border rounded-lg p-6 space-y-6">
      {/* Status Banner */}
      <div className="bg-green-500/10 border border-green-500/20 rounded-lg p-4 flex gap-3">
        <CheckCircle2 className="w-5 h-5 text-green-500 flex-shrink-0 mt-0.5" />
        <div className="text-sm text-foreground">
          <p className="font-medium mb-1">Automatic High-Quality Resampling</p>
          <p className="text-muted-foreground">
            Resampling is <strong>always active</strong> and automatically matches your output device's sample rate
            to prevent playback speed issues. Uses Sinc interpolation (256 taps, 0.95 cutoff) for transparent quality.
          </p>
        </div>
      </div>

      {/* Current Implementation Details */}
      <div className="space-y-4">
        <div>
          <h3 className="text-sm font-medium mb-2">Current Configuration</h3>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
              <span className="text-muted-foreground">Algorithm</span>
              <span className="font-medium">Sinc (Rubato)</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
              <span className="text-muted-foreground">Filter Length</span>
              <span className="font-medium">256 taps</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
              <span className="text-muted-foreground">Cutoff Frequency</span>
              <span className="font-medium">0.95 × Nyquist</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
              <span className="text-muted-foreground">Target Rate</span>
              <span className="font-medium">Auto (Matches Device)</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
              <span className="text-muted-foreground">CPU Usage</span>
              <span className="font-medium">~10% (44.1 → 96 kHz)</span>
            </div>
          </div>
        </div>
      </div>

      {/* Info Box */}
      <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 flex gap-3">
        <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
        <div className="text-sm text-foreground">
          <p className="font-medium mb-1">How It Works</p>
          <p className="text-muted-foreground mb-2">
            When you play a 44.1 kHz audio file through a 96 kHz device, Soul Player automatically upsamples
            it to match. This prevents the "chipmunk effect" (audio playing too fast) and ensures correct playback speed.
          </p>
          <p className="text-muted-foreground">
            <strong>Example:</strong> Playing a 44.1 kHz MP3 on a 96 kHz DAC → automatically resampled to 96 kHz with
            high-quality Sinc interpolation.
          </p>
        </div>
      </div>

      {/* Future Enhancement Note */}
      <div className="text-xs text-muted-foreground bg-muted/20 p-3 rounded border border-dashed">
        <strong>Coming in Phase 1.5.4:</strong> User-selectable quality presets (Fast/Balanced/High/Maximum)
        and optional r8brain backend for even higher quality resampling.
      </div>
    </div>
  );
}
