/**
 * WCAG 2.1 Contrast Checker
 * Calculates relative luminance and contrast ratio between two colors
 */

interface RGB {
  r: number
  g: number
  b: number
}

/**
 * Convert hex color to RGB
 */
export function hexToRgb(hex: string): RGB | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : null
}

/**
 * Parse rgb/rgba string to RGB
 */
export function parseRgbString(rgb: string): RGB | null {
  const match = rgb.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/)
  if (!match) return null
  return {
    r: parseInt(match[1]),
    g: parseInt(match[2]),
    b: parseInt(match[3]),
  }
}

/**
 * Calculate relative luminance for a color channel
 * Formula from WCAG 2.1: https://www.w3.org/TR/WCAG21/#dfn-relative-luminance
 */
function getLuminanceChannel(channel: number): number {
  const sRGB = channel / 255
  return sRGB <= 0.03928 ? sRGB / 12.92 : Math.pow((sRGB + 0.055) / 1.055, 2.4)
}

/**
 * Calculate relative luminance of an RGB color
 */
export function getRelativeLuminance(rgb: RGB): number {
  const r = getLuminanceChannel(rgb.r)
  const g = getLuminanceChannel(rgb.g)
  const b = getLuminanceChannel(rgb.b)
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/**
 * Calculate contrast ratio between two colors
 * Formula from WCAG 2.1: (L1 + 0.05) / (L2 + 0.05)
 * where L1 is the lighter color and L2 is the darker color
 */
export function getContrastRatio(color1: RGB, color2: RGB): number {
  const lum1 = getRelativeLuminance(color1)
  const lum2 = getRelativeLuminance(color2)
  const lighter = Math.max(lum1, lum2)
  const darker = Math.min(lum1, lum2)
  return (lighter + 0.05) / (darker + 0.05)
}

/**
 * Check if contrast ratio meets WCAG AA standards
 * @param ratio - Contrast ratio
 * @param isLargeText - Whether text is large (18pt/14pt bold or larger)
 * @returns true if contrast passes WCAG AA
 */
export function meetsWCAG_AA(ratio: number, isLargeText: boolean = false): boolean {
  return isLargeText ? ratio >= 3.0 : ratio >= 4.5
}

/**
 * Check if contrast ratio meets WCAG AAA standards
 * @param ratio - Contrast ratio
 * @param isLargeText - Whether text is large (18pt/14pt bold or larger)
 * @returns true if contrast passes WCAG AAA
 */
export function meetsWCAG_AAA(ratio: number, isLargeText: boolean = false): boolean {
  return isLargeText ? ratio >= 4.5 : ratio >= 7.0
}

/**
 * Get a human-readable assessment of contrast ratio
 */
export function getContrastAssessment(ratio: number, isLargeText: boolean = false): {
  ratio: number
  passAA: boolean
  passAAA: boolean
  grade: 'Excellent' | 'Good' | 'Acceptable' | 'Poor' | 'Fail'
} {
  const passAA = meetsWCAG_AA(ratio, isLargeText)
  const passAAA = meetsWCAG_AAA(ratio, isLargeText)

  let grade: 'Excellent' | 'Good' | 'Acceptable' | 'Poor' | 'Fail'
  if (passAAA) {
    grade = 'Excellent'
  } else if (passAA && ratio >= 6) {
    grade = 'Good'
  } else if (passAA) {
    grade = 'Acceptable'
  } else if (ratio >= 3.0) {
    grade = 'Poor'
  } else {
    grade = 'Fail'
  }

  return {
    ratio: Math.round(ratio * 100) / 100,
    passAA,
    passAAA,
    grade,
  }
}
