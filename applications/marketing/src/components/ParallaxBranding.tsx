'use client'

import { useEffect, useState } from 'react'

export function ParallaxBranding() {
  const [scrollY, setScrollY] = useState(0)

  useEffect(() => {
    const handleScroll = () => {
      setScrollY(window.scrollY)
    }

    window.addEventListener('scroll', handleScroll, { passive: true })
    return () => window.removeEventListener('scroll', handleScroll)
  }, [])

  const easeInOutCubic = (t: number) => t < 0.5
    ? 4 * t * t * t
    : 1 - Math.pow(-2 * t + 2, 3) / 2

  const progress = Math.min(scrollY / 800, 1)
  const easedProgress = easeInOutCubic(progress)

  const startBottom = -200
  const endBottom = -498
  const distance = startBottom - endBottom
  const translateY = easedProgress * distance

  return (
    <div
      className="absolute transition-opacity duration-200 group"
      style={{
        right: '-475px',
        bottom: `${startBottom}px`,
        transform: `translateY(${translateY}px)`,
        pointerEvents: 'none',
      }}
    >
        <div
          className="relative group-hover:opacity-10 transition-opacity duration-200"
          style={{
            padding: '430px'
          }}
        >
          <div
            className="relative inline-block"
            style={{
              textAlign: 'right',
              pointerEvents: 'auto',
            }}
          >
            <div
              className="absolute backdrop-blur-[12px]"
              style={{
                top: '-31px',
                right: '-39px',
                bottom: '-31px',
                left: '-39px',
                background: 'rgba(0, 0, 0, 0.12)',
                filter: 'blur(16px)',
                maskImage: 'radial-gradient(ellipse at center, black 0%, rgba(0,0,0,0.95) 10%, rgba(0,0,0,0.88) 18%, rgba(0,0,0,0.78) 25%, rgba(0,0,0,0.65) 32%, rgba(0,0,0,0.52) 38%, rgba(0,0,0,0.4) 44%, rgba(0,0,0,0.28) 50%, rgba(0,0,0,0.18) 56%, rgba(0,0,0,0.11) 61%, rgba(0,0,0,0.06) 66%, rgba(0,0,0,0.03) 70%, rgba(0,0,0,0.012) 74%, rgba(0,0,0,0.004) 77%, rgba(0,0,0,0.001) 79%, transparent 80%)',
                WebkitMaskImage: 'radial-gradient(ellipse at center, black 0%, rgba(0,0,0,0.95) 10%, rgba(0,0,0,0.88) 18%, rgba(0,0,0,0.78) 25%, rgba(0,0,0,0.65) 32%, rgba(0,0,0,0.52) 38%, rgba(0,0,0,0.4) 44%, rgba(0,0,0,0.28) 50%, rgba(0,0,0,0.18) 56%, rgba(0,0,0,0.11) 61%, rgba(0,0,0,0.06) 66%, rgba(0,0,0,0.03) 70%, rgba(0,0,0,0.012) 74%, rgba(0,0,0,0.004) 77%, rgba(0,0,0,0.001) 79%, transparent 80%)',
                zIndex: 1,
                pointerEvents: 'none',
              }}
            />

            <div
              className="absolute"
              style={{
                top: '-39px',
                right: '-47px',
                bottom: '-39px',
                left: '-47px',
                background: 'rgba(124, 58, 237, 0.32)',
                filter: 'blur(10px)',
                maskImage: 'radial-gradient(ellipse at center, black 0%, rgba(0,0,0,0.98) 8%, rgba(0,0,0,0.92) 16%, rgba(0,0,0,0.84) 23%, rgba(0,0,0,0.72) 30%, rgba(0,0,0,0.6) 36%, rgba(0,0,0,0.48) 42%, rgba(0,0,0,0.36) 48%, rgba(0,0,0,0.26) 54%, rgba(0,0,0,0.17) 59%, rgba(0,0,0,0.1) 64%, rgba(0,0,0,0.055) 68%, rgba(0,0,0,0.025) 72%, rgba(0,0,0,0.01) 75%, rgba(0,0,0,0.003) 77%, rgba(0,0,0,0.0008) 79%, transparent 80%)',
                WebkitMaskImage: 'radial-gradient(ellipse at center, black 0%, rgba(0,0,0,0.98) 8%, rgba(0,0,0,0.92) 16%, rgba(0,0,0,0.84) 23%, rgba(0,0,0,0.72) 30%, rgba(0,0,0,0.6) 36%, rgba(0,0,0,0.48) 42%, rgba(0,0,0,0.36) 48%, rgba(0,0,0,0.26) 54%, rgba(0,0,0,0.17) 59%, rgba(0,0,0,0.1) 64%, rgba(0,0,0,0.055) 68%, rgba(0,0,0,0.025) 72%, rgba(0,0,0,0.01) 75%, rgba(0,0,0,0.003) 77%, rgba(0,0,0,0.0008) 79%, transparent 80%)',
                zIndex: 2,
                pointerEvents: 'none',
              }}
            />

            <div
              className="relative px-6 py-4"
              style={{
                zIndex: 3,
                pointerEvents: 'none',
              }}
            >
              <h2
                className="font-serif font-bold tracking-tight text-zinc-100 transition-colors duration-700"
                style={{
                  textShadow: '0 2px 8px rgba(0, 0, 0, 0.5), 0 4px 20px rgba(124, 58, 237, 0.4)',
                  fontSize: 'clamp(2rem, 5vw, 3.75rem)',
                  whiteSpace: 'nowrap',
                  marginBottom: '0.0625rem',
                  lineHeight: '1'
                }}
              >
                Soul Player
              </h2>
              <p
                className="text-zinc-300 transition-colors duration-700"
                style={{
                  textShadow: '0 1px 6px rgba(0, 0, 0, 0.4), 0 2px 12px rgba(124, 58, 237, 0.3)',
                  fontSize: 'clamp(0.875rem, 1.5vw, 1rem)'
                }}
              >
                brought to you by Soul Audio
              </p>
            </div>
          </div>
        </div>
    </div>
  )
}
