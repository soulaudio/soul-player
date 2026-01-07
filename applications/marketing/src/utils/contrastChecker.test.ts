import { describe, it, expect } from 'vitest'
import {
  hexToRgb,
  parseRgbString,
  getRelativeLuminance,
  getContrastRatio,
  meetsWCAG_AA,
  getContrastAssessment,
} from './contrastChecker'

describe('Contrast Checker', () => {
  describe('Color Parsing', () => {
    it('should parse hex colors correctly', () => {
      expect(hexToRgb('#ffffff')).toEqual({ r: 255, g: 255, b: 255 })
      expect(hexToRgb('#000000')).toEqual({ r: 0, g: 0, b: 0 })
      expect(hexToRgb('#7c3aed')).toEqual({ r: 124, g: 58, b: 237 })
    })

    it('should parse rgb strings correctly', () => {
      expect(parseRgbString('rgb(255, 255, 255)')).toEqual({ r: 255, g: 255, b: 255 })
      expect(parseRgbString('rgb(0, 0, 0)')).toEqual({ r: 0, g: 0, b: 0 })
      expect(parseRgbString('rgba(124, 58, 237, 0.5)')).toEqual({ r: 124, g: 58, b: 237 })
    })
  })

  describe('Luminance Calculation', () => {
    it('should calculate correct luminance for white', () => {
      const white = { r: 255, g: 255, b: 255 }
      expect(getRelativeLuminance(white)).toBeCloseTo(1, 2)
    })

    it('should calculate correct luminance for black', () => {
      const black = { r: 0, g: 0, b: 0 }
      expect(getRelativeLuminance(black)).toBe(0)
    })
  })

  describe('Contrast Ratio', () => {
    it('should calculate 21:1 for white on black', () => {
      const white = { r: 255, g: 255, b: 255 }
      const black = { r: 0, g: 0, b: 0 }
      expect(getContrastRatio(white, black)).toBeCloseTo(21, 1)
    })

    it('should calculate 1:1 for same colors', () => {
      const color = { r: 128, g: 128, b: 128 }
      expect(getContrastRatio(color, color)).toBe(1)
    })
  })

  describe('WCAG AA Compliance', () => {
    it('should pass AA for 4.5:1 ratio on normal text', () => {
      expect(meetsWCAG_AA(4.5, false)).toBe(true)
      expect(meetsWCAG_AA(4.4, false)).toBe(false)
    })

    it('should pass AA for 3:1 ratio on large text', () => {
      expect(meetsWCAG_AA(3.0, true)).toBe(true)
      expect(meetsWCAG_AA(2.9, true)).toBe(false)
    })
  })

  describe('Theme Color Contrast - Dark Theme', () => {
    const darkBg = { r: 0, g: 0, b: 0 } // rgb(0, 0, 0)

    it('Soul Player text (zinc-100) should have excellent contrast on dark background', () => {
      const soulPlayerText = { r: 244, g: 244, b: 245 } // zinc-100
      const ratio = getContrastRatio(soulPlayerText, darkBg)
      const assessment = getContrastAssessment(ratio, true) // Large text

      expect(assessment.passAA).toBe(true)
      expect(assessment.passAAA).toBe(true)
      expect(ratio).toBeGreaterThan(15) // Excellent contrast
    })

    it('Main text (zinc-300) should meet AA standards on dark background', () => {
      const mainText = { r: 212, g: 212, b: 216 } // zinc-300
      const ratio = getContrastRatio(mainText, darkBg)
      const assessment = getContrastAssessment(ratio, false)

      expect(assessment.passAA).toBe(true)
      expect(ratio).toBeGreaterThan(4.5)
    })

    it('Description text (zinc-400) should meet AA standards', () => {
      const descText = { r: 161, g: 161, b: 170 } // zinc-400
      const ratio = getContrastRatio(descText, darkBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
      expect(ratio).toBeGreaterThan(4.5)
    })

    it('Theme label (zinc-400) should meet AA standards for small text', () => {
      const themeLabel = { r: 161, g: 161, b: 170 } // zinc-400
      const ratio = getContrastRatio(themeLabel, darkBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
    })
  })

  describe('Theme Color Contrast - Light Theme', () => {
    const lightBg = { r: 250, g: 249, b: 255 } // rgb(250, 249, 255)

    it('Soul Player text (zinc-900) should have excellent contrast on light background', () => {
      const soulPlayerText = { r: 24, g: 24, b: 27 } // zinc-900
      const ratio = getContrastRatio(soulPlayerText, lightBg)
      const assessment = getContrastAssessment(ratio, true) // Large text

      expect(assessment.passAA).toBe(true)
      expect(assessment.passAAA).toBe(true)
      expect(ratio).toBeGreaterThan(15) // Excellent contrast
    })

    it('Main text (zinc-800) should meet AA standards on light background', () => {
      const mainText = { r: 39, g: 39, b: 42 } // zinc-800
      const ratio = getContrastRatio(mainText, lightBg)
      const assessment = getContrastAssessment(ratio, false)

      expect(assessment.passAA).toBe(true)
      expect(ratio).toBeGreaterThan(4.5)
    })

    it('Description text (zinc-600) should meet AA standards', () => {
      const descText = { r: 82, g: 82, b: 91 } // zinc-600
      const ratio = getContrastRatio(descText, lightBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
      expect(ratio).toBeGreaterThan(4.5)
    })

    it('Theme label (zinc-600) should meet AA standards', () => {
      const themeLabel = { r: 82, g: 82, b: 91 } // zinc-600
      const ratio = getContrastRatio(themeLabel, lightBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
    })

    it('Soul Audio subtitle (zinc-700) should meet AA standards', () => {
      const subtitle = { r: 63, g: 63, b: 70 } // zinc-700
      const ratio = getContrastRatio(subtitle, lightBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
    })
  })

  describe('Theme Color Contrast - Ocean Theme', () => {
    const oceanBg = { r: 8, g: 47, b: 73 } // rgb(8, 47, 73)

    it('Soul Player text (slate-100) should have good contrast on ocean background', () => {
      const soulPlayerText = { r: 241, g: 245, b: 249 } // slate-100
      const ratio = getContrastRatio(soulPlayerText, oceanBg)
      const assessment = getContrastAssessment(ratio, true) // Large text

      expect(assessment.passAA).toBe(true)
      expect(ratio).toBeGreaterThan(10) // Very good contrast
    })

    it('Main text (slate-100) should meet AA standards', () => {
      const mainText = { r: 241, g: 245, b: 249 } // slate-100
      const ratio = getContrastRatio(mainText, oceanBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
      expect(ratio).toBeGreaterThan(4.5)
    })

    it('Description text (slate-200) should meet AA standards', () => {
      const descText = { r: 226, g: 232, b: 240 } // slate-200
      const ratio = getContrastRatio(descText, oceanBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
    })

    it('Theme label (slate-300) should meet AA standards', () => {
      const themeLabel = { r: 203, g: 213, b: 225 } // slate-300
      const ratio = getContrastRatio(themeLabel, oceanBg)

      expect(meetsWCAG_AA(ratio, false)).toBe(true)
    })
  })
})
