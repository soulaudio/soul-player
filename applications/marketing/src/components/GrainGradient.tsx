'use client'

import { CSSProperties } from 'react'

interface GrainGradientProps {
  from?: string
  via?: string
  to?: string
  className?: string
  children?: React.ReactNode
}

export function GrainGradient({
  from = '#7c3aed',
  via = '#a78bfa',
  to = '#c4b5fd',
  className = '',
  children
}: GrainGradientProps) {
  const gradientStyle: CSSProperties = {
    background: `linear-gradient(135deg, ${from} 0%, ${via} 50%, ${to} 100%)`,
    backgroundSize: '100% 100%',
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
  }

  return (
    <div
      className={`relative overflow-hidden ${className}`}
      style={gradientStyle}
    >
      <div className="grain absolute inset-0 pointer-events-none" />
      <div className="relative z-10">
        {children}
      </div>
    </div>
  )
}
