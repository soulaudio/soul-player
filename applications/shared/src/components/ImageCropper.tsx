/**
 * ImageCropper - Canvas-based image cropping component
 * Supports circle (for artists) and square (for albums/playlists) crop modes
 */

import { useRef, useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { ZoomIn, ZoomOut, RotateCcw } from 'lucide-react'

interface ImageCropperProps {
  /** Image source URL (can be data URL or blob URL) */
  imageSrc: string
  /** Crop shape - circle for artists, square for albums/playlists */
  cropShape: 'circle' | 'square'
  /** Size of the output image in pixels */
  outputSize?: number
  /** Called when crop is confirmed */
  onCrop: (croppedImageBase64: string) => void
  /** Called when cancel is clicked */
  onCancel: () => void
}

export function ImageCropper({
  imageSrc,
  cropShape,
  outputSize = 500,
  onCrop,
  onCancel,
}: ImageCropperProps) {
  const { t } = useTranslation()
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const imageRef = useRef<HTMLImageElement | null>(null)

  // Cropper state
  const [scale, setScale] = useState(1)
  const [position, setPosition] = useState({ x: 0, y: 0 })
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const [imageLoaded, setImageLoaded] = useState(false)

  // Canvas dimensions
  const canvasSize = 300
  const cropRadius = canvasSize / 2 - 20

  // Load the image
  useEffect(() => {
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => {
      imageRef.current = img
      setImageLoaded(true)

      // Calculate initial scale to fit the crop area
      const minDim = Math.min(img.width, img.height)
      const initialScale = (cropRadius * 2) / minDim
      setScale(initialScale * 1.1) // Slightly larger so user can pan

      // Center the image
      setPosition({ x: 0, y: 0 })
    }
    img.src = imageSrc
  }, [imageSrc, cropRadius])

  // Draw the canvas
  const draw = useCallback(() => {
    const canvas = canvasRef.current
    const ctx = canvas?.getContext('2d')
    const img = imageRef.current

    if (!canvas || !ctx || !img) return

    // Clear canvas
    ctx.clearRect(0, 0, canvasSize, canvasSize)

    // Save context for clipping
    ctx.save()

    // Draw the image scaled and positioned
    const imgWidth = img.width * scale
    const imgHeight = img.height * scale
    const centerX = canvasSize / 2
    const centerY = canvasSize / 2

    ctx.drawImage(
      img,
      centerX - imgWidth / 2 + position.x,
      centerY - imgHeight / 2 + position.y,
      imgWidth,
      imgHeight
    )

    ctx.restore()

    // Draw overlay with crop hole
    ctx.save()
    ctx.fillStyle = 'rgba(0, 0, 0, 0.6)'
    ctx.fillRect(0, 0, canvasSize, canvasSize)

    // Cut out the crop area
    ctx.globalCompositeOperation = 'destination-out'
    ctx.beginPath()
    if (cropShape === 'circle') {
      ctx.arc(centerX, centerY, cropRadius, 0, Math.PI * 2)
    } else {
      ctx.rect(centerX - cropRadius, centerY - cropRadius, cropRadius * 2, cropRadius * 2)
    }
    ctx.fill()
    ctx.restore()

    // Draw crop border
    ctx.save()
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)'
    ctx.lineWidth = 2
    ctx.beginPath()
    if (cropShape === 'circle') {
      ctx.arc(centerX, centerY, cropRadius, 0, Math.PI * 2)
    } else {
      ctx.rect(centerX - cropRadius, centerY - cropRadius, cropRadius * 2, cropRadius * 2)
    }
    ctx.stroke()
    ctx.restore()
  }, [scale, position, cropShape, cropRadius])

  // Redraw when state changes
  useEffect(() => {
    if (imageLoaded) {
      draw()
    }
  }, [draw, imageLoaded])

  // Calculate minimum scale to ensure image covers crop area
  const getMinScale = useCallback(() => {
    const img = imageRef.current
    if (!img) return 0.1
    const minDim = Math.min(img.width, img.height)
    return (cropRadius * 2) / minDim
  }, [cropRadius])

  // Clamp position to ensure image covers the crop area
  const clampPosition = useCallback((pos: { x: number; y: number }, currentScale: number) => {
    const img = imageRef.current
    if (!img) return pos

    const imgWidth = img.width * currentScale
    const imgHeight = img.height * currentScale

    // Calculate max allowed movement in each direction
    // The image edge must not go past the crop area edge
    const maxX = Math.max(0, (imgWidth - cropRadius * 2) / 2)
    const maxY = Math.max(0, (imgHeight - cropRadius * 2) / 2)

    return {
      x: Math.max(-maxX, Math.min(maxX, pos.x)),
      y: Math.max(-maxY, Math.min(maxY, pos.y)),
    }
  }, [cropRadius])

  // Mouse/touch handlers
  const handlePointerDown = (e: React.PointerEvent) => {
    setIsDragging(true)
    setDragStart({
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    })
    e.currentTarget.setPointerCapture(e.pointerId)
  }

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging) return
    const newPos = {
      x: e.clientX - dragStart.x,
      y: e.clientY - dragStart.y,
    }
    setPosition(clampPosition(newPos, scale))
  }

  const handlePointerUp = () => {
    setIsDragging(false)
  }

  // Max zoom: 8× the minimum zoom (relative so it stays sensible regardless of image size)
  const MAX_ZOOM_FACTOR = 8
  const getMaxScale = useCallback(() => getMinScale() * MAX_ZOOM_FACTOR, [getMinScale])

  // Zoom handlers with bounds clamping
  const updateScale = useCallback((newScale: number) => {
    const clampedScale = Math.max(getMinScale(), Math.min(getMaxScale(), newScale))
    setScale(clampedScale)
    setPosition((pos) => clampPosition(pos, clampedScale))
  }, [getMinScale, getMaxScale, clampPosition])

  const handleZoomIn = () => updateScale(scale * 1.2)
  const handleZoomOut = () => updateScale(scale / 1.2)

  const handleReset = () => {
    setScale(getMinScale() * 1.1)
    setPosition({ x: 0, y: 0 })
  }

  // Wheel zoom
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault()
    updateScale(scale * (e.deltaY > 0 ? 0.9 : 1.1))
  }

  // Logarithmic zoom slider — zoom feels exponential to humans, so log mapping
  // gives a linear "feel": slider midpoint = √(min×max) = ~2.8× minimum zoom.
  const getSliderValue = useCallback((): number => {
    const minScale = getMinScale()
    const maxScale = getMaxScale()
    const logMin = Math.log(minScale)
    const logMax = Math.log(maxScale)
    const logCurrent = Math.log(Math.max(minScale, scale))
    return Math.round(((logCurrent - logMin) / (logMax - logMin)) * 100)
  }, [scale, getMinScale, getMaxScale])

  const handleSliderInput = useCallback((value: number) => {
    const minScale = getMinScale()
    const maxScale = getMaxScale()
    const logMin = Math.log(minScale)
    const logMax = Math.log(maxScale)
    updateScale(Math.exp(logMin + (value / 100) * (logMax - logMin)))
  }, [getMinScale, getMaxScale, updateScale])

  const handleSliderWheel = (e: React.WheelEvent) => {
    e.preventDefault()
    updateScale(scale * (e.deltaY > 0 ? 0.9 : 1.1))
  }

  // Generate cropped image
  const handleCrop = () => {
    const img = imageRef.current
    if (!img) return

    // Create output canvas
    const outputCanvas = document.createElement('canvas')
    outputCanvas.width = outputSize
    outputCanvas.height = outputSize
    const ctx = outputCanvas.getContext('2d')

    if (!ctx) return

    // Calculate what portion of the image is in the crop area
    const imgWidth = img.width * scale
    const imgHeight = img.height * scale
    const centerX = canvasSize / 2
    const centerY = canvasSize / 2

    // Source coordinates in the scaled image space
    const cropLeft = centerX - cropRadius - (centerX - imgWidth / 2 + position.x)
    const cropTop = centerY - cropRadius - (centerY - imgHeight / 2 + position.y)

    // Convert to source image coordinates
    const srcX = cropLeft / scale
    const srcY = cropTop / scale
    const srcSize = (cropRadius * 2) / scale

    // Apply circular mask if needed
    if (cropShape === 'circle') {
      ctx.beginPath()
      ctx.arc(outputSize / 2, outputSize / 2, outputSize / 2, 0, Math.PI * 2)
      ctx.closePath()
      ctx.clip()
    }

    // Draw the cropped portion
    ctx.drawImage(img, srcX, srcY, srcSize, srcSize, 0, 0, outputSize, outputSize)

    // Convert to base64
    const base64 = outputCanvas.toDataURL('image/jpeg', 0.9)
    onCrop(base64)
  }

  return (
    <div className="flex flex-col items-center gap-4">
      {/* Crop title */}
      <h3 className="text-lg font-semibold">{t('artwork.edit.cropTitle')}</h3>

      {/* Canvas container */}
      <div
        ref={containerRef}
        className="relative select-none touch-none"
        style={{ width: canvasSize, height: canvasSize }}
      >
        <canvas
          ref={canvasRef}
          width={canvasSize}
          height={canvasSize}
          className="rounded-lg cursor-move bg-muted"
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerLeave={handlePointerUp}
          onWheel={handleWheel}
        />

        {/* Loading state */}
        {!imageLoaded && (
          <div className="absolute inset-0 flex items-center justify-center bg-muted rounded-lg">
            <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full" />
          </div>
        )}
      </div>

      {/* Instructions */}
      <p className="text-sm text-muted-foreground">{t('artwork.edit.dragToReposition')}</p>

      {/* Zoom controls */}
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={handleZoomOut}
          className="p-2 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
          title={t('artwork.edit.zoomOut')}
        >
          <ZoomOut className="w-5 h-5" />
        </button>

        {/* Native range input with logarithmic mapping for natural zoom feel */}
        <input
          type="range"
          min={0}
          max={100}
          step={1}
          value={getSliderValue()}
          onChange={(e) => handleSliderInput(Number(e.target.value))}
          onWheel={handleSliderWheel}
          className="w-32 accent-primary cursor-pointer"
          title={t('artwork.edit.zoomIn')}
        />

        <button
          type="button"
          onClick={handleZoomIn}
          className="p-2 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
          title={t('artwork.edit.zoomIn')}
        >
          <ZoomIn className="w-5 h-5" />
        </button>

        <button
          type="button"
          onClick={handleReset}
          className="p-2 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors ml-2"
          title={t('artwork.edit.resetZoom')}
        >
          <RotateCcw className="w-5 h-5" />
        </button>
      </div>

      {/* Action buttons */}
      <div className="flex items-center gap-3 mt-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 rounded-lg border hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
        >
          {t('common.cancel')}
        </button>
        <button
          type="button"
          onClick={handleCrop}
          disabled={!imageLoaded}
          className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]"
        >
          {t('artwork.edit.crop')}
        </button>
      </div>
    </div>
  )
}
