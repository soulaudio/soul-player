/**
 * Volume control - adapted from desktop
 */

import { useState, useRef, useEffect } from 'react'
import { usePlayerStore } from '@soul-player/shared/stores/player'
import { playerCommands } from '@/lib/demo/commands'
import { Volume2, VolumeX } from 'lucide-react'

export function VolumeControl() {
  const { volume } = usePlayerStore()
  const [isMuted, setIsMuted] = useState(false)
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume)
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume)
    }
  }, [volume, isMuted])

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVolume = parseFloat(e.target.value)

    usePlayerStore.getState().setVolume(newVolume)

    if (newVolume > 0 && isMuted) {
      setIsMuted(false)
    }

    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current)
    }

    debounceTimerRef.current = setTimeout(() => {
      playerCommands.setVolume(newVolume)
        .catch((error) => {
          console.error('[VolumeControl] Set volume failed:', error)
        })
    }, 150)
  }

  const handleMuteToggle = async () => {
    try {
      if (isMuted) {
        await playerCommands.setVolume(volumeBeforeMute)
        usePlayerStore.getState().setVolume(volumeBeforeMute)
        setIsMuted(false)
      } else {
        setVolumeBeforeMute(volume)
        await playerCommands.setVolume(0)
        usePlayerStore.getState().setVolume(0)
        setIsMuted(true)
      }
    } catch (error) {
      console.error('[VolumeControl] Mute toggle failed:', error)
    }
  }

  const displayVolume = isMuted ? 0 : volume

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={handleMuteToggle}
        className="p-2 rounded-full hover:bg-accent transition-colors"
        aria-label={isMuted ? 'Unmute' : 'Mute'}
      >
        {isMuted || volume === 0 ? (
          <VolumeX className="w-5 h-5" />
        ) : (
          <Volume2 className="w-5 h-5" />
        )}
      </button>

      <div className="relative w-24 h-2 group">
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={displayVolume}
          onChange={handleVolumeChange}
          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
          aria-label="Volume"
        />

        <div className="absolute inset-0 bg-muted rounded-full" />

        <div
          className="absolute inset-y-0 left-0 bg-primary rounded-full transition-all duration-100"
          style={{ width: `${displayVolume * 100}%` }}
        />

        <div
          className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg opacity-0 group-hover:opacity-100 transition-opacity"
          style={{ left: `${displayVolume * 100}%`, transform: 'translate(-50%, -50%)' }}
        />
      </div>

      <span className="text-xs text-muted-foreground font-mono w-8 text-right">
        {Math.round(displayVolume * 100)}
      </span>
    </div>
  )
}
