'use client'

import { useState, useEffect } from 'react'

interface Theme {
  id: string
  name: string
  gradientFrom: string
  gradientTo: string
}

const THEMES: Theme[] = [
  {
    id: 'light',
    name: 'Light',
    gradientFrom: 'rgba(124, 58, 237, 0.15)',
    gradientTo: 'rgba(167, 139, 250, 0.1)'
  },
  {
    id: 'dark',
    name: 'Dark',
    gradientFrom: 'rgba(124, 58, 237, 0.3)',
    gradientTo: 'rgba(167, 139, 250, 0.15)'
  },
  {
    id: 'ocean',
    name: 'Ocean',
    gradientFrom: 'rgba(34, 211, 238, 0.3)',
    gradientTo: 'rgba(14, 165, 233, 0.15)'
  },
]

// Helper function to determine if a theme is light or dark
function isLightTheme(themeId: string): boolean {
  return themeId === 'light' || themeId === 'ocean'
}

export function DemoThemeSwitcher() {
  const [currentTheme, setCurrentTheme] = useState('dark')
  const [isOpen, setIsOpen] = useState(false)

  useEffect(() => {
    // Apply theme to demo container
    const demoContainer = document.querySelector('[data-demo-container]')
    if (demoContainer) {
      demoContainer.setAttribute('data-theme', currentTheme)
    }

    // Update demo backdrop gradient (grainy radial gradient centered on demo)
    const theme = THEMES.find(t => t.id === currentTheme)
    if (theme) {
      const demoBackdrop = document.querySelector('[data-demo-backdrop]')
      if (demoBackdrop) {
        const el = demoBackdrop as HTMLElement
        if (isLightTheme(currentTheme)) {
          if (currentTheme === 'ocean') {
            // Ocean theme - use light cyan gradient
            el.style.background = 'radial-gradient(ellipse 120% 80% at 50% 65%, rgba(34, 211, 238, 0.15) 0%, rgba(14, 165, 233, 0.1) 30%, rgba(34, 211, 238, 0.05) 50%, transparent 70%)'
          } else {
            // Light theme - use light violet gradient
            el.style.background = 'radial-gradient(ellipse 120% 80% at 50% 65%, rgba(124, 58, 237, 0.15) 0%, rgba(167, 139, 250, 0.1) 30%, rgba(124, 58, 237, 0.05) 50%, transparent 70%)'
          }
        } else {
          // Dark theme - deep midnight gradient (much darker and more subtle)
          el.style.background = 'radial-gradient(ellipse 120% 80% at 50% 65%, rgba(88, 50, 180, 0.15) 0%, rgba(75, 40, 160, 0.12) 30%, rgba(60, 30, 140, 0.08) 50%, rgba(45, 20, 100, 0.04) 65%, transparent 80%)'
        }
      }

      // Update branding gradient (bigger ellipse extending lower)
      const brandingGradient = document.querySelector('[data-branding-gradient]')
      if (brandingGradient) {
        const el = brandingGradient as HTMLElement
        if (isLightTheme(currentTheme)) {
          if (currentTheme === 'ocean') {
            el.style.background = 'radial-gradient(ellipse 110% 60% at 50% 50%, rgba(34, 211, 238, 0.2) 0%, rgba(34, 211, 238, 0.12) 35%, rgba(34, 211, 238, 0.04) 65%, transparent 100%)'
          } else {
            el.style.background = 'radial-gradient(ellipse 110% 60% at 50% 50%, rgba(124, 58, 237, 0.15) 0%, rgba(124, 58, 237, 0.1) 35%, rgba(124, 58, 237, 0.03) 65%, transparent 100%)'
          }
        } else {
          el.style.background = 'radial-gradient(ellipse 110% 60% at 50% 50%, rgba(88, 50, 180, 0.12) 0%, rgba(75, 40, 160, 0.08) 35%, rgba(60, 30, 140, 0.04) 65%, transparent 100%)'
        }
      }

      // Update heading gradient
      const headingGradient = document.querySelector('[data-heading-gradient]')
      if (headingGradient) {
        const el = headingGradient as HTMLElement
        const gradientValue = isLightTheme(currentTheme)
          ? 'linear-gradient(to right, rgb(109, 40, 217), rgb(88, 28, 135))' // violet-700 to violet-900 for light themes
          : 'linear-gradient(to right, rgb(124, 58, 237), rgb(117, 49, 227))' // violet-600 to violet-650 (custom) for dark theme

        el.style.backgroundImage = gradientValue
        el.style.setProperty('-webkit-background-clip', 'text')
        el.style.setProperty('-webkit-text-fill-color', 'transparent')
      }

      // Update hero section background color
      const heroSectionBg = document.querySelector('[data-hero-section]')
      if (heroSectionBg) {
        const el = heroSectionBg as HTMLElement
        if (isLightTheme(currentTheme)) {
          if (currentTheme === 'ocean') {
            el.style.setProperty('background-color', 'rgb(224, 242, 254)') // sky-100
          } else {
            el.style.setProperty('background-color', 'rgb(250, 249, 255)') // light purple/gray
          }
        } else {
          el.style.setProperty('background-color', 'hsl(250, 15%, 4%)') // deep midnight
        }
      }

      // Update text colors for contrast
      const mainText = document.querySelector('[data-main-text]')
      if (mainText) {
        const el = mainText as HTMLElement
        el.style.color = isLightTheme(currentTheme)
          ? 'rgb(39, 39, 42)' // zinc-800 for light themes
          : 'rgb(212, 212, 220)' // lighter for dark
      }

      const descText = document.querySelector('[data-desc-text]')
      if (descText) {
        const el = descText as HTMLElement
        el.style.color = isLightTheme(currentTheme)
          ? 'rgb(82, 82, 91)' // zinc-600 for light themes
          : 'rgb(140, 140, 150)' // muted gray for dark
      }

      const badges = document.querySelectorAll('[data-badge-text]')
      badges.forEach((badge) => {
        const el = badge as HTMLElement
        if (isLightTheme(currentTheme)) {
          el.style.color = currentTheme === 'ocean'
            ? 'rgb(82, 82, 91)' // zinc-600 for ocean (better contrast on sky-100)
            : 'rgb(113, 113, 122)' // zinc-500 for light theme
        } else {
          el.style.color = 'rgb(130, 130, 140)' // muted gray for dark
        }
      })

      const themeLabel = document.querySelector('[data-theme-label]')
      if (themeLabel) {
        const el = themeLabel as HTMLElement
        el.style.color = isLightTheme(currentTheme)
          ? 'rgb(82, 82, 91)' // zinc-600 for light themes
          : 'rgb(140, 140, 150)' // muted gray for dark
      }

      // Update Soul Player text color for contrast
      const soulPlayerTitle = Array.from(document.querySelectorAll('h2')).find(
        el => el.textContent?.includes('Soul Player')
      )
      if (soulPlayerTitle) {
        const el = soulPlayerTitle as HTMLElement
        if (isLightTheme(currentTheme)) {
          el.style.color = 'rgb(24, 24, 27)' // zinc-900 for light themes - very dark
          el.style.textShadow = '0 2px 8px rgba(0, 0, 0, 0.35)' // Darker shadow for light themes
        } else {
          el.style.color = 'rgb(225, 225, 230)' // softer light gray for dark
          el.style.textShadow = '0 2px 6px rgba(0, 0, 0, 0.4)' // Subtle shadow for dark theme
        }
      }

      // Update Soul Audio subtitle
      const subtitle = Array.from(document.querySelectorAll('p')).find(
        el => el.textContent?.includes('brought to you by Soul Audio')
      )
      if (subtitle) {
        const el = subtitle as HTMLElement
        if (isLightTheme(currentTheme)) {
          el.style.color = 'rgb(63, 63, 70)' // zinc-700 for light themes
          el.style.textShadow = '0 1px 4px rgba(0, 0, 0, 0.25)' // Darker shadow for light themes
        } else {
          el.style.color = 'rgb(130, 130, 140)' // muted gray for dark
          el.style.textShadow = '0 1px 4px rgba(0, 0, 0, 0.3)' // Subtle shadow for dark theme
        }
      }

      // Update demo container border to match theme
      const demoContainerBorder = document.querySelector('[data-demo-container]')?.parentElement
      if (demoContainerBorder) {
        const el = demoContainerBorder as HTMLElement
        if (isLightTheme(currentTheme)) {
          el.style.borderColor = 'rgba(39, 39, 42, 0.2)' // Dark border for light themes
        } else {
          el.style.borderColor = 'rgba(88, 50, 180, 0.2)' // Subtle purple border for dark theme
        }
      }

      // Update theme button colors to match the overall theme
      const themeButtons = document.querySelectorAll('.hidden.sm\\:flex button')
      themeButtons.forEach((button) => {
        const btn = button as HTMLElement
        const isActive = btn.textContent?.toLowerCase().includes(currentTheme.toLowerCase())

        if (isActive) {
          // Active button styling - matches theme colors
          if (currentTheme === 'ocean') {
            btn.style.backgroundColor = 'rgb(6, 182, 212)' // cyan-600
            btn.style.color = 'rgb(255, 255, 255)' // white
            btn.style.borderColor = 'rgba(34, 211, 238, 0.5)' // cyan-400/50
          } else if (currentTheme === 'light') {
            btn.style.backgroundColor = 'rgb(39, 39, 42)' // zinc-800
            btn.style.color = 'rgb(250, 250, 250)' // zinc-50
            btn.style.borderColor = 'rgba(39, 39, 42, 0.4)' // zinc-800/40
          } else {
            btn.style.backgroundColor = 'rgb(88, 50, 180)' // darker purple
            btn.style.color = 'rgb(225, 225, 230)' // soft white
            btn.style.borderColor = 'rgba(88, 50, 180, 0.4)' // subtle purple border
          }
        } else {
          // Inactive button styling - better contrast
          if (isLightTheme(currentTheme)) {
            btn.style.backgroundColor = 'rgba(255, 255, 255, 0.8)' // white/80
            btn.style.color = 'rgb(63, 63, 70)' // zinc-700
            btn.style.borderColor = 'rgba(228, 228, 231, 0.6)' // zinc-200/60
          } else {
            btn.style.backgroundColor = 'rgba(30, 30, 40, 0.6)' // very dark with hint of purple
            btn.style.color = 'rgb(130, 130, 140)' // muted gray
            btn.style.borderColor = 'rgba(50, 50, 60, 0.4)' // subtle dark border
          }
        }
      })

      // Update download button colors for theme contrast
      const downloadButton = document.querySelector('[data-download-button]')
      if (downloadButton) {
        const btn = downloadButton as HTMLElement
        if (isLightTheme(currentTheme)) {
          // Light themes: Dark button
          btn.style.backgroundColor = 'rgb(24, 24, 27)' // zinc-900
          btn.style.color = 'rgb(250, 250, 250)' // zinc-50
        } else {
          // Dark theme: Subtle purple button
          btn.style.backgroundColor = 'rgb(88, 50, 180)' // purple
          btn.style.color = 'rgb(225, 225, 230)' // soft white
        }
      }

      // Update "Other platforms" text color
      const otherPlatformsButton = document.querySelector('[data-other-platforms]')
      if (otherPlatformsButton) {
        const btn = otherPlatformsButton as HTMLElement
        btn.style.color = isLightTheme(currentTheme)
          ? 'rgb(109, 40, 217)' // violet-700 for light themes
          : 'rgb(170, 140, 230)' // muted purple for dark theme
      }

      // Update platforms dropdown theme
      const platformsDropdown = document.querySelector('[data-platforms-dropdown]')
      if (platformsDropdown) {
        const dropdown = platformsDropdown as HTMLElement
        if (isLightTheme(currentTheme)) {
          // Light theme dropdown
          dropdown.style.background = 'rgba(255, 255, 255, 0.95)'
          dropdown.style.border = '1px solid rgba(228, 228, 231, 0.8)'
        } else {
          // Dark theme dropdown
          dropdown.style.background = 'rgba(20, 15, 30, 0.95)'
          dropdown.style.border = '1px solid rgba(88, 50, 180, 0.3)'
        }
      }

      // Update dropdown items
      const dropdownItems = document.querySelectorAll('[data-dropdown-item]')
      dropdownItems.forEach((item) => {
        const el = item as HTMLElement
        if (isLightTheme(currentTheme)) {
          el.style.color = 'rgb(39, 39, 42)' // zinc-800
          // Update icon colors
          const icon = el.querySelector('svg')
          if (icon) {
            (icon as SVGElement).style.color = 'rgb(82, 82, 91)' // zinc-600
          }
          // Add hover styles
          el.addEventListener('mouseenter', () => {
            el.style.backgroundColor = 'rgba(244, 244, 245, 0.6)' // zinc-100/60
            el.style.color = 'rgb(24, 24, 27)' // zinc-900
          })
          el.addEventListener('mouseleave', () => {
            el.style.backgroundColor = ''
            el.style.color = 'rgb(39, 39, 42)' // zinc-800
          })
        } else {
          el.style.color = 'rgb(200, 200, 210)' // muted light gray
          // Update icon colors
          const icon = el.querySelector('svg')
          if (icon) {
            (icon as SVGElement).style.color = 'rgb(140, 140, 150)' // muted gray
          }
          // Add hover styles
          el.addEventListener('mouseenter', () => {
            el.style.backgroundColor = 'rgba(88, 50, 180, 0.15)' // subtle purple highlight
            el.style.color = 'rgb(220, 220, 230)' // lighter on hover
          })
          el.addEventListener('mouseleave', () => {
            el.style.backgroundColor = ''
            el.style.color = 'rgb(200, 200, 210)' // muted light gray
          })
        }
      })

      // Update dropdown divider
      const dropdownDivider = document.querySelector('[data-dropdown-divider]')
      if (dropdownDivider) {
        const divider = dropdownDivider as HTMLElement
        divider.style.borderTop = isLightTheme(currentTheme)
          ? '1px solid rgba(228, 228, 231, 0.6)' // zinc-200/60
          : '1px solid rgba(88, 50, 180, 0.2)' // subtle purple divider
      }
    }
  }, [currentTheme])

  return (
    <div className="flex items-center gap-2">
      {/* Desktop: Buttons */}
      <div className="hidden sm:flex items-center gap-2">
        {THEMES.map((theme) => (
          <button
            key={theme.id}
            onClick={() => setCurrentTheme(theme.id)}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all border border-zinc-800/50 ${
              currentTheme === theme.id
                ? 'bg-violet-600/30 text-violet-200'
                : 'bg-zinc-900/50 text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'
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
          className="flex items-center gap-2 px-3 py-2 bg-zinc-900/80 backdrop-blur-sm border border-zinc-800/50 rounded-lg hover:bg-zinc-800/80 transition-colors text-sm text-zinc-300"
          aria-label="Switch theme"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
          </svg>
          <span>{THEMES.find(t => t.id === currentTheme)?.name}</span>
        </button>

        {isOpen && (
          <>
            {/* Backdrop */}
            <div
              className="fixed inset-0 z-[70]"
              onClick={() => setIsOpen(false)}
            />

            {/* Dropdown */}
            <div className="absolute bottom-full mb-2 left-0 z-[80] bg-zinc-900/95 backdrop-blur-sm border border-zinc-800/50 rounded-lg shadow-xl overflow-hidden min-w-[140px]">
              {THEMES.map((theme) => (
                <button
                  key={theme.id}
                  onClick={() => {
                    setCurrentTheme(theme.id)
                    setIsOpen(false)
                  }}
                  className={`w-full text-left px-4 py-2 text-sm transition-colors ${
                    currentTheme === theme.id
                      ? 'bg-violet-600/20 text-violet-300'
                      : 'text-zinc-300 hover:bg-zinc-800/80'
                  }`}
                >
                  {theme.name}
                  {currentTheme === theme.id && (
                    <span className="ml-2 text-violet-400">✓</span>
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
