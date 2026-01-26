'use client'

import React, { useEffect, useRef, useState } from 'react'

interface ScrollRotate3DProps {
  children: React.ReactNode
  initialRotateY?: number
  maxRotateY?: number
  initialRotateX?: number
  maxRotateX?: number
  perspective?: number
}

export function ScrollRotate3D({
  children,
  initialRotateY = 15,
  maxRotateY = -15,
  initialRotateX = -5,
  maxRotateX = 5,
  perspective = 1200,
}: ScrollRotate3DProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [rotateY, setRotateY] = useState(initialRotateY)
  const [rotateX, setRotateX] = useState(initialRotateX)

  useEffect(() => {
    const element = containerRef.current
    if (!element) return

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const rect = entry.boundingClientRect
            const windowHeight = window.innerHeight
            const elementTop = rect.top
            const elementHeight = rect.height
            const progress = Math.max(0, Math.min(1, (windowHeight - elementTop) / (windowHeight + elementHeight)))

            setRotateY(initialRotateY + (maxRotateY - initialRotateY) * progress)
            setRotateX(initialRotateX + (maxRotateX - initialRotateX) * progress)
          }
        })
      },
      {
        threshold: [0, 0.25, 0.5, 0.75, 1],
        rootMargin: '0px',
      }
    )

    observer.observe(element)
    return () => observer.disconnect()
  }, [initialRotateY, maxRotateY, initialRotateX, maxRotateX])

  return (
    <div ref={containerRef} style={{ perspective: `${perspective}px` }}>
      <div style={{ transform: `rotateY(${rotateY}deg) rotateX(${rotateX}deg)`, transition: 'transform 0.3s ease-out' }}>
        {children}
      </div>
    </div>
  )
}
