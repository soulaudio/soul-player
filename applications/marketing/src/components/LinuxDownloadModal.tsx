'use client'

import { useState, useEffect, useRef } from 'react'
import { X, Download, Terminal, Package, Box, Archive, CheckCircle2, AlertCircle } from 'lucide-react'
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

// Map of which formats support auto-updates
const AUTO_UPDATE_SUPPORT: Record<string, boolean> = {
  appimage: true,
  flatpak: false,
  deb: false,
  rpm: false,
  aur: false,
}

// Map of package manager update commands
const UPDATE_COMMANDS: Record<string, string> = {
  deb: 'sudo apt update && sudo apt upgrade soul-player',
  rpm: 'sudo dnf upgrade soul-player',
  flatpak: 'flatpak update io.github.soulaudio.SoulPlayer',
  aur: 'yay -Syu soul-player',
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
            className="p-2 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity"
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

              const supportsAutoUpdate = AUTO_UPDATE_SUPPORT[download.id] ?? false
              const updateCommand = UPDATE_COMMANDS[download.id]

              return (
                <div
                  key={download.id}
                  className="group relative p-4 rounded-xl border border-border bg-card hover:border-primary/30 hover:bg-foreground/[var(--hover-bg-opacity)] transition-all duration-[var(--transition-duration)]"
                >
                  <div className="flex items-start gap-4">
                    <div className="p-3 rounded-lg bg-muted group-hover:scale-110 transition-transform">
                      <Icon className="w-6 h-6 text-muted-foreground" />
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <h3 className="text-lg font-semibold text-foreground">
                          {download.displayName}
                        </h3>
                        {supportsAutoUpdate ? (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 bg-green-500/10 text-green-600 dark:text-green-400 rounded-full text-xs font-medium">
                            <CheckCircle2 className="w-3 h-3" />
                            Auto-updates
                          </span>
                        ) : (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 rounded-full text-xs font-medium">
                            <AlertCircle className="w-3 h-3" />
                            Manual updates
                          </span>
                        )}
                      </div>
                      <p className="text-sm text-muted-foreground mt-1">
                        {download.description}
                      </p>

                      {/* Update command for non-auto-update formats */}
                      {!supportsAutoUpdate && updateCommand && (
                        <div className="mt-2 p-2 bg-muted/50 rounded-lg border border-border/50">
                          <p className="text-xs text-muted-foreground mb-1.5">Update command:</p>
                          <div className="flex items-center gap-2">
                            <code className="flex-1 px-2 py-1.5 bg-background border rounded text-xs font-mono">
                              {updateCommand}
                            </code>
                            <button
                              onClick={() => handleCopyCommand(download.id + '-update', updateCommand)}
                              className="px-2 py-1.5 bg-primary/10 hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)] text-primary rounded text-xs font-medium"
                            >
                              {copiedId === download.id + '-update' ? 'Copied!' : 'Copy'}
                            </button>
                          </div>
                        </div>
                      )}

                      {/* Install command (for AUR mainly) */}
                      {download.installCommand && (
                        <div className="mt-2">
                          <p className="text-xs text-muted-foreground mb-1.5">Install command:</p>
                          <div className="flex items-center gap-2">
                            <code className="flex-1 px-2 py-1.5 bg-muted/80 text-foreground rounded text-xs font-mono">
                              {download.installCommand}
                            </code>
                            <button
                              onClick={() => handleCopyCommand(download.id, download.installCommand!)}
                              className="px-2 py-1.5 bg-primary/10 hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)] text-primary rounded text-xs font-medium"
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
                <strong className="text-foreground">AppImage</strong> - Universal format that works everywhere. No installation needed, supports in-app auto-updates.
              </li>
              <li>
                <strong className="text-foreground">Flatpak</strong> - Sandboxed app with desktop integration. Updates via Flathub (coming soon).
              </li>
              <li>
                <strong className="text-foreground">DEB/RPM</strong> - Native packages for better system integration. Updates via package manager.
              </li>
              <li>
                <strong className="text-foreground">AUR</strong> - For Arch Linux users. Always up-to-date via AUR helpers like yay.
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}
