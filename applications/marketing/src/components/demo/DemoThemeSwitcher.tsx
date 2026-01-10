'use client'

import { useState, useEffect } from 'react'
import { useTheme, builtInThemes } from '@soul-player/shared'

export function DemoThemeSwitcher() {
  const { currentTheme, setTheme } = useTheme()
  const [isOpen, setIsOpen] = useState(false)

  // Apply theme to demo container when it changes
  useEffect(() => {
    const demoContainer = document.querySelector('[data-demo-container]')
    if (demoContainer) {
      demoContainer.setAttribute('data-theme', currentTheme.id)
    }
  }, [currentTheme.id])

  return (
    <div className="flex items-center gap-2">
      {/* Desktop: Buttons */}
      <div className="hidden sm:flex items-center gap-2">
        {builtInThemes.map((theme) => (
          <button
            key={theme.id}
            onClick={() => setTheme(theme.id)}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all border ${
              currentTheme.id === theme.id
                ? 'bg-primary/20 text-primary-foreground border-primary/30'
                : 'bg-muted/50 text-muted-foreground hover:bg-muted/80 hover:text-foreground border-border'
            }`}
          >
            {theme.name}
          </button>
        ))}
      </div>

      {/* Mobile: Dropdown */}
      <div className="sm:hidden relative">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className="flex items-center gap-2 px-3 py-2 bg-muted/80 backdrop-blur-sm border border-border rounded-lg hover:bg-muted transition-colors text-sm text-muted-foreground"
          aria-label="Switch theme"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
          </svg>
          <span>{currentTheme.name}</span>
        </button>

        {isOpen && (
          <>
            {/* Backdrop */}
            <div
              className="fixed inset-0 z-[70]"
              onClick={() => setIsOpen(false)}
            />

            {/* Dropdown */}
            <div className="absolute bottom-full mb-2 left-0 z-[80] bg-card/95 backdrop-blur-sm border border-border rounded-lg shadow-xl overflow-hidden min-w-[140px]">
              {builtInThemes.map((theme) => (
                <button
                  key={theme.id}
                  onClick={() => {
                    setTheme(theme.id)
                    setIsOpen(false)
                  }}
                  className={`w-full text-left px-4 py-2 text-sm transition-colors ${
                    currentTheme.id === theme.id
                      ? 'bg-primary/20 text-primary-foreground'
                      : 'text-muted-foreground hover:bg-muted/80'
                  }`}
                >
                  {theme.name}
                  {currentTheme.id === theme.id && (
                    <span className="ml-2 text-primary">✓</span>
                  )}
                </button>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
