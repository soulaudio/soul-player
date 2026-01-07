'use client'

import { ReactNode } from 'react'

interface DemoModeWrapperProps {
  children: ReactNode
  className?: string
  interactive?: boolean // NEW: Allow interactive mode
}

/**
 * Wraps components for marketing showcase
 * Can be interactive (default) or non-interactive for screenshots
 */
export function DemoModeWrapper({ children, className = '', interactive = true }: DemoModeWrapperProps) {
  if (interactive) {
    // Interactive mode - allow full user interaction
    return (
      <div className={`relative ${className}`}>
        <div className="absolute inset-0">
          {children}
        </div>
      </div>
    )
  }

  // Non-interactive mode - prevent all interactions
  return (
    <div className={`relative ${className}`}>
      {/* Render the component - fill the container */}
      <div className="absolute inset-0 pointer-events-none select-none">
        {children}
      </div>

      {/* Invisible overlay to prevent any interaction */}
      <div
        className="absolute inset-0 z-50 cursor-default"
        onClick={(e) => e.preventDefault()}
        onMouseDown={(e) => e.preventDefault()}
      />
    </div>
  )
}
