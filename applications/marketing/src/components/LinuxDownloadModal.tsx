'use client'

import { useState, useEffect, useRef } from 'react'
import { X, Download, Terminal, Package, Box, Archive } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { LINUX_DOWNLOADS, fillVersionPattern, type LinuxDownload } from '@/utils/downloads'
import { getDownloadUrl } from '@/utils/github'

interface LinuxDownloadModalProps {
  isOpen: boolean
  onClose: () => void
  version: string | null
}

const LINUX_ICONS: Record<string, LucideIcon> = {
  appimage: Package,
  flatpak: Box,
  deb: Archive,
  rpm: Archive,
  aur: Terminal,
}

export function LinuxDownloadModal({ isOpen, onClose, version }: LinuxDownloadModalProps) {
  const modalRef = useRef<HTMLDivElement>(null)
  const [copiedId, setCopiedId] = useState<string | null>(null)

  // Close on escape key
  useEffect(() => {
    if (!isOpen) return

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      }
    }

    document.addEventListener('keydown', handleEscape)
    return () => document.removeEventListener('keydown', handleEscape)
  }, [isOpen, onClose])

  // Close when clicking outside
  useEffect(() => {
    if (!isOpen) return

    const handleClickOutside = (e: MouseEvent) => {
      if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
        onClose()
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [isOpen, onClose])

  // Prevent body scroll when modal is open
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = ''
    }
    return () => {
      document.body.style.overflow = ''
    }
  }, [isOpen])

  const handleCopyCommand = (id: string, command: string) => {
    navigator.clipboard.writeText(command)
    setCopiedId(id)
    setTimeout(() => setCopiedId(null), 2000)
  }

  const getDownloadUrlForLinux = (download: LinuxDownload): string => {
    if (download.isAur) {
      return 'https://aur.archlinux.org/packages/soul-player'
    }
    const filename = fillVersionPattern(download.filePattern, version || '0.1.1')
    return getDownloadUrl(filename)
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-[99999] flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div
        ref={modalRef}
        className="relative w-full max-w-2xl max-h-[90vh] overflow-hidden bg-card border border-border rounded-2xl shadow-2xl animate-in zoom-in-95 duration-200"
      >
        {/* Header */}
        <div className="sticky top-0 z-10 flex items-center justify-between px-6 py-4 border-b border-border bg-card/95 backdrop-blur-sm">
          <div>
            <h2 className="text-2xl font-bold text-foreground">Download for Linux</h2>
            <p className="text-sm text-muted-foreground mt-1">
              Choose your preferred package format
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
            aria-label="Close modal"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="overflow-y-auto max-h-[calc(90vh-5rem)] px-6 py-4">
          <div className="space-y-3">
            {LINUX_DOWNLOADS.map((download) => {
              const Icon = LINUX_ICONS[download.id] || Package
              const downloadUrl = getDownloadUrlForLinux(download)
              const isAur = download.isAur

              return (
                <div
                  key={download.id}
                  className={`group relative p-4 rounded-xl border transition-all duration-200 ${
                    download.recommended
                      ? 'border-primary/50 bg-primary/5 hover:border-primary hover:bg-primary/10'
                      : 'border-border bg-card hover:border-primary/30 hover:bg-muted/50'
                  }`}
                >
                  {download.recommended && (
                    <div className="absolute -top-2.5 left-4 px-2.5 py-0.5 bg-primary text-primary-foreground text-xs font-semibold rounded-full">
                      Recommended
                    </div>
                  )}

                  <div className="flex items-start gap-4">
                    <div className={`p-3 rounded-lg ${
                      download.recommended ? 'bg-primary/10' : 'bg-muted'
                    } group-hover:scale-110 transition-transform`}>
                      <Icon className={`w-6 h-6 ${
                        download.recommended ? 'text-primary' : 'text-muted-foreground'
                      }`} />
                    </div>

                    <div className="flex-1 min-w-0">
                      <h3 className="text-lg font-semibold text-foreground">
                        {download.displayName}
                      </h3>
                      <p className="text-sm text-muted-foreground mt-1">
                        {download.description}
                      </p>

                      {download.installCommand && (
                        <div className="mt-3">
                          <div className="flex items-center gap-2">
                            <code className="flex-1 px-3 py-2 bg-muted/80 text-foreground rounded-lg text-sm font-mono">
                              {download.installCommand}
                            </code>
                            <button
                              onClick={() => handleCopyCommand(download.id, download.installCommand!)}
                              className="px-3 py-2 bg-primary/10 hover:bg-primary/20 text-primary rounded-lg text-sm font-medium transition-colors"
                            >
                              {copiedId === download.id ? 'Copied!' : 'Copy'}
                            </button>
                          </div>
                        </div>
                      )}
                    </div>

                    <div className="flex-shrink-0">
                      {isAur ? (
                        <a
                          href={downloadUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg font-medium hover:scale-105 transition-transform"
                        >
                          View AUR
                        </a>
                      ) : (
                        <a
                          href={downloadUrl}
                          className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg font-medium hover:scale-105 transition-transform"
                        >
                          <Download className="w-4 h-4" />
                          Download
                        </a>
                      )}
                    </div>
                  </div>
                </div>
              )
            })}
          </div>

          {/* Help text */}
          <div className="mt-6 p-4 bg-muted/50 rounded-xl border border-border">
            <h4 className="text-sm font-semibold text-foreground mb-2">Need help choosing?</h4>
            <ul className="text-sm text-muted-foreground space-y-1.5">
              <li>
                <strong className="text-foreground">AppImage</strong> - Works everywhere, no installation needed. Just make it executable and run.
              </li>
              <li>
                <strong className="text-foreground">Flatpak</strong> - Sandboxed app with automatic updates via Flathub (coming soon).
              </li>
              <li>
                <strong className="text-foreground">DEB/RPM</strong> - Native packages for Debian/Ubuntu and Fedora/RHEL systems.
              </li>
              <li>
                <strong className="text-foreground">AUR</strong> - For Arch Linux users, install via your favorite AUR helper.
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}
