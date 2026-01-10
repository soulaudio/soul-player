'use client'

import { Fade } from 'react-awesome-reveal'
import { ReactNode } from 'react'

interface FadeInProps {
  children: ReactNode
  delay?: number
  direction?: 'up' | 'down' | 'left' | 'right' | 'none'
  fullWidth?: boolean
  className?: string
}

export function FadeIn({
  children,
  delay = 0,
  direction = 'up',
  fullWidth = false,
  className = ''
}: FadeInProps) {
  // Map direction to react-awesome-reveal direction
  const directionMap = {
    up: 'up',
    down: 'down',
    left: 'left',
    right: 'right',
    none: undefined
  } as const

  return (
    <Fade
      direction={directionMap[direction]}
      delay={delay * 1000}
      duration={700}
      triggerOnce
      fraction={0.1}
      className={fullWidth ? 'w-full' : className}
    >
      {children}
    </Fade>
  )
}
