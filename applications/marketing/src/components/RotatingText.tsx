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
      className={`inline-block transition-all duration-300 ${
        isAnimating ? 'opacity-0 translate-y-2' : 'opacity-100 translate-y-0'
      }`}
    >
      {ROTATING_WORDS[currentIndex]}
    </span>
  )
}
