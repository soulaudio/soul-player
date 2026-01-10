// Visual representation of the audio processing pipeline

import { ArrowRight } from 'lucide-react';

interface PipelineVisualizationProps {
  dspEnabled: boolean;
  upsamplingEnabled: boolean;
  volumeLevelingEnabled: boolean;
}

export function PipelineVisualization({
  dspEnabled,
  upsamplingEnabled,
  volumeLevelingEnabled,
}: PipelineVisualizationProps) {
  const stages = [
    { id: 'file', label: 'FILE', active: true, description: 'Audio file loading' },
    { id: 'decode', label: 'DECODE', active: true, description: 'Format decoding' },
    { id: 'dsp', label: 'DSP', active: dspEnabled, description: 'Effects processing', optional: true },
    { id: 'upsample', label: 'UPSAMPLE', active: upsamplingEnabled, description: 'Sample rate conversion', optional: true },
    { id: 'level', label: 'LEVEL', active: volumeLevelingEnabled, description: 'Volume normalization', optional: true },
    { id: 'volume', label: 'VOLUME', active: true, description: 'Volume control' },
    { id: 'output', label: 'OUTPUT', active: true, description: 'Audio device' },
  ];

  return (
    <div className="bg-gradient-to-r from-primary/5 to-primary/10 border border-primary/20 rounded-lg p-6">
      <h3 className="text-sm font-semibold text-primary mb-4 uppercase tracking-wide">
        Audio Processing Pipeline
      </h3>

      <div className="flex items-center justify-between gap-2 overflow-x-auto pb-2">
        {stages.map((stage, index) => (
          <div key={stage.id} className="flex items-center gap-2 flex-shrink-0">
            {/* Stage Box */}
            <div
              className={`
                relative px-4 py-3 rounded-lg border-2 transition-all duration-200
                min-w-[100px] text-center group
                ${
                  stage.active
                    ? 'border-primary bg-primary/10 shadow-sm'
                    : 'border-dashed border-muted-foreground/30 bg-muted/30'
                }
                ${stage.optional && !stage.active ? 'opacity-50' : ''}
              `}
              title={stage.description}
            >
              {/* Optional badge */}
              {stage.optional && !stage.active && (
                <div className="absolute -top-2 -right-2 bg-muted-foreground/20 text-[10px] px-1.5 py-0.5 rounded-full border border-muted-foreground/30">
                  OFF
                </div>
              )}

              <div
                className={`
                  text-xs font-bold tracking-wider
                  ${stage.active ? 'text-primary' : 'text-muted-foreground'}
                `}
              >
                {stage.label}
              </div>

              {/* Tooltip on hover */}
              <div className="hidden group-hover:block absolute -bottom-12 left-1/2 -translate-x-1/2 z-10 bg-popover border border-border rounded px-2 py-1 text-xs whitespace-nowrap shadow-lg">
                {stage.description}
                <div className="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-popover border-l border-t border-border rotate-45" />
              </div>
            </div>

            {/* Arrow between stages */}
            {index < stages.length - 1 && (
              <ArrowRight
                className={`
                  w-4 h-4 flex-shrink-0
                  ${stage.active ? 'text-primary' : 'text-muted-foreground/30'}
                `}
              />
            )}
          </div>
        ))}
      </div>

      {/* Info text */}
      <p className="text-xs text-muted-foreground mt-4 text-center">
        Audio flows through {stages.filter(s => s.active).length} active stages •
        {dspEnabled ? ' DSP enabled •' : ''}
        {upsamplingEnabled ? ' Upsampling enabled •' : ''}
        {volumeLevelingEnabled ? ' Volume leveling enabled •' : ''}
        {' '}Bit-perfect at 100% volume
      </p>
    </div>
  );
}
