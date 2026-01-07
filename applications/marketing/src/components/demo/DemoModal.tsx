/**
 * Simple modal/dialog for demo
 */

'use client'

import { ReactNode } from 'react'
import { X } from 'lucide-react'

interface DemoModalProps {
  isOpen: boolean
  onClose: () => void
  title: string
  children: ReactNode
}

export function DemoModal({ isOpen, onClose, title, children }: DemoModalProps) {
  if (!isOpen) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="relative bg-card border rounded-lg shadow-lg w-full max-w-2xl max-h-[80vh] overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-lg font-semibold">{title}</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-accent rounded-md transition-colors"
            aria-label="Close"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto max-h-[calc(80vh-80px)]">
          {children}
        </div>
      </div>
    </div>
  )
}
