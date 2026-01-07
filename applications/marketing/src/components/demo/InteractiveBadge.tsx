/**
 * Badge to indicate the demo is interactive
 */

'use client'

import { MousePointer2 } from 'lucide-react'

export function InteractiveBadge() {
  return (
    <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-primary/10 border border-primary/20 text-primary text-sm font-medium">
      <MousePointer2 className="w-4 h-4" />
      <span>Interactive Demo - Click to Play!</span>
    </div>
  )
}
