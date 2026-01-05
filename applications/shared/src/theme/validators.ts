/**
 * Accessibility validators for themes
 * Includes WCAG 2.1 contrast ratio checking
 */

import type { Theme, ThemeValidationResult, ContrastCheckResult } from './types';
import { themeSchema } from './schema';

/**
 * Parse HSL color string to numeric values
 * Input: "210 100% 50%" or "210 100 50"
 * Output: { h: 210, s: 100, l: 50 }
 */
function parseHSL(hsl: string): { h: number; s: number; l: number } {
  const match = hsl.match(/(\d+)\s+(\d+)%?\s+(\d+)%?/);
  if (!match) {
    throw new Error(`Invalid HSL color: ${hsl}`);
  }
  return {
    h: parseInt(match[1], 10),
    s: parseInt(match[2], 10),
    l: parseInt(match[3], 10),
  };
}

/**
 * Convert HSL to RGB
 * Returns RGB values in range 0-255
 */
function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  s /= 100;
  l /= 100;

  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;

  let r = 0,
    g = 0,
    b = 0;

  if (h >= 0 && h < 60) {
    r = c;
    g = x;
    b = 0;
  } else if (h >= 60 && h < 120) {
    r = x;
    g = c;
    b = 0;
  } else if (h >= 120 && h < 180) {
    r = 0;
    g = c;
    b = x;
  } else if (h >= 180 && h < 240) {
    r = 0;
    g = x;
    b = c;
  } else if (h >= 240 && h < 300) {
    r = x;
    g = 0;
    b = c;
  } else if (h >= 300 && h < 360) {
    r = c;
    g = 0;
    b = x;
  }

  return [
    Math.round((r + m) * 255),
    Math.round((g + m) * 255),
    Math.round((b + m) * 255),
  ];
}

/**
 * Calculate relative luminance for a color (WCAG formula)
 * Input: RGB values 0-255
 * Output: Relative luminance 0-1
 */
function getRelativeLuminance(r: number, g: number, b: number): number {
  const [rs, gs, bs] = [r, g, b].map((val) => {
    const sRGB = val / 255;
    return sRGB <= 0.03928 ? sRGB / 12.92 : Math.pow((sRGB + 0.055) / 1.055, 2.4);
  });

  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

/**
 * Calculate contrast ratio between two colors (WCAG formula)
 * Returns ratio in range 1-21
 */
export function calculateContrastRatio(color1: string, color2: string): number {
  const hsl1 = parseHSL(color1);
  const hsl2 = parseHSL(color2);

  const rgb1 = hslToRgb(hsl1.h, hsl1.s, hsl1.l);
  const rgb2 = hslToRgb(hsl2.h, hsl2.s, hsl2.l);

  const l1 = getRelativeLuminance(rgb1[0], rgb1[1], rgb1[2]);
  const l2 = getRelativeLuminance(rgb2[0], rgb2[1], rgb2[2]);

  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);

  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Check contrast ratio against WCAG standards
 */
export function checkContrast(
  textColor: string,
  backgroundColor: string
): ContrastCheckResult {
  const ratio = calculateContrastRatio(textColor, backgroundColor);

  return {
    ratio,
    passes: {
      aa: ratio >= 4.5, // WCAG AA for normal text
      aaa: ratio >= 7, // WCAG AAA for normal text
    },
    textColor,
    backgroundColor,
  };
}

/**
 * Validate theme structure using Zod schema
 */
export function validateThemeStructure(theme: unknown): ThemeValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  try {
    themeSchema.parse(theme);
  } catch (error) {
    if (error instanceof Error) {
      errors.push(error.message);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Validate theme accessibility (contrast ratios)
 */
export function validateThemeAccessibility(theme: Theme): ThemeValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Critical contrast checks (must pass AA)
  const criticalPairs: Array<[string, string, string]> = [
    ['foreground', 'background', 'Main text on background'],
    ['primary-foreground', 'primary', 'Primary button text'],
    ['secondary-foreground', 'secondary', 'Secondary button text'],
    ['card-foreground', 'card', 'Card text'],
    ['popover-foreground', 'popover', 'Popover text'],
    ['destructive-foreground', 'destructive', 'Destructive button text'],
    ['accent-foreground', 'accent', 'Accent text'],
    ['muted-foreground', 'muted', 'Muted text'],
  ];

  for (const [fgKey, bgKey, description] of criticalPairs) {
    const fg = theme.colors[fgKey as keyof typeof theme.colors];
    const bg = theme.colors[bgKey as keyof typeof theme.colors];

    if (fg && bg) {
      const result = checkContrast(fg, bg);

      if (!result.passes.aa) {
        errors.push(
          `${description}: Contrast ratio ${result.ratio.toFixed(2)}:1 fails WCAG AA (requires 4.5:1)`
        );
      } else if (!result.passes.aaa) {
        warnings.push(
          `${description}: Contrast ratio ${result.ratio.toFixed(2)}:1 passes AA but not AAA (7:1)`
        );
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Validate complete theme (structure + accessibility)
 */
export function validateTheme(theme: unknown): ThemeValidationResult {
  const structureResult = validateThemeStructure(theme);

  if (!structureResult.valid) {
    return structureResult;
  }

  const accessibilityResult = validateThemeAccessibility(theme as Theme);

  return {
    valid: structureResult.valid && accessibilityResult.valid,
    errors: [...structureResult.errors, ...accessibilityResult.errors],
    warnings: [...structureResult.warnings, ...accessibilityResult.warnings],
  };
}
