/**
 * useGridScale - hook for managing grid scale state with persistence
 * Supports Ctrl+/Ctrl- keyboard shortcuts for scaling
 */

import { useState, useCallback, useEffect } from 'react'

const SCALE_STEPS = [0.75, 1, 1.25, 1.5]
const DEFAULT_SCALE = 1
const STORAGE_KEY = 'grid-scale'

export function useGridScale() {
  const [scale, setScale] = useState<number>(() => {
    if (typeof window === 'undefined') return DEFAULT_SCALE
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const parsed = parseFloat(stored)
      if (SCALE_STEPS.includes(parsed)) {
        return parsed
      }
    }
    return DEFAULT_SCALE
  })

  // Persist scale to localStorage
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(scale))
  }, [scale])

  // Keyboard shortcuts: Ctrl+ and Ctrl-
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check for Ctrl/Cmd key
      const isModifier = e.ctrlKey || e.metaKey
      if (!isModifier) return

      // Ctrl/Cmd + = or Ctrl/Cmd + + (scale up)
      if (e.key === '=' || e.key === '+') {
        e.preventDefault()
        setScale(current => {
          const currentIndex = SCALE_STEPS.indexOf(current)
          if (currentIndex === -1) return DEFAULT_SCALE
          const nextIndex = Math.min(currentIndex + 1, SCALE_STEPS.length - 1)
          return SCALE_STEPS[nextIndex]
        })
      }
      // Ctrl/Cmd + - (scale down)
      else if (e.key === '-') {
        e.preventDefault()
        setScale(current => {
          const currentIndex = SCALE_STEPS.indexOf(current)
          if (currentIndex === -1) return DEFAULT_SCALE
          const prevIndex = Math.max(currentIndex - 1, 0)
          return SCALE_STEPS[prevIndex]
        })
      }
      // Ctrl/Cmd + 0 (reset to default)
      else if (e.key === '0') {
        e.preventDefault()
        setScale(DEFAULT_SCALE)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  const scaleUp = useCallback(() => {
    setScale(current => {
      const currentIndex = SCALE_STEPS.indexOf(current)
      if (currentIndex === -1) return DEFAULT_SCALE
      // Scale up means larger items = fewer columns = higher scale value
      const nextIndex = Math.min(currentIndex + 1, SCALE_STEPS.length - 1)
      return SCALE_STEPS[nextIndex]
    })
  }, [])

  const scaleDown = useCallback(() => {
    setScale(current => {
      const currentIndex = SCALE_STEPS.indexOf(current)
      if (currentIndex === -1) return DEFAULT_SCALE
      // Scale down means smaller items = more columns = lower scale value
      const prevIndex = Math.max(currentIndex - 1, 0)
      return SCALE_STEPS[prevIndex]
    })
  }, [])

  const resetScale = useCallback(() => {
    setScale(DEFAULT_SCALE)
  }, [])

  return {
    scale,
    scaleUp,
    scaleDown,
    resetScale,
    canScaleUp: scale < SCALE_STEPS[SCALE_STEPS.length - 1],
    canScaleDown: scale > SCALE_STEPS[0],
  }
}
