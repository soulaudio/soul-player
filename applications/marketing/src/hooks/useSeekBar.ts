/**
 * Demo version of useSeekBar
 * Uses demo commands instead of Tauri
 */

import { useState, useCallback, useRef } from 'react'
import { playerCommands } from '@/lib/demo/commands'
import { usePlayerStore } from '@soul-player/shared/stores/player'

interface UseSeekBarReturn {
  isDragging: boolean
  seekPosition: number | null
  handleSeekStart: (position: number) => void
  handleSeekChange: (position: number) => void
  handleSeekEnd: (finalPosition?: number) => void
}

export function useSeekBar(debounceMs: number = 300): UseSeekBarReturn {
  const [isDragging, setIsDragging] = useState(false)
  const [seekPosition, setSeekPosition] = useState<number | null>(null)
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const handleSeekStart = useCallback((position: number) => {
    setIsDragging(true)
    setSeekPosition(position)
  }, [])

  const handleSeekChange = useCallback((position: number) => {
    setSeekPosition(position)

    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current)
    }

    debounceTimerRef.current = setTimeout(() => {
      // Debounced - only final position sent
    }, debounceMs)
  }, [debounceMs])

  const handleSeekEnd = useCallback((finalPosition?: number) => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current)
      debounceTimerRef.current = null
    }

    const targetPosition = finalPosition ?? seekPosition

    if (targetPosition !== null) {
      const { duration } = usePlayerStore.getState()

      // Update store immediately
      const progressPercentage = duration > 0
        ? Math.min(100, (targetPosition / duration) * 100)
        : 0
      usePlayerStore.getState().setProgress(progressPercentage)

      // Send seek command to demo playback manager
      playerCommands.seekTo(targetPosition)
        .catch((error) => {
          console.error('[useSeekBar] Seek failed:', error)
        })
    }

    setIsDragging(false)
    setSeekPosition(null)
  }, [seekPosition])

  return {
    isDragging,
    seekPosition,
    handleSeekStart,
    handleSeekChange,
    handleSeekEnd,
  }
}
