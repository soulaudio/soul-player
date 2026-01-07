'use client'

import { useEffect, useState } from 'react'

export function ParallaxBranding() {
  const [scrollY, setScrollY] = useState(0)
  const [currentTheme, setCurrentTheme] = useState('dark')

  useEffect(() => {
    const handleScroll = () => {
      setScrollY(window.scrollY)
    }

    window.addEventListener('scroll', handleScroll, { passive: true })
    return () => window.removeEventListener('scroll', handleScroll)
  }, [])

  useEffect(() => {
    // Detect theme changes from the demo container
    const demoContainer = document.querySelector('[data-demo-container]')
    if (demoContainer) {
      const theme = demoContainer.getAttribute('data-theme') || 'dark'
      setCurrentTheme(theme)

      // Create observer to watch for theme changes
      const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
          if (mutation.type === 'attributes' && mutation.attributeName === 'data-theme') {
            const newTheme = demoContainer.getAttribute('data-theme') || 'dark'
            setCurrentTheme(newTheme)
          }
        })
      })

      observer.observe(demoContainer, { attributes: true })
      return () => observer.disconnect()
    }
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

  // Determine logo color based on theme
  const logoColor = currentTheme === 'dark' ? 'white' : 'black'

  return (
    <div
      className="absolute transition-opacity duration-200 group"
      style={{
        right: '-495px',
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
                top: '-15px',
                right: '-20px',
                bottom: '-15px',
                left: '-20px',
                background: 'rgba(0, 0, 0, 0.12)',
                filter: 'blur(10px)',
                maskImage: 'radial-gradient(ellipse at center, black 0%, rgba(0,0,0,0.95) 10%, rgba(0,0,0,0.88) 18%, rgba(0,0,0,0.78) 25%, rgba(0,0,0,0.65) 32%, rgba(0,0,0,0.52) 38%, rgba(0,0,0,0.4) 44%, rgba(0,0,0,0.28) 50%, rgba(0,0,0,0.18) 56%, rgba(0,0,0,0.11) 61%, rgba(0,0,0,0.06) 66%, rgba(0,0,0,0.03) 70%, rgba(0,0,0,0.012) 74%, rgba(0,0,0,0.004) 77%, rgba(0,0,0,0.001) 79%, transparent 80%)',
                WebkitMaskImage: 'radial-gradient(ellipse at center, black 0%, rgba(0,0,0,0.95) 10%, rgba(0,0,0,0.88) 18%, rgba(0,0,0,0.78) 25%, rgba(0,0,0,0.65) 32%, rgba(0,0,0,0.52) 38%, rgba(0,0,0,0.4) 44%, rgba(0,0,0,0.28) 50%, rgba(0,0,0,0.18) 56%, rgba(0,0,0,0.11) 61%, rgba(0,0,0,0.06) 66%, rgba(0,0,0,0.03) 70%, rgba(0,0,0,0.012) 74%, rgba(0,0,0,0.004) 77%, rgba(0,0,0,0.001) 79%, transparent 80%)',
                zIndex: 1,
                pointerEvents: 'none',
              }}
            />

            <div
              data-branding-gradient
              className="absolute"
              style={{
                top: '-20px',
                right: '-25px',
                bottom: '-20px',
                left: '-25px',
                background: 'radial-gradient(ellipse 90% 50% at 50% 50%, hsl(var(--primary) / 0.15) 0%, hsl(var(--primary) / 0.1) 35%, hsl(var(--primary) / 0.03) 65%, transparent 100%)',
                filter: 'blur(8px)',
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
              <div
                className="flex justify-end transition-all duration-700"
                style={{
                  filter: logoColor === 'black'
                    ? 'drop-shadow(0 2px 8px rgba(0, 0, 0, 0.5)) brightness(0) saturate(100%)'
                    : 'drop-shadow(0 2px 8px rgba(0, 0, 0, 0.5))',
                }}
              >
                <img
                  src="/soul-audio-logo.svg"
                  alt="Soul Audio"
                  style={{
                    height: 'clamp(5rem, 10vw, 8rem)',
                    width: 'auto',
                    marginLeft: '-40px',
                  }}
                />
              </div>
            </div>
          </div>
        </div>
    </div>
  )
}
