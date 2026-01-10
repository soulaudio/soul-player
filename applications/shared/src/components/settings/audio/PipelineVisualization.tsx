// Pipeline overview with navigation - click stages to scroll to settings
// Correct audio pipeline order: Source → Decode → Resample → DSP → Leveling → Output

import { ArrowRight, Activity, Volume2, Speaker, Zap, Settings2 } from 'lucide-react';

interface PipelineVisualizationProps {
  backend?: string;
  deviceName?: string | null;
  dspEnabled: boolean;
  dspEffectCount?: number;
  upsamplingEnabled: boolean;
  upsamplingRate?: string;
  volumeLevelingEnabled: boolean;
  volumeLevelingMode?: string;
  loading?: boolean;
}

interface PipelineStageInfo {
  id: string;
  icon: React.ReactNode;
  label: string;
  sublabel?: string;
  active: boolean;
  optional?: boolean;
  navigateTo?: string;
}

function PipelineStageSkeleton() {
  return (
    <div className="flex-1 min-w-0 px-4 py-4 rounded-lg bg-muted/40 animate-pulse">
      <div className="flex items-center gap-3">
        <div className="w-6 h-6 bg-muted rounded" />
        <div className="flex-1 space-y-1.5">
          <div className="h-4 bg-muted rounded w-16" />
          <div className="h-3 bg-muted rounded w-12" />
        </div>
      </div>
    </div>
  );
}

export function PipelineVisualization({
  backend = 'Default',
  deviceName,
  dspEnabled,
  dspEffectCount = 0,
  upsamplingEnabled,
  upsamplingRate = 'Auto',
  volumeLevelingEnabled,
  volumeLevelingMode = 'Disabled',
  loading = false,
}: PipelineVisualizationProps) {
  if (loading) {
    return (
      <div className="w-full">
        <div className="flex items-center w-full gap-2">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="flex items-center flex-1 min-w-0">
              <PipelineStageSkeleton />
              {i < 5 && (
                <div className="flex-shrink-0 mx-2">
                  <ArrowRight className="w-5 h-5 text-muted-foreground/40" />
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    );
  }
  // Correct pipeline order: Resample → DSP → Leveling → Buffer → Output
  const stages: PipelineStageInfo[] = [
    {
      id: 'resample',
      icon: <Activity className="w-6 h-6" />,
      label: 'Resample',
      sublabel: upsamplingRate,
      active: upsamplingEnabled,
      navigateTo: 'audio-stage-1',
    },
    {
      id: 'dsp',
      icon: <Zap className="w-6 h-6" />,
      label: 'DSP',
      sublabel: dspEnabled ? `${dspEffectCount} effects` : 'Off',
      active: dspEnabled,
      optional: true,
      navigateTo: 'audio-stage-2',
    },
    {
      id: 'level',
      icon: <Volume2 className="w-6 h-6" />,
      label: 'Leveling',
      sublabel: volumeLevelingEnabled ? volumeLevelingMode : 'Off',
      active: volumeLevelingEnabled,
      optional: true,
      navigateTo: 'audio-stage-3',
    },
    {
      id: 'buffer',
      icon: <Settings2 className="w-6 h-6" />,
      label: 'Buffer',
      sublabel: 'Performance',
      active: true,
      navigateTo: 'audio-stage-4',
    },
    {
      id: 'output',
      icon: <Speaker className="w-6 h-6" />,
      label: 'Output',
      sublabel: deviceName || backend,
      active: true,
      navigateTo: 'audio-stage-5',
    },
  ];

  const handleStageClick = (navigateTo?: string) => {
    if (!navigateTo) return;
    const element = document.getElementById(navigateTo);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  };

  return (
    <div className="w-full">
      {/* Pipeline Flow - Full width navigation */}
      <div className="flex items-center w-full gap-2">
        {stages.map((stage, index) => (
          <div key={stage.id} className="flex items-center flex-1 min-w-0">
            {/* Stage Box - Clickable */}
            <button
              onClick={() => handleStageClick(stage.navigateTo)}
              className={`
                relative flex items-center gap-3 flex-1 min-w-0
                px-4 py-4 rounded-lg transition-all text-left
                hover:ring-2 hover:ring-primary/50 hover:bg-primary/5
                ${stage.active
                  ? 'bg-primary/10'
                  : 'bg-muted/40'
                }
                ${stage.optional && !stage.active ? 'opacity-50' : ''}
              `}
            >
              {/* Icon */}
              <div className={`flex-shrink-0 ${stage.active ? 'text-primary' : 'text-muted-foreground'}`}>
                {stage.icon}
              </div>

              {/* Label & Sublabel */}
              <div className="min-w-0 flex-1">
                <div
                  className={`text-sm font-semibold uppercase tracking-wide truncate ${
                    stage.active ? 'text-primary' : 'text-muted-foreground'
                  }`}
                >
                  {stage.label}
                </div>
                {stage.sublabel && (
                  <div className="text-xs text-muted-foreground truncate">
                    {stage.sublabel}
                  </div>
                )}
              </div>

              {/* Off indicator for optional stages */}
              {stage.optional && !stage.active && (
                <div className="absolute -top-1 -right-1 bg-muted-foreground/50 text-[9px] px-1.5 py-0.5 rounded text-background font-medium">
                  OFF
                </div>
              )}
            </button>

            {/* Arrow between stages - More visible */}
            {index < stages.length - 1 && (
              <div className="flex-shrink-0 mx-2">
                <ArrowRight
                  className={`w-5 h-5 ${
                    stage.active ? 'text-primary' : 'text-muted-foreground/40'
                  }`}
                />
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
