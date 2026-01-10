// Volume leveling (ReplayGain / EBU R128) settings component

import { Info } from 'lucide-react';

interface VolumeLevelingSettingsProps {
  mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128';
  onModeChange: (mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128') => void;
}

const modes = [
  {
    value: 'disabled',
    label: 'Disabled',
    description: 'No volume normalization applied',
    targetLevel: null,
  },
  {
    value: 'replaygain_track',
    label: 'ReplayGain (Track)',
    description: 'Normalize each track independently',
    targetLevel: '-18 LUFS',
  },
  {
    value: 'replaygain_album',
    label: 'ReplayGain (Album)',
    description: 'Normalize albums while preserving relative track levels',
    targetLevel: '-18 LUFS',
  },
  {
    value: 'ebu_r128',
    label: 'EBU R128',
    description: 'European Broadcasting Union loudness standard',
    targetLevel: '-23 LUFS',
  },
] as const;

export function VolumeLevelingSettings({
  mode,
  onModeChange,
}: VolumeLevelingSettingsProps) {
  return (
    <div className="bg-card border border-border rounded-lg p-6 space-y-6">
      {/* Mode Selection */}
      <div className="space-y-3">
        <label className="text-sm font-medium">Normalization Mode</label>

        <div className="space-y-2">
          {modes.map((option) => {
            const isSelected = option.value === mode;

            return (
              <button
                key={option.value}
                onClick={() => onModeChange(option.value)}
                className={`
                  w-full text-left p-4 rounded-lg border-2 transition-all
                  ${
                    isSelected
                      ? 'border-primary bg-primary/5 shadow-sm'
                      : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  }
                `}
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-semibold">{option.label}</span>
                      {option.targetLevel && (
                        <span className="text-xs px-2 py-0.5 bg-muted rounded text-muted-foreground">
                          {option.targetLevel}
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-muted-foreground">{option.description}</p>
                  </div>

                  {isSelected && (
                    <div className="flex-shrink-0">
                      <input
                        type="radio"
                        checked={true}
                        onChange={() => {}}
                        className="w-4 h-4 text-primary"
                      />
                    </div>
                  )}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Pre-amp (only when leveling is enabled) */}
      {mode !== 'disabled' && (
        <div className="space-y-3">
          <label className="text-sm font-medium flex items-center gap-2">
            Pre-amp Adjustment
            <Info className="w-3 h-3 text-muted-foreground" title="Additional gain applied after normalization" />
          </label>

          <div className="space-y-2">
            <input
              type="range"
              min="-12"
              max="12"
              step="0.5"
              defaultValue="0"
              className="w-full"
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>-12 dB</span>
              <span className="font-medium text-foreground">0 dB</span>
              <span>+12 dB</span>
            </div>
          </div>

          <p className="text-xs text-muted-foreground">
            Adjust overall volume after normalization. Use negative values if tracks are clipping.
          </p>
        </div>
      )}

      {/* Prevent Clipping */}
      {mode !== 'disabled' && (
        <div className="space-y-3">
          <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-muted/30 transition-colors">
            <input
              type="checkbox"
              defaultChecked={true}
              className="w-4 h-4 mt-0.5"
            />
            <div className="flex-1">
              <div className="text-sm font-medium">Prevent Clipping</div>
              <p className="text-xs text-muted-foreground mt-1">
                Automatically reduce gain if normalized audio would clip. Preserves dynamic range.
              </p>
            </div>
          </label>
        </div>
      )}

      {/* Info Boxes */}
      <div className="space-y-3">
        {/* ReplayGain vs EBU R128 explanation */}
        {mode !== 'disabled' && (
          <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 flex gap-3">
            <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
            <div className="text-sm">
              <p className="font-medium mb-1">
                {mode.startsWith('replaygain') ? 'ReplayGain' : 'EBU R128'}
              </p>
              <p className="text-muted-foreground text-xs">
                {mode.startsWith('replaygain') ? (
                  <>
                    ReplayGain analyzes audio to determine perceived loudness and adjusts playback volume accordingly.
                    <strong> Track mode</strong> normalizes each song independently.
                    <strong> Album mode</strong> maintains relative volume differences within albums.
                  </>
                ) : (
                  <>
                    EBU R128 is a professional loudness standard used in broadcasting. It provides more accurate
                    perceptual loudness measurement than traditional ReplayGain. Target level is -23 LUFS.
                  </>
                )}
              </p>
            </div>
          </div>
        )}

        {/* Tag requirement */}
        {mode !== 'disabled' && (
          <div className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-4 flex gap-3">
            <Info className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5" />
            <div className="text-sm text-foreground">
              <p className="font-medium mb-1">Tag Requirement</p>
              <p className="text-muted-foreground text-xs">
                {mode.startsWith('replaygain') ? (
                  <>
                    ReplayGain requires audio files to have ReplayGain tags. Files without tags will play at original volume.
                    Use a tagging tool like foobar2000, Mp3tag, or Picard to analyze and tag your files.
                  </>
                ) : (
                  <>
                    EBU R128 analyzes tracks in real-time during first playback. Analysis results are cached for future playback.
                    First playback may have slight delay while analyzing.
                  </>
                )}
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
