/**
 * Core theme type definitions for Soul Player
 * Supports colors (HSL format), gradients, and typography
 */

/**
 * HSL color value without the hsl() wrapper
 * Format: "hue saturation% lightness%" (e.g., "210 100% 50%")
 */
export type HSLColor = string;

/**
 * CSS gradient definition
 * Format: Full CSS gradient syntax (e.g., "linear-gradient(135deg, ...)")
 */
export type Gradient = string;

/**
 * Core color tokens used throughout the application
 */
export interface ThemeColors {
  // Base colors
  background: HSLColor;
  foreground: HSLColor;

  // Component colors
  card: HSLColor;
  'card-foreground': HSLColor;
  popover: HSLColor;
  'popover-foreground': HSLColor;

  // Semantic colors
  primary: HSLColor;
  'primary-foreground': HSLColor;
  secondary: HSLColor;
  'secondary-foreground': HSLColor;
  muted: HSLColor;
  'muted-foreground': HSLColor;
  accent: HSLColor;
  'accent-foreground': HSLColor;
  destructive: HSLColor;
  'destructive-foreground': HSLColor;

  // UI elements
  border: HSLColor;
  input: HSLColor;
  ring: HSLColor;
}

/**
 * Gradient definitions for various UI elements
 */
export interface ThemeGradients {
  hero?: Gradient;
  player?: Gradient;
  sidebar?: Gradient;
  waveform?: Gradient;
}

/**
 * Typography configuration
 */
export interface ThemeTypography {
  fontFamily: {
    sans: string[];
    mono: string[];
  };
  fontSize?: {
    base?: string;
  };
}

/**
 * Complete theme definition
 */
export interface Theme {
  /** Unique identifier for the theme */
  id: string;

  /** Human-readable name */
  name: string;

  /** Theme version (semver) */
  version: string;

  /** Theme author (optional) */
  author?: string;

  /** Theme description (optional) */
  description?: string;

  /** Color palette */
  colors: ThemeColors;

  /** Gradient definitions */
  gradients?: ThemeGradients;

  /** Typography settings */
  typography?: ThemeTypography;

  /** Whether this is a built-in theme (cannot be deleted) */
  isBuiltIn?: boolean;
}

/**
 * Theme metadata (lightweight version for listings)
 */
export interface ThemeMetadata {
  id: string;
  name: string;
  version: string;
  author?: string;
  description?: string;
  isBuiltIn?: boolean;
}

/**
 * Theme validation result
 */
export interface ThemeValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

/**
 * Contrast check result for accessibility
 */
export interface ContrastCheckResult {
  ratio: number;
  passes: {
    aa: boolean;
    aaa: boolean;
  };
  textColor: string;
  backgroundColor: string;
}

/**
 * Theme export format (includes metadata for sharing)
 */
export interface ThemeExport extends Theme {
  exportedAt: string;
  exportedBy?: string;
}
