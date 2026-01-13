'use client'

import { useState, useEffect } from 'react'

const ROTATING_WORDS = [
  'enjoy',
  'discover',
  'organize',
  'experience',
  'curate',
]

export function RotatingText() {
  const [currentIndex, setCurrentIndex] = useState(0)
  const [isAnimating, setIsAnimating] = useState(false)

  useEffect(() => {
    const interval = setInterval(() => {
      setIsAnimating(true)
      setTimeout(() => {
        setCurrentIndex((prev) => (prev + 1) % ROTATING_WORDS.length)
        setIsAnimating(false)
      }, 300)
    }, 3000)

    return () => clearInterval(interval)
  }, [])

  return (
    <span
      className={`inline-block transition-all duration-300 text-transparent bg-clip-text ${
        isAnimating ? 'opacity-0 translate-y-2' : 'opacity-100 translate-y-0'
      }`}
      style={{
        backgroundImage: 'linear-gradient(135deg, hsl(var(--primary)) 0%, color-mix(in srgb, hsl(var(--primary)) 30%, hsl(var(--foreground)) 70%) 30%, hsl(var(--foreground)) 50%, color-mix(in srgb, hsl(var(--foreground)) 70%, hsl(var(--accent)) 30%) 70%, color-mix(in srgb, hsl(var(--foreground)) 60%, hsl(var(--accent)) 40%) 100%)',
        WebkitBackgroundClip: 'text',
        WebkitTextFillColor: 'transparent',
      }}
    >
      {ROTATING_WORDS[currentIndex]}
    </span>
  )
}
