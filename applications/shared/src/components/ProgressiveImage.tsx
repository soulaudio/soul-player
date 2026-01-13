/**
 * ProgressiveImage - Smooth blur-to-sharp image loading
 *
 * Modern progressive image loading with blur placeholder technique.
 * Based on 2025 best practices for optimal UX.
 *
 * @see https://www.mux.com/blog/blurry-image-placeholders-on-the-web
 * @see https://kentcdodds.com/blog/building-an-awesome-image-loading-experience
 */

import { useState, useEffect, useRef, ImgHTMLAttributes } from 'react'
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
  const [isLoaded, setIsLoaded] = useState(false)
  const [hasError, setHasError] = useState(false)
  const imgRef = useRef<HTMLImageElement>(null)

  useEffect(() => {
    if (!src) {
      setHasError(true)
      return
    }

    setIsLoaded(false)
    setHasError(false)

    // Create a new image to preload
    const img = new Image()

    img.onload = () => {
      setIsLoaded(true)
    }

    img.onerror = () => {
      setHasError(true)
    }

    img.src = src

    // If image is already cached, it will be loaded immediately
    if (img.complete) {
      setIsLoaded(true)
    }

    return () => {
      img.onload = null
      img.onerror = null
    }
  }, [src])

  const shapeClass = shape === 'circular' ? 'rounded-full' : 'rounded-lg'

  // Show placeholder when no src, error, or loading
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

  return (
    <div className={cn('relative overflow-hidden', shapeClass, className)}>
      <img
        ref={imgRef}
        src={src}
        alt={alt}
        className={cn(
          'w-full h-full object-cover transition-all duration-500 ease-out',
          shapeClass,
          // Blur and scale up while loading (LQIP technique)
          !isLoaded && 'blur-md scale-110 opacity-0',
          // Sharp and normal scale when loaded
          isLoaded && 'blur-0 scale-100 opacity-100'
        )}
        loading="lazy"
        {...props}
      />
    </div>
  )
}
