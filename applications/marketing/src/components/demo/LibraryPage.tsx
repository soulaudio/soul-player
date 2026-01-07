'use client'

import { useState } from 'react'

type ViewMode = 'tracks' | 'albums'

interface Track {
  id: number
  title: string
  artist: string
  album: string
  duration: number
}

interface Album {
  id: number
  title: string
  artist: string
  year: number
  trackCount: number
}

// Mock data
const MOCK_TRACKS: Track[] = [
  { id: 1, title: 'Bohemian Rhapsody', artist: 'Queen', album: 'A Night at the Opera', duration: 354 },
  { id: 2, title: 'Stairway to Heaven', artist: 'Led Zeppelin', album: 'Led Zeppelin IV', duration: 482 },
  { id: 3, title: 'Hotel California', artist: 'Eagles', album: 'Hotel California', duration: 391 },
  { id: 4, title: 'Imagine', artist: 'John Lennon', album: 'Imagine', duration: 183 },
  { id: 5, title: 'Smells Like Teen Spirit', artist: 'Nirvana', album: 'Nevermind', duration: 301 },
  { id: 6, title: 'Billie Jean', artist: 'Michael Jackson', album: 'Thriller', duration: 294 },
  { id: 7, title: 'Sweet Child O\' Mine', artist: 'Guns N\' Roses', album: 'Appetite for Destruction', duration: 356 },
  { id: 8, title: 'November Rain', artist: 'Guns N\' Roses', album: 'Use Your Illusion I', duration: 537 },
]

const MOCK_ALBUMS: Album[] = [
  { id: 1, title: 'A Night at the Opera', artist: 'Queen', year: 1975, trackCount: 12 },
  { id: 2, title: 'Led Zeppelin IV', artist: 'Led Zeppelin', year: 1971, trackCount: 8 },
  { id: 3, title: 'Hotel California', artist: 'Eagles', year: 1976, trackCount: 9 },
  { id: 4, title: 'Imagine', artist: 'John Lennon', year: 1971, trackCount: 10 },
  { id: 5, title: 'Nevermind', artist: 'Nirvana', year: 1991, trackCount: 13 },
  { id: 6, title: 'Thriller', artist: 'Michael Jackson', year: 1982, trackCount: 9 },
]

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

export function LibraryPage() {
  const [viewMode, setViewMode] = useState<ViewMode>('tracks')

  return (
    <div className="flex flex-col" style={{ height: '100%' }}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold">Library</h1>
          <p className="text-muted-foreground mt-1">
            {MOCK_TRACKS.length} tracks • {MOCK_ALBUMS.length} albums
          </p>
        </div>

        {/* View mode toggle */}
        <div className="flex items-center gap-2 bg-muted rounded-lg p-1">
          <button
            onClick={() => setViewMode('tracks')}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              viewMode === 'tracks' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
            }`}
            aria-label="View tracks"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
            </svg>
            <span className="text-sm font-medium">Tracks</span>
          </button>
          <button
            onClick={() => setViewMode('albums')}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              viewMode === 'albums' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
            }`}
            aria-label="View albums"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
            </svg>
            <span className="text-sm font-medium">Albums</span>
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {viewMode === 'tracks' ? (
          <div className="border rounded-lg">
            <table className="w-full">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="text-left p-3 text-sm font-medium">#</th>
                  <th className="text-left p-3 text-sm font-medium">Title</th>
                  <th className="text-left p-3 text-sm font-medium">Artist</th>
                  <th className="text-left p-3 text-sm font-medium">Album</th>
                  <th className="text-right p-3 text-sm font-medium">Duration</th>
                </tr>
              </thead>
              <tbody>
                {MOCK_TRACKS.map((track, index) => (
                  <tr
                    key={track.id}
                    className="border-b last:border-0 hover:bg-muted/50 transition-colors cursor-pointer"
                  >
                    <td className="p-3 text-sm text-muted-foreground">{index + 1}</td>
                    <td className="p-3 text-sm font-medium">{track.title}</td>
                    <td className="p-3 text-sm text-muted-foreground">{track.artist}</td>
                    <td className="p-3 text-sm text-muted-foreground">{track.album}</td>
                    <td className="p-3 text-sm text-muted-foreground text-right">
                      {formatDuration(track.duration)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="grid grid-cols-4 gap-4">
            {MOCK_ALBUMS.map((album) => (
              <div
                key={album.id}
                className="group cursor-pointer border rounded-lg p-4 hover:bg-muted/50 transition-colors"
              >
                <div className="aspect-square bg-muted rounded-lg mb-3 flex items-center justify-center">
                  <svg className="w-12 h-12 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
                  </svg>
                </div>
                <h3 className="font-medium truncate">{album.title}</h3>
                <p className="text-sm text-muted-foreground truncate">{album.artist}</p>
                <p className="text-xs text-muted-foreground mt-1">
                  {album.year} • {album.trackCount} tracks
                </p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
