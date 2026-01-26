'use client'

import React from 'react'
import { ScrollRotate3D } from './ScrollRotate3D'

/**
 * Example usage of ScrollRotate3D component
 *
 * This file demonstrates different use cases and configurations.
 * Remove or adapt as needed for your marketing site.
 */

export function ScrollRotate3DExample() {
  return (
    <div className="min-h-screen py-32 space-y-96">
      {/* Example 1: Default behavior */}
      <section className="max-w-4xl mx-auto px-6">
        <h2 className="text-3xl font-bold mb-12 text-center" style={{ color: 'hsl(var(--foreground))' }}>
          Default Configuration
        </h2>
        <ScrollRotate3D>
          <div
            className="p-12 rounded-2xl"
            style={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
            }}
          >
            <h3 className="text-2xl font-semibold mb-4" style={{ color: 'hsl(var(--foreground))' }}>
              Default 3D Scroll Effect
            </h3>
            <p style={{ color: 'hsl(var(--muted-foreground))' }}>
              Scroll up and down to see this card rotate and scale smoothly.
              Default: 15deg to -15deg Y-rotation, -5deg to 5deg X-rotation.
            </p>
          </div>
        </ScrollRotate3D>
      </section>

      {/* Example 2: Subtle effect */}
      <section className="max-w-4xl mx-auto px-6">
        <h2 className="text-3xl font-bold mb-12 text-center" style={{ color: 'hsl(var(--foreground))' }}>
          Subtle Effect
        </h2>
        <ScrollRotate3D
          initialRotateY={8}
          maxRotateY={-8}
          initialRotateX={-3}
          maxRotateX={3}
        >
          <div
            className="p-12 rounded-2xl"
            style={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
            }}
          >
            <h3 className="text-2xl font-semibold mb-4" style={{ color: 'hsl(var(--foreground))' }}>
              Subtle Movement
            </h3>
            <p style={{ color: 'hsl(var(--muted-foreground))' }}>
              More subtle rotation (8deg to -8deg) with minimal scale change.
              Great for professional layouts.
            </p>
          </div>
        </ScrollRotate3D>
      </section>

      {/* Example 3: Dramatic effect */}
      <section className="max-w-4xl mx-auto px-6">
        <h2 className="text-3xl font-bold mb-12 text-center" style={{ color: 'hsl(var(--foreground))' }}>
          Dramatic Effect
        </h2>
        <ScrollRotate3D
          initialRotateY={25}
          maxRotateY={-25}
          initialRotateX={-10}
          maxRotateX={10}
          perspective={800}
        >
          <div
            className="p-12 rounded-2xl"
            style={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              boxShadow: '0 25px 50px -12px hsl(var(--primary) / 0.25)',
            }}
          >
            <h3 className="text-2xl font-semibold mb-4" style={{ color: 'hsl(var(--foreground))' }}>
              Dramatic 3D Movement
            </h3>
            <p style={{ color: 'hsl(var(--muted-foreground))' }}>
              Larger rotation angles (25deg to -25deg) with pronounced scale effect.
              Lower perspective (800px) for more pronounced depth.
            </p>
          </div>
        </ScrollRotate3D>
      </section>

      {/* Example 4: With image/screenshot */}
      <section className="max-w-5xl mx-auto px-6">
        <h2 className="text-3xl font-bold mb-12 text-center" style={{ color: 'hsl(var(--foreground))' }}>
          Product Screenshot
        </h2>
        <ScrollRotate3D
          initialRotateY={12}
          maxRotateY={-12}
          initialRotateX={-4}
          maxRotateX={4}
        >
          <div
            className="rounded-2xl overflow-hidden"
            style={{
              backgroundColor: 'hsl(var(--muted))',
              border: '1px solid hsl(var(--border))',
              boxShadow: '0 25px 50px -12px hsl(var(--primary) / 0.15)',
            }}
          >
            {/* Placeholder for actual screenshot */}
            <div className="aspect-video flex items-center justify-center p-12">
              <div className="text-center">
                <div
                  className="w-24 h-24 mx-auto mb-6 rounded-full flex items-center justify-center"
                  style={{ backgroundColor: 'hsl(var(--primary) / 0.1)' }}
                >
                  <svg className="w-12 h-12" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
                  </svg>
                </div>
                <p className="text-lg" style={{ color: 'hsl(var(--muted-foreground))' }}>
                  Replace with actual product screenshot
                </p>
              </div>
            </div>
          </div>
        </ScrollRotate3D>
      </section>

      {/* Spacer for scrolling */}
      <div className="h-96" />
    </div>
  )
}
