'use client'

import { ReactNode } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
// Using inline SVG icons to match desktop app

interface MainLayoutProps {
  children: ReactNode
}

interface NavTab {
  path: string
  label: string
  icon: ReactNode
}

const NAV_TABS: NavTab[] = [
  { path: '/', label: 'Library', icon: null },
  { path: '/playlists', label: 'Playlists', icon: null },
  { path: '/artists', label: 'Artists', icon: null },
  { path: '/albums', label: 'Albums', icon: null },
  { path: '/genres', label: 'Genres', icon: null },
]

export function MainLayout({ children }: MainLayoutProps) {
  const navigate = useNavigate()
  const location = useLocation()

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/'
    }
    return location.pathname.startsWith(path)
  }

  return (
    <div className="flex flex-col bg-background text-foreground" style={{ height: '100%' }}>
      {/* Header */}
      <header className="border-b bg-card">
        <div className="flex items-center justify-between px-4 py-2">
          {/* Left: Home + Navigation Tabs */}
          <div className="flex items-center gap-1">
            {/* Home Button */}
            <button
              onClick={() => navigate('/')}
              className="p-2 rounded-lg hover:bg-accent transition-colors mr-2"
              aria-label="Home"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
              </svg>
            </button>

            {/* Navigation Tabs */}
            <nav className="flex items-center gap-1">
              {NAV_TABS.map((tab) => (
                <button
                  key={tab.path}
                  onClick={() => navigate(tab.path)}
                  className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                    isActive(tab.path)
                      ? 'bg-primary text-primary-foreground'
                      : 'hover:bg-accent'
                  }`}
                  aria-label={tab.label}
                >
                  {tab.label}
                </button>
              ))}
            </nav>
          </div>

          {/* Right: Search + Action Buttons + Settings */}
          <div className="flex items-center gap-2">
            {/* Search Button */}
            <button
              onClick={() => navigate('/search')}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg hover:bg-accent transition-colors text-sm"
              aria-label="Search"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
            </button>

            {/* Divider */}
            <div className="w-px h-6 bg-border" />

            {/* Import Button */}
            <button
              className="p-2 rounded-lg hover:bg-accent transition-colors"
              aria-label="Import Music"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
              </svg>
            </button>

            {/* Sources Button */}
            <button
              className="p-2 rounded-lg hover:bg-accent transition-colors"
              aria-label="Manage Sources"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
              </svg>
            </button>

            {/* Divider */}
            <div className="w-px h-6 bg-border" />

            {/* Settings Button */}
            <button
              onClick={() => navigate('/settings')}
              className="p-2 rounded-lg hover:bg-accent transition-colors"
              aria-label="Settings"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-auto p-6">{children}</main>

      {/* Player Footer */}
      <footer className="border-t bg-card">
        {/* Main controls row */}
        <div className="p-4">
          <div className="grid grid-cols-3 items-center gap-4">
            {/* Left: Track info */}
            <div className="flex items-center gap-3 min-w-0">
              <div className="w-12 h-12 bg-muted rounded flex-shrink-0" />
              <div className="min-w-0">
                <div className="text-sm font-medium truncate">Bohemian Rhapsody</div>
                <div className="text-xs text-muted-foreground truncate">Queen</div>
              </div>
            </div>

            {/* Center: Playback controls */}
            <div className="flex items-center justify-center gap-2">
              <button className="p-2 hover:bg-accent rounded-full transition-colors">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M8.445 14.832A1 1 0 0010 14v-2.798l5.445 3.63A1 1 0 0017 14V6a1 1 0 00-1.555-.832L10 8.798V6a1 1 0 00-1.555-.832l-6 4a1 1 0 000 1.664l6 4z" />
                </svg>
              </button>
              <button className="p-3 bg-primary text-primary-foreground hover:bg-primary/90 rounded-full transition-colors">
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" />
                </svg>
              </button>
              <button className="p-2 hover:bg-accent rounded-full transition-colors">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M4.555 5.168A1 1 0 003 6v8a1 1 0 001.555.832L10 11.202V14a1 1 0 001.555.832l6-4a1 1 0 000-1.664l-6-4A1 1 0 0010 6v2.798l-5.445-3.63z" />
                </svg>
              </button>
            </div>

            {/* Right: Volume control */}
            <div className="flex items-center justify-end gap-2">
              <button className="p-2 hover:bg-accent rounded-full transition-colors">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.984 5.984 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.983 3.983 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd" />
                </svg>
              </button>
              <div className="w-24 h-1 bg-muted rounded-full overflow-hidden">
                <div className="h-full w-3/4 bg-primary" />
              </div>
            </div>
          </div>
        </div>

        {/* Progress bar row */}
        <div className="px-4 pb-3">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span>2:34</span>
            <div className="flex-1 h-1 bg-muted rounded-full overflow-hidden">
              <div className="h-full w-1/3 bg-primary" />
            </div>
            <span>5:55</span>
          </div>
        </div>
      </footer>
    </div>
  )
}
