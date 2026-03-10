/**
 * ProgressiveImage - Smooth blur-to-sharp image loading
 *
 * Modern progressive image loading with blur placeholder technique.
 * Based on 2025 best practices for optimal UX.
 *
 * @see https://www.mux.com/blog/blurry-image-placeholders-on-the-web
 * @see https://kentcdodds.com/blog/building-an-awesome-image-loading-experience
 */

import { useState, useEffect, useRef, useMemo, ImgHTMLAttributes } from 'react'
import { cn } from '../lib/utils'

interface ProgressiveImageProps extends Omit<ImgHTMLAttributes<HTMLImageElement>, 'onLoad' | 'src'> {
  /** Main image source */
  src?: string | null
  /** Alt text for accessibility */
  alt: string
  /** Additional CSS classes */
  className?: string
  /** Shape: rounded for albums/playlists, circular for artists */
  shape?: 'rounded' | 'circular'
}

/**
 * Progressive image component with blur-up loading effect
 *
 * Implementation:
 * 1. Shows blurred, slightly scaled image while loading
 * 2. Preloads full image in background
 * 3. Smoothly transitions to sharp image when loaded
 * 4. Uses CSS transforms for performance (GPU accelerated)
 */
export function ProgressiveImage({
  src,
  alt,
  className,
  shape = 'rounded',
  ...props
}: ProgressiveImageProps) {
  // Data URLs are already in memory — synchronously treat as loaded to avoid
  // a false-loading frame that would trigger the expensive blur/scale transition.
  const isDataUrl = useMemo(() => Boolean(src?.startsWith('data:')), [src])

  const [isLoaded, setIsLoaded] = useState(isDataUrl)
  const [hasError, setHasError] = useState(false)
  const imgRef = useRef<HTMLImageElement>(null)
  const prevSrcRef = useRef(src)

  useEffect(() => {
    if (!src) {
      setHasError(true)
      return
    }

    setHasError(false)

    // Data URLs are already in memory — no async preload needed, mark as loaded.
    if (src.startsWith('data:')) {
      setIsLoaded(true)
      return
    }

    // New non-data src: reset to unloaded so the fade-in triggers.
    if (src !== prevSrcRef.current) {
      setIsLoaded(false)
    }
    prevSrcRef.current = src

    // Preload HTTP/HTTPS image before displaying to avoid layout shift.
    const img = new Image()

    img.onload = () => setIsLoaded(true)
    img.onerror = () => setHasError(true)
    img.src = src

    // Browser already cached it — mark immediately to skip the fade.
    if (img.complete) setIsLoaded(true)

    return () => {
      img.onload = null
      img.onerror = null
    }
  }, [src])

  const shapeClass = shape === 'circular' ? 'rounded-full' : 'rounded-lg'

  if (!src || hasError) {
    return (
      <div
        className={cn(
          'w-full h-full bg-muted flex items-center justify-center',
          shapeClass,
          className
        )}
      >
        <svg
          className="w-1/3 h-1/3 text-muted-foreground opacity-20"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
          />
        </svg>
      </div>
    )
  }

  // Data URLs: render immediately — no fade, no blur, no layout cost.
  // They are already decoded in RAM; the progressive effect adds only jank.
  if (isDataUrl) {
    return (
      <div className={cn('relative overflow-hidden', shapeClass, className)}>
        <img
          ref={imgRef}
          src={src}
          alt={alt}
          className={cn('w-full h-full object-cover', shapeClass)}
          {...props}
        />
      </div>
    )
  }

  return (
    <div className={cn('relative overflow-hidden', shapeClass, className)}>
      <img
        ref={imgRef}
        src={src}
        alt={alt}
        className={cn(
          // opacity-only transition — always GPU-composited, never triggers paint.
          // Avoids the filter:blur + transform:scale combo that caused scroll jank
          // when multiple images loaded simultaneously during fast scrolling.
          'w-full h-full object-cover transition-opacity duration-300 ease-out',
          shapeClass,
          !isLoaded && 'opacity-0',
          isLoaded && 'opacity-100'
        )}
        loading="lazy"
        {...props}
      />
    </div>
  )
}
