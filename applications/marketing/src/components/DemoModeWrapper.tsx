'use client'

import { ReactNode } from 'react'

interface DemoModeWrapperProps {
  children: ReactNode
  className?: string
}

/**
 * Wraps components in a non-interactive demo mode for marketing showcase
 * Prevents all user interactions while maintaining visual appearance
 */
export function DemoModeWrapper({ children, className = '' }: DemoModeWrapperProps) {
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
