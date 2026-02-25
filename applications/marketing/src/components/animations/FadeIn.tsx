'use client'

import { motion, useInView } from 'framer-motion'
import { ReactNode, useRef } from 'react'

interface FadeInProps {
  children: ReactNode
  delay?: number
  direction?: 'up' | 'down' | 'left' | 'right' | 'none'
  fullWidth?: boolean
  className?: string
}

const directionOffset: Record<NonNullable<FadeInProps['direction']>, { x?: number; y?: number }> = {
  up: { y: 24 },
  down: { y: -24 },
  left: { x: 24 },
  right: { x: -24 },
  none: {},
}

export function FadeIn({
  children,
  delay = 0,
  direction = 'up',
  fullWidth = false,
  className = ''
}: FadeInProps) {
  const ref = useRef(null)
  const isInView = useInView(ref, { once: true, amount: 0.1 })
  const offset = directionOffset[direction]

  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, ...offset }}
      animate={isInView ? { opacity: 1, x: 0, y: 0 } : { opacity: 0, ...offset }}
      transition={{ duration: 0.7, delay }}
      className={fullWidth ? 'w-full' : className}
    >
      {children}
    </motion.div>
  )
}
