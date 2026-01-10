// Pipeline stage wrapper component with consistent layout
// Clean, minimal design with clear visual hierarchy

import { ReactNode } from 'react';

export interface PipelineStageProps {
  /** HTML id for navigation */
  id?: string;
  /** Stage title */
  title: string;
  /** Stage description shown below title */
  description: string;
  /** Whether this stage is active/enabled */
  isActive?: boolean;
  /** Optional status indicator text */
  statusText?: string;
  /** Optional current value/config summary */
  currentConfig?: string;
  /** Whether this is the last stage */
  isLast?: boolean;
  /** Whether this stage is optional */
  isOptional?: boolean;
  /** Loading state */
  isLoading?: boolean;
  /** Children content (the settings controls) */
  children: ReactNode;
}

export function PipelineStage({
  id,
  title,
  description,
  isActive = true,
  statusText,
  currentConfig,
  isLast = false,
  isOptional = false,
  children,
}: PipelineStageProps) {
  return (
    <div id={id} className={`scroll-mt-4 ${!isLast ? 'pb-8 mb-8 border-b border-border/40' : ''}`}>
      {/* Header */}
      <div className="flex items-start justify-between gap-4 mb-5">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-3 mb-1">
            {/* Active indicator dot */}
            <div
              className={`w-2 h-2 rounded-full flex-shrink-0 ${
                isActive ? 'bg-primary' : 'bg-muted-foreground/30'
              }`}
            />
            <h3 className={`text-lg font-semibold ${!isActive && 'text-muted-foreground'}`}>
              {title}
            </h3>
            {isOptional && (
              <span className="text-[10px] px-2 py-0.5 rounded-full bg-muted text-muted-foreground uppercase tracking-wide">
                Optional
              </span>
            )}
            {statusText && (
              <span
                className={`text-[10px] px-2 py-0.5 rounded-full uppercase tracking-wide ${
                  isActive
                    ? 'bg-primary/10 text-primary'
                    : 'bg-muted text-muted-foreground'
                }`}
              >
                {statusText}
              </span>
            )}
          </div>
          <p className="text-sm text-muted-foreground ml-5">{description}</p>
        </div>

        {/* Current config */}
        {currentConfig && (
          <div className="text-right flex-shrink-0">
            <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Current</div>
            <div className="text-sm font-medium truncate max-w-[150px]">{currentConfig}</div>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="ml-5">
        {children}
      </div>
    </div>
  );
}
