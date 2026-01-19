'use client'

import { useState, useEffect, useRef } from 'react'
import { X, Download, Apple, Cpu } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { DOWNLOAD_CONFIGS, ALTERNATE_DOWNLOADS, fillVersionPattern } from '@/utils/downloads'
import { getDownloadUrl } from '@/utils/github'

interface MacosDownloadModalProps {
  isOpen: boolean
  onClose: () => void
  version: string | null
}

interface MacosOption {
  id: string
  filePattern: string
  displayName: string
  description: string
  recommended: boolean
  icon: LucideIcon
}

export function MacosDownloadModal({ isOpen, onClose, version }: MacosDownloadModalProps) {
  const modalRef = useRef<HTMLDivElement>(null)

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

  if (!isOpen) return null

  const macosOptions: MacosOption[] = [
    {
      id: 'apple-silicon',
      filePattern: DOWNLOAD_CONFIGS.macos.filePattern,
      displayName: 'Apple Silicon (M1/M2/M3/M4)',
      description: 'For Macs with Apple Silicon processors',
      recommended: true,
      icon: Apple,
    },
    {
      id: 'intel',
      filePattern: ALTERNATE_DOWNLOADS.find((d) => d.platform === 'macos')?.filePattern || '',
      displayName: 'Intel (x64)',
      description: 'For Intel-based Macs',
      recommended: false,
      icon: Cpu,
    },
  ]

  return (
    <div className="fixed inset-0 z-[99999] flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div
        ref={modalRef}
        className="relative w-full max-w-2xl max-h-[90vh] overflow-hidden bg-card border border-border rounded-2xl shadow-2xl animate-in zoom-in-95 duration-200"
      >
        {/* Header */}
        <div className="sticky top-0 z-10 flex items-center justify-between px-6 py-4 border-b border-border bg-card/95 backdrop-blur-sm">
          <div>
            <h2 className="text-2xl font-bold text-foreground">Download for macOS</h2>
            <p className="text-sm text-muted-foreground mt-1">
              Choose your Mac's processor type
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
            {macosOptions.map((option) => {
              const Icon = option.icon
              const filename = fillVersionPattern(option.filePattern, version || '0.1.1')
              const downloadUrl = getDownloadUrl(filename)

              return (
                <div
                  key={option.id}
                  className={`group relative p-4 rounded-xl border transition-all duration-200 ${
                    option.recommended
                      ? 'border-primary/50 bg-primary/5 hover:border-primary hover:bg-primary/10'
                      : 'border-border bg-card hover:border-primary/30 hover:bg-muted/50'
                  }`}
                >
                  {option.recommended && (
                    <div className="absolute -top-2.5 left-4 px-2.5 py-0.5 bg-primary text-primary-foreground text-xs font-semibold rounded-full">
                      Recommended
                    </div>
                  )}

                  <div className="flex items-start gap-4">
                    <div className={`p-3 rounded-lg ${
                      option.recommended ? 'bg-primary/10' : 'bg-muted'
                    } group-hover:scale-110 transition-transform`}>
                      <Icon className={`w-6 h-6 ${
                        option.recommended ? 'text-primary' : 'text-muted-foreground'
                      }`} />
                    </div>

                    <div className="flex-1 min-w-0">
                      <h3 className="text-lg font-semibold text-foreground">
                        {option.displayName}
                      </h3>
                      <p className="text-sm text-muted-foreground mt-1">
                        {option.description}
                      </p>
                    </div>

                    <div className="flex-shrink-0">
                      <a
                        href={downloadUrl}
                        className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg font-medium hover:scale-105 transition-transform"
                      >
                        <Download className="w-4 h-4" />
                        Download
                      </a>
                    </div>
                  </div>
                </div>
              )
            })}
          </div>

          {/* Help text */}
          <div className="mt-6 p-4 bg-muted/50 rounded-xl border border-border">
            <h4 className="text-sm font-semibold text-foreground mb-2">How to check your Mac's processor</h4>
            <ul className="text-sm text-muted-foreground space-y-1.5">
              <li>
                <strong className="text-foreground">Apple menu</strong> → About This Mac → Check "Chip" or "Processor"
              </li>
              <li>
                <strong className="text-foreground">Apple Silicon</strong> - Shows "Apple M1", "Apple M2", "Apple M3", or "Apple M4"
              </li>
              <li>
                <strong className="text-foreground">Intel</strong> - Shows "Intel Core i5", "Intel Core i7", etc.
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}
