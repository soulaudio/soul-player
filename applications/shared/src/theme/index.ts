/**
 * Theme system exports
 */

// Types
export type {
  Theme,
  ThemeColors,
  ThemeGradients,
  ThemeTypography,
  ThemeMetadata,
  ThemeValidationResult,
  ContrastCheckResult,
  ThemeExport,
  HSLColor,
  Gradient,
} from './types';

// Built-in themes
export { lightTheme, darkTheme, oceanTheme, builtInThemes, defaultTheme } from './themes';

// Manager
export { ThemeManager, themeManager } from './ThemeManager';

// React components
export { ThemeProvider, useTheme } from './ThemeProvider';
export { ThemeSwitcher, ThemePreview, ThemePicker } from './components';

// Validators
export {
  validateTheme,
  validateThemeStructure,
  validateThemeAccessibility,
  checkContrast,
  calculateContrastRatio,
} from './validators';

// Schema
export { themeSchema, themeExportSchema, themeMetadataSchema } from './schema';
