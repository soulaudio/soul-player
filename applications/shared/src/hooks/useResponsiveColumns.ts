/**
 * useResponsiveColumns - calculates column count based on scale and viewport width
 * Updates on window resize to match Tailwind responsive breakpoints
 */

import { useState, useEffect } from 'react'

// Tailwind breakpoints
const BREAKPOINTS = {
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
}

/**
 * Column count mapping for each scale at different breakpoints
 * Must match the gridClass responsive classes in each page
 */
const COLUMN_MAP = {
  0.75: {
    base: 2,
    sm: 3,
    md: 5,
    lg: 7,
    xl: 8,
  },
  1: {
    base: 2,
    sm: 3,
    md: 4,
    lg: 5,
    xl: 6,
  },
  1.25: {
    base: 2,
    sm: 2,
    md: 3,
    lg: 4,
    xl: 5,
  },
  1.5: {
    base: 1,
    sm: 2,
    md: 3,
    lg: 3,
    xl: 4,
  },
} as const

type Scale = 0.75 | 1 | 1.25 | 1.5

function getBreakpoint(width: number): 'base' | 'sm' | 'md' | 'lg' | 'xl' {
  if (width >= BREAKPOINTS.xl) return 'xl'
  if (width >= BREAKPOINTS.lg) return 'lg'
  if (width >= BREAKPOINTS.md) return 'md'
  if (width >= BREAKPOINTS.sm) return 'sm'
  return 'base'
}

export function useResponsiveColumns(scale: number): number {
  const [columnCount, setColumnCount] = useState<number>(() => {
    if (typeof window === 'undefined') return 5
    const breakpoint = getBreakpoint(window.innerWidth)
    const scaleKey = (scale in COLUMN_MAP ? scale : 1) as Scale
    return COLUMN_MAP[scaleKey][breakpoint]
  })

  useEffect(() => {
    const calculateColumns = () => {
      const breakpoint = getBreakpoint(window.innerWidth)
      const scaleKey = (scale in COLUMN_MAP ? scale : 1) as Scale
      const newColumnCount = COLUMN_MAP[scaleKey][breakpoint]
      setColumnCount(newColumnCount)
    }

    // Recalculate on resize
    window.addEventListener('resize', calculateColumns)

    // Recalculate when scale changes
    calculateColumns()

    return () => window.removeEventListener('resize', calculateColumns)
  }, [scale])

  return columnCount
}
