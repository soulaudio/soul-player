'use client'

import { useNavigate } from 'react-router-dom'
import { usePlayerStore } from '@soul-player/shared/stores/player'
import { usePlayerCommands } from '@soul-player/shared'
import { Play, Pause, SkipBack, SkipForward, Music, ChevronLeft, Shuffle, Repeat, Repeat1 } from 'lucide-react'

export function NowPlayingPage() {
  const navigate = useNavigate()
  const { currentTrack, isPlaying, queue, progress, duration, shuffleEnabled, repeatMode } = usePlayerStore()
  const { resumePlayback, pausePlayback, skipNext, skipPrevious, seek, setShuffle, setRepeatMode, skipToQueueIndex } = usePlayerCommands()

  const handlePlayPause = async () => {
    if (isPlaying) {
      await pausePlayback()
    } else {
      await resumePlayback()
    }
  }

  const handleSeek = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const x = e.clientX - rect.left
    const percentage = (x / rect.width) * 100
    const newPosition = (percentage / 100) * duration
    seek(newPosition)
  }

  const handleRepeatToggle = async () => {
    const modes: Array<'off' | 'all' | 'one'> = ['off', 'all', 'one']
    const currentIndex = modes.indexOf(repeatMode)
    const nextMode = modes[(currentIndex + 1) % modes.length]
    await setRepeatMode(nextMode)
  }

  const handleShuffleToggle = async () => {
    await setShuffle(!shuffleEnabled)
  }

  const handleTrackClick = async (index: number) => {
    await skipToQueueIndex(index)
  }

  const formatTime = (seconds: number | undefined) => {
    if (seconds === undefined || isNaN(seconds)) return '--:--'
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  // If nothing is playing, redirect to home
  if (!currentTrack) {
    return (
      <div className="h-full flex flex-col items-center justify-center">
        <div className="w-24 h-24 rounded-full bg-muted flex items-center justify-center mb-6">
          <Music className="w-12 h-12 text-muted-foreground" />
        </div>
        <h2 className="text-xl font-medium text-muted-foreground mb-2">Nothing playing</h2>
        <p className="text-sm text-muted-foreground mb-6">Select a track from your library to start listening</p>
        <button
          onClick={() => navigate('/library')}
          className="px-6 py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          Browse Library
        </button>
      </div>
    )
  }

  const currentPosition = (progress / 100) * duration

  return (
    <div className="h-full flex">
      {/* Left Side - Artwork and Controls */}
      <div className="w-1/2 flex flex-col p-8 bg-gradient-to-br from-background to-muted/30">
        {/* Back Button */}
        <button
          onClick={() => navigate(-1)}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors mb-6 w-fit"
        >
          <ChevronLeft className="w-5 h-5" />
          <span className="text-sm font-medium">Back</span>
        </button>

        {/* Artwork */}
        <div className="flex-1 flex items-center justify-center">
          <div className="w-full max-w-md aspect-square rounded-2xl overflow-hidden shadow-2xl bg-muted flex items-center justify-center">
            {currentTrack.coverArtPath ? (
              <img
                src={currentTrack.coverArtPath}
                alt={currentTrack.album || currentTrack.title}
                className="w-full h-full object-cover"
              />
            ) : (
              <Music className="w-24 h-24 text-muted-foreground" />
            )}
          </div>
        </div>

        {/* Track Info */}
        <div className="mt-8 text-center">
          <h1 className="text-2xl font-bold truncate">{currentTrack.title}</h1>
          <p className="text-lg text-muted-foreground truncate mt-1">{currentTrack.artist}</p>
          {currentTrack.album && (
            <p className="text-sm text-muted-foreground truncate mt-1">{currentTrack.album}</p>
          )}
        </div>

        {/* Progress Bar */}
        <div className="mt-6">
          <div
            className="h-1.5 bg-muted rounded-full cursor-pointer group"
            onClick={handleSeek}
          >
            <div
              className="h-full bg-primary rounded-full relative group-hover:bg-primary/80 transition-colors"
              style={{ width: `${progress}%` }}
            >
              <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
            </div>
          </div>
          <div className="flex justify-between text-xs text-muted-foreground mt-2">
            <span>{formatTime(currentPosition)}</span>
            <span>{formatTime(duration)}</span>
          </div>
        </div>

        {/* Playback Controls */}
        <div className="flex items-center justify-center gap-6 mt-6">
          <button
            onClick={handleShuffleToggle}
            className={`p-2 rounded-full transition-colors ${
              shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'
            }`}
            aria-label="Toggle shuffle"
          >
            <Shuffle className="w-5 h-5" />
          </button>

          <button
            onClick={skipPrevious}
            className="p-3 rounded-full hover:bg-accent transition-colors"
            aria-label="Previous track"
          >
            <SkipBack className="w-6 h-6" />
          </button>

          <button
            onClick={handlePlayPause}
            className="p-5 rounded-full bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? <Pause className="w-8 h-8" /> : <Play className="w-8 h-8 ml-1" />}
          </button>

          <button
            onClick={skipNext}
            className="p-3 rounded-full hover:bg-accent transition-colors"
            aria-label="Next track"
          >
            <SkipForward className="w-6 h-6" />
          </button>

          <button
            onClick={handleRepeatToggle}
            className={`p-2 rounded-full transition-colors ${
              repeatMode !== 'off' ? 'text-primary' : 'text-muted-foreground hover:text-foreground'
            }`}
            aria-label="Toggle repeat"
          >
            {repeatMode === 'one' ? <Repeat1 className="w-5 h-5" /> : <Repeat className="w-5 h-5" />}
          </button>
        </div>
      </div>

      {/* Right Side - Queue/Tracklist */}
      <div className="w-1/2 flex flex-col border-l bg-card">
        <div className="p-6 border-b">
          <h2 className="text-lg font-bold">Up Next</h2>
          <p className="text-sm text-muted-foreground mt-1">
            {queue.length} track{queue.length !== 1 ? 's' : ''} in queue
          </p>
        </div>

        <div className="flex-1 overflow-auto">
          {queue.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
              <Music className="w-12 h-12 mb-4 opacity-50" />
              <p>Queue is empty</p>
            </div>
          ) : (
            <div className="divide-y">
              {queue.map((track, index) => {
                const isCurrentTrack = currentTrack && track.id === currentTrack.id
                return (
                  <button
                    key={`${track.id}-${index}`}
                    onClick={() => handleTrackClick(index)}
                    className={`w-full flex items-center gap-4 p-4 hover:bg-accent/50 transition-colors text-left ${
                      isCurrentTrack ? 'bg-primary/10' : ''
                    }`}
                  >
                    {/* Track Number or Playing Indicator */}
                    <div className="w-8 text-center flex-shrink-0">
                      {isCurrentTrack && isPlaying ? (
                        <div className="flex items-center justify-center gap-0.5">
                          <span className="w-1 h-3 bg-primary rounded-full animate-pulse" />
                          <span className="w-1 h-4 bg-primary rounded-full animate-pulse" style={{ animationDelay: '0.2s' }} />
                          <span className="w-1 h-2 bg-primary rounded-full animate-pulse" style={{ animationDelay: '0.4s' }} />
                        </div>
                      ) : (
                        <span className={`text-sm ${isCurrentTrack ? 'text-primary font-medium' : 'text-muted-foreground'}`}>
                          {index + 1}
                        </span>
                      )}
                    </div>

                    {/* Artwork */}
                    <div className="w-12 h-12 rounded overflow-hidden bg-muted flex-shrink-0 flex items-center justify-center">
                      {track.coverArtPath ? (
                        <img
                          src={track.coverArtPath}
                          alt={track.album || track.title}
                          className="w-full h-full object-cover"
                        />
                      ) : (
                        <Music className="w-6 h-6 text-muted-foreground" />
                      )}
                    </div>

                    {/* Track Info */}
                    <div className="flex-1 min-w-0">
                      <p className={`font-medium truncate ${isCurrentTrack ? 'text-primary' : ''}`}>
                        {track.title}
                      </p>
                      <p className="text-sm text-muted-foreground truncate">
                        {track.artist}
                      </p>
                    </div>

                    {/* Duration */}
                    <span className="text-sm text-muted-foreground flex-shrink-0">
                      {track.duration ? formatTime(track.duration) : '--:--'}
                    </span>
                  </button>
                )
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
