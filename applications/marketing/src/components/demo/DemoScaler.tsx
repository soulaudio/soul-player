'use client'

import { ReactNode, useEffect, useRef, useState } from 'react'

interface DemoScalerProps {
  children: ReactNode
  /** Design width - the width the demo was designed for */
  designWidth?: number
  /** Design height - the height the demo was designed for */
  designHeight?: number
  /** Minimum scale factor to prevent demo from becoming too small */
  minScale?: number
}

/**
 * Scales demo content proportionally to fit container while keeping everything visible.
 * Acts like a responsive image - the entire demo scales as one unit.
 */
export function DemoScaler({
  children,
  designWidth = 1200,
  designHeight = 750,
  minScale = 0.4,
}: DemoScalerProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const contentRef = useRef<HTMLDivElement>(null)
  const [scale, setScale] = useState(1)

  useEffect(() => {
    const updateScale = () => {
      if (!containerRef.current || !contentRef.current) return

      const container = containerRef.current.getBoundingClientRect()

      // Calculate scale to fit both width and height
      const scaleX = container.width / designWidth
      const scaleY = container.height / designHeight

      // Use the smaller scale to ensure everything fits
      const newScale = Math.max(Math.min(scaleX, scaleY), minScale)

      setScale(newScale)
    }

    // Initial scale calculation
    updateScale()

    // Create ResizeObserver to watch container size changes
    const resizeObserver = new ResizeObserver(updateScale)
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current)
    }

    // Also listen to window resize as backup
    window.addEventListener('resize', updateScale)

    return () => {
      resizeObserver.disconnect()
      window.removeEventListener('resize', updateScale)
    }
  }, [designWidth, designHeight, minScale])

  // Calculate the scaled dimensions for proper centering
  const scaledWidth = designWidth * scale
  const scaledHeight = designHeight * scale

  return (
    <div
      ref={containerRef}
      className="w-full h-full flex items-center justify-center overflow-hidden"
    >
      <div
        style={{
          width: scaledWidth,
          height: scaledHeight,
          position: 'relative',
        }}
      >
        <div
          ref={contentRef}
          style={{
            width: designWidth,
            height: designHeight,
            transform: `scale(${scale})`,
            transformOrigin: 'top left',
            position: 'absolute',
            top: 0,
            left: 0,
          }}
        >
          {children}
        </div>
      </div>
    </div>
  )
}
