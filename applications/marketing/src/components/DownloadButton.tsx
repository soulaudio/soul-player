'use client'

import { useState, useEffect, useRef } from 'react'
import { Download, ChevronDown, Monitor, Apple, Boxes } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

type Platform = 'windows' | 'macos' | 'linux' | 'unknown'

interface PlatformInfo {
  name: string
  Icon: LucideIcon
  downloadUrl: string
}

const PLATFORMS: Record<Platform, PlatformInfo> = {
  windows: {
    name: 'Windows',
    Icon: Monitor,
    downloadUrl: '#download-windows'
  },
  macos: {
    name: 'macOS',
    Icon: Apple,
    downloadUrl: '#download-macos'
  },
  linux: {
    name: 'Linux',
    Icon: Boxes,
    downloadUrl: '#download-linux'
  },
  unknown: {
    name: 'Download',
    Icon: Download,
    downloadUrl: '#download'
  }
}

function detectPlatform(): Platform {
  if (typeof window === 'undefined') return 'unknown'

  const ua = window.navigator.userAgent.toLowerCase()

  if (ua.includes('win')) return 'windows'
  if (ua.includes('mac')) return 'macos'
  if (ua.includes('linux')) return 'linux'

  return 'unknown'
}

export function DownloadButton() {
  const [platform, setPlatform] = useState<Platform>('unknown')
  const [showDropdown, setShowDropdown] = useState(false)
  const [mounted, setMounted] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setPlatform(detectPlatform())
    setMounted(true)
  }, [])

  // Close dropdown when clicking outside
  useEffect(() => {
    if (!showDropdown) return

    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowDropdown(false)
      }
    }

    const handleScroll = () => {
      setShowDropdown(false)
    }

    document.addEventListener('mousedown', handleClickOutside)
    window.addEventListener('scroll', handleScroll, { passive: true })

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      window.removeEventListener('scroll', handleScroll)
    }
  }, [showDropdown])

  const currentPlatform = PLATFORMS[platform]
  const otherPlatforms = Object.entries(PLATFORMS).filter(([key]) => key !== platform && key !== 'unknown')

  return (
    <div className="relative inline-block">
      <a
        href={currentPlatform.downloadUrl}
        data-download-button
        className="group inline-flex items-center gap-2 sm:gap-3 px-4 sm:px-6 md:px-8 py-2.5 sm:py-3 md:py-4 bg-primary text-primary-foreground rounded-full font-semibold transition-all duration-700 text-sm sm:text-base md:text-lg shadow-lg hover:scale-105"
      >
        <Download className="w-4 h-4 sm:w-5 sm:h-5 group-hover:translate-y-0.5 transition-transform" />
        <span className="whitespace-nowrap">
          Download for {currentPlatform.name}
        </span>
      </a>

      <div className="mt-2 text-center relative" ref={dropdownRef}>
        <button
          onClick={() => setShowDropdown(!showDropdown)}
          data-other-platforms
          className="text-sm transition-colors duration-700 inline-flex items-center gap-1 hover:opacity-80 text-muted-foreground"
        >
          Other platforms
          <ChevronDown className={`w-3 h-3 transition-transform duration-200 ${showDropdown ? 'rotate-180' : ''}`} />
        </button>

        {/* Dropdown menu - positioned absolutely below the button */}
        {showDropdown && mounted && (
          <div
            data-platforms-dropdown
            className="absolute top-full left-1/2 -translate-x-1/2 mt-2 backdrop-blur-md rounded-xl shadow-2xl overflow-hidden min-w-[220px] z-[9999] transition-colors duration-700 animate-in fade-in slide-in-from-top-2 bg-card border border-border"
          >
            <div className="p-1.5">
              {otherPlatforms.map(([key, info]) => {
                const PlatformIcon = info.Icon
                return (
                  <a
                    key={key}
                    href={info.downloadUrl}
                    onClick={() => setShowDropdown(false)}
                    data-dropdown-item
                    className="flex items-center gap-3 px-3 py-2.5 text-sm rounded-lg transition-all duration-200 group hover:bg-muted text-foreground"
                  >
                    <PlatformIcon className="w-4 h-4 transition-colors duration-200 text-muted-foreground group-hover:text-primary" />
                    <span className="font-medium">{info.name}</span>
                  </a>
                )
              })}
            </div>

            <div data-dropdown-divider className="p-1.5 border-t border-border">
              <a
                href="#download-server"
                onClick={() => setShowDropdown(false)}
                data-dropdown-item
                className="flex items-center gap-3 px-3 py-2.5 text-sm rounded-lg transition-all duration-200 group hover:bg-muted text-foreground"
              >
                <Boxes className="w-4 h-4 transition-colors duration-200 text-muted-foreground group-hover:text-primary" />
                <span className="font-medium">Server (Docker)</span>
              </a>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
