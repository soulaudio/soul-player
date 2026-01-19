'use client'

import { useState, useEffect, useRef } from 'react'
import { Download, ChevronDown, Monitor, Apple, Boxes } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useGitHubRelease } from '@/hooks/useGitHubRelease'
import {
  detectPlatform,
  DOWNLOAD_CONFIGS,
  ALTERNATE_DOWNLOADS,
  fillVersionPattern,
  type Platform
} from '@/utils/downloads'
import { getDownloadUrl, getReleasesPageUrl } from '@/utils/github'
import { LinuxDownloadModal } from './LinuxDownloadModal'

interface PlatformInfo {
  name: string
  Icon: LucideIcon
}

const PLATFORM_INFO: Record<Platform, PlatformInfo> = {
  windows: {
    name: 'Windows',
    Icon: Monitor,
  },
  macos: {
    name: 'macOS',
    Icon: Apple,
  },
  linux: {
    name: 'Linux',
    Icon: Boxes,
  },
  unknown: {
    name: 'Download',
    Icon: Download,
  }
}

export function DownloadButton() {
  const [platform, setPlatform] = useState<Platform>('unknown')
  const [showDropdown, setShowDropdown] = useState(false)
  const [showLinuxModal, setShowLinuxModal] = useState(false)
  const [mounted, setMounted] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)
  const { version, isLoading } = useGitHubRelease()

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

  const currentVersion = version || '0.1.1' // Fallback version
  const currentPlatform = PLATFORM_INFO[platform]
  const downloadConfig = platform !== 'unknown' ? DOWNLOAD_CONFIGS[platform] : null

  // Get download URL for current platform
  const getDownloadUrlForPlatform = (): string => {
    if (!downloadConfig) {
      return getReleasesPageUrl()
    }
    const filename = fillVersionPattern(downloadConfig.filePattern, currentVersion)
    return getDownloadUrl(filename)
  }

  const handlePrimaryDownload = (e: React.MouseEvent<HTMLAnchorElement>) => {
    if (platform === 'linux') {
      e.preventDefault()
      setShowLinuxModal(true)
    }
  }

  // Get alternate platforms for dropdown
  const alternatePlatforms = Object.entries(PLATFORM_INFO)
    .filter(([key]) => key !== platform && key !== 'unknown')
    .map(([key, info]) => ({ key: key as Exclude<Platform, 'unknown'>, ...info }))

  return (
    <>
      <div className="relative inline-block">
        <a
          href={getDownloadUrlForPlatform()}
          onClick={handlePrimaryDownload}
          data-download-button
          className="group inline-flex items-center gap-2 sm:gap-3 px-4 sm:px-6 md:px-8 py-2.5 sm:py-3 md:py-4 bg-primary text-primary-foreground rounded-full font-semibold transition-all duration-700 text-sm sm:text-base md:text-lg shadow-lg hover:scale-105"
        >
          <Download className="w-4 h-4 sm:w-5 sm:h-5 group-hover:translate-y-0.5 transition-transform" />
          <span className="whitespace-nowrap">
            {isLoading ? 'Loading...' : `Download for ${currentPlatform.name}`}
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

          {/* Dropdown menu */}
          {showDropdown && mounted && (
            <div
              data-platforms-dropdown
              className="absolute top-full left-1/2 -translate-x-1/2 mt-2 backdrop-blur-md rounded-xl shadow-2xl overflow-hidden min-w-[220px] z-[9999] transition-colors duration-700 animate-in fade-in slide-in-from-top-2 bg-card border border-border"
            >
              <div className="p-1.5">
                {alternatePlatforms.map(({ key, name, Icon }) => {
                  const config = DOWNLOAD_CONFIGS[key]
                  const url = config
                    ? getDownloadUrl(fillVersionPattern(config.filePattern, currentVersion))
                    : getReleasesPageUrl()

                  const handleClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
                    if (key === 'linux') {
                      e.preventDefault()
                      setShowDropdown(false)
                      setShowLinuxModal(true)
                    } else {
                      setShowDropdown(false)
                    }
                  }

                  return (
                    <a
                      key={key}
                      href={url}
                      onClick={handleClick}
                      data-dropdown-item
                      className="flex items-center gap-3 px-3 py-2.5 text-sm rounded-lg transition-all duration-200 group hover:bg-muted text-foreground"
                    >
                      <Icon className="w-4 h-4 transition-colors duration-200 text-muted-foreground group-hover:text-primary" />
                      <span className="font-medium">{name}</span>
                    </a>
                  )
                })}

                {/* Alternate downloads (e.g., macOS Intel) */}
                {ALTERNATE_DOWNLOADS
                  .filter(alt => alt.platform !== platform)
                  .map((alt) => {
                    const url = getDownloadUrl(fillVersionPattern(alt.filePattern, currentVersion))
                    return (
                      <a
                        key={alt.label}
                        href={url}
                        onClick={() => setShowDropdown(false)}
                        data-dropdown-item
                        className="flex items-center gap-3 px-3 py-2.5 text-sm rounded-lg transition-all duration-200 group hover:bg-muted text-foreground"
                      >
                        <div className="w-4 h-4 rounded bg-muted-foreground/20" />
                        <div className="flex-1">
                          <div className="font-medium text-xs">{alt.label}</div>
                          <div className="text-xs text-muted-foreground">{alt.description}</div>
                        </div>
                      </a>
                    )
                  })}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Linux Download Modal */}
      <LinuxDownloadModal
        isOpen={showLinuxModal}
        onClose={() => setShowLinuxModal(false)}
        version={currentVersion}
      />
    </>
  )
}
