'use client'

import React from 'react'
import { ScrollRotate3DExample } from '@/components/animations/ScrollRotate3D.example'

/**
 * Demo page for ScrollRotate3D component
 *
 * Access at: http://localhost:3000/scroll-rotate-demo
 *
 * This page can be removed after testing or kept for reference.
 */
export default function ScrollRotateDemoPage() {
  return (
    <main className="min-h-screen" style={{ backgroundColor: 'hsl(var(--background))' }}>
      {/* Header */}
      <div className="py-16 text-center">
        <h1
          className="text-4xl md:text-5xl lg:text-6xl font-serif font-bold"
          style={{ color: 'hsl(var(--foreground))' }}
        >
          ScrollRotate3D Component Demo
        </h1>
        <p
          className="mt-4 text-lg md:text-xl max-w-2xl mx-auto px-6"
          style={{ color: 'hsl(var(--muted-foreground))' }}
        >
          Scroll down to see various 3D rotation effects. Each section demonstrates different
          configuration options.
        </p>
      </div>

      {/* Examples */}
      <ScrollRotate3DExample />
    </main>
  )
}
