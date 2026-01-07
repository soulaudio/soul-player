/**
 * Playback controls component with progress bar
 * Positioned at bottom of demo player
 */

'use client'

import {
  Play,
  Pause,
  SkipForward,
  SkipBack,
  Volume2,
  VolumeX,
  Shuffle,
  Repeat,
  Repeat1
} from 'lucide-react'
import { PlaybackState, RepeatMode, ShuffleMode } from '@/lib/demo/types'

interface PlaybackControlsProps {
  // State
  state: PlaybackState
  currentTrack: { title: string; artist: string; album?: string } | null
  position: number
  duration: number
  volume: number
  isMuted: boolean
  shuffle: ShuffleMode
  repeat: RepeatMode

  // Callbacks
  onPlay: () => void
  onPause: () => void
  onNext: () => void
  onPrevious: () => void
  onSeek: (position: number) => void
  onVolumeChange: (volume: number) => void
  onToggleMute: () => void
  onShuffleToggle: () => void
  onRepeatCycle: () => void
}

function formatTime(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

export function PlaybackControls({
  state,
  currentTrack,
  position,
  duration,
  volume,
  isMuted,
  shuffle,
  repeat,
  onPlay,
  onPause,
  onNext,
  onPrevious,
  onSeek,
  onVolumeChange,
  onToggleMute,
  onShuffleToggle,
  onRepeatCycle
}: PlaybackControlsProps) {
  const isPlaying = state === PlaybackState.Playing
  const progress = duration > 0 ? (position / duration) * 100 : 0

  const handleProgressClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const x = e.clientX - rect.left
    const percent = x / rect.width
    const newPosition = percent * duration
    onSeek(newPosition)
  }

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onVolumeChange(Number(e.target.value))
  }

  return (
    <div className="border-t bg-background p-4 space-y-3">
      {/* Track info */}
      <div className="flex items-center justify-between min-h-[40px]">
        <div className="flex-1 min-w-0">
          {currentTrack ? (
            <div>
              <p className="text-sm font-medium truncate">{currentTrack.title}</p>
              <p className="text-xs text-muted-foreground truncate">
                {currentTrack.artist}
                {currentTrack.album && ` • ${currentTrack.album}`}
              </p>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No track playing</p>
          )}
        </div>

        {/* Time display */}
        {currentTrack && (
          <div className="text-xs text-muted-foreground ml-4 tabular-nums">
            {formatTime(position)} / {formatTime(duration)}
          </div>
        )}
      </div>

      {/* Progress bar */}
      <div className="space-y-1">
        <div
          className="h-1.5 bg-muted rounded-full cursor-pointer group relative"
          onClick={handleProgressClick}
        >
          <div
            className="h-full bg-primary rounded-full transition-all"
            style={{ width: `${progress}%` }}
          />
          <div
            className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
            style={{ left: `${progress}%`, transform: 'translate(-50%, -50%)' }}
          />
        </div>
      </div>

      {/* Controls */}
      <div className="flex items-center justify-between">
        {/* Left: Shuffle & Repeat */}
        <div className="flex items-center gap-1">
          <button
            onClick={onShuffleToggle}
            className={`p-2 rounded-md transition-colors ${
              shuffle !== ShuffleMode.Off
                ? 'bg-primary/10 text-primary hover:bg-primary/20'
                : 'hover:bg-muted'
            }`}
            title={shuffle === ShuffleMode.Off ? 'Enable shuffle' : 'Disable shuffle'}
            aria-label="Toggle shuffle"
          >
            <Shuffle className="w-4 h-4" />
          </button>

          <button
            onClick={onRepeatCycle}
            className={`p-2 rounded-md transition-colors ${
              repeat !== RepeatMode.Off
                ? 'bg-primary/10 text-primary hover:bg-primary/20'
                : 'hover:bg-muted'
            }`}
            title={
              repeat === RepeatMode.Off
                ? 'Enable repeat'
                : repeat === RepeatMode.All
                  ? 'Repeat one'
                  : 'Disable repeat'
            }
            aria-label="Toggle repeat"
          >
            {repeat === RepeatMode.One ? (
              <Repeat1 className="w-4 h-4" />
            ) : (
              <Repeat className="w-4 h-4" />
            )}
          </button>
        </div>

        {/* Center: Main controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={onPrevious}
            className="p-2 rounded-md hover:bg-muted transition-colors"
            title="Previous track"
            aria-label="Previous track"
          >
            <SkipBack className="w-5 h-5" />
          </button>

          <button
            onClick={isPlaying ? onPause : onPlay}
            className="p-3 rounded-full bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
            title={isPlaying ? 'Pause' : 'Play'}
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5 ml-0.5" />}
          </button>

          <button
            onClick={onNext}
            className="p-2 rounded-md hover:bg-muted transition-colors"
            title="Next track"
            aria-label="Next track"
          >
            <SkipForward className="w-5 h-5" />
          </button>
        </div>

        {/* Right: Volume */}
        <div className="flex items-center gap-2 min-w-[120px]">
          <button
            onClick={onToggleMute}
            className="p-2 rounded-md hover:bg-muted transition-colors"
            title={isMuted ? 'Unmute' : 'Mute'}
            aria-label={isMuted ? 'Unmute' : 'Mute'}
          >
            {isMuted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
          </button>

          <input
            type="range"
            min="0"
            max="100"
            value={isMuted ? 0 : volume}
            onChange={handleVolumeChange}
            className="flex-1 h-1.5 bg-muted rounded-full appearance-none cursor-pointer
                     [&::-webkit-slider-thumb]:appearance-none
                     [&::-webkit-slider-thumb]:w-3
                     [&::-webkit-slider-thumb]:h-3
                     [&::-webkit-slider-thumb]:rounded-full
                     [&::-webkit-slider-thumb]:bg-primary
                     [&::-webkit-slider-thumb]:cursor-pointer
                     [&::-moz-range-thumb]:w-3
                     [&::-moz-range-thumb]:h-3
                     [&::-moz-range-thumb]:rounded-full
                     [&::-moz-range-thumb]:bg-primary
                     [&::-moz-range-thumb]:border-0
                     [&::-moz-range-thumb]:cursor-pointer"
            title="Volume"
            aria-label="Volume"
          />
        </div>
      </div>
    </div>
  )
}
