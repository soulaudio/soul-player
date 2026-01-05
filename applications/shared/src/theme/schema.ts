/**
 * Zod validation schemas for theme structure
 */

import { z } from 'zod';

/**
 * HSL color format validation
 * Matches: "210 100% 50%" or "210 100 50" (with or without % signs)
 */
const hslColorSchema = z
  .string()
  .regex(
    /^\d{1,3}\s+\d{1,3}%?\s+\d{1,3}%?$/,
    'HSL color must be in format: "hue saturation% lightness%" (e.g., "210 100% 50%")'
  );

/**
 * CSS gradient validation (basic check for gradient syntax)
 */
const gradientSchema = z
  .string()
  .regex(
    /(linear-gradient|radial-gradient|conic-gradient)\s*\(/,
    'Gradient must be a valid CSS gradient (linear-gradient, radial-gradient, or conic-gradient)'
  );

/**
 * Color palette schema - all required color tokens
 */
const themeColorsSchema = z.object({
  background: hslColorSchema,
  foreground: hslColorSchema,
  card: hslColorSchema,
  'card-foreground': hslColorSchema,
  popover: hslColorSchema,
  'popover-foreground': hslColorSchema,
  primary: hslColorSchema,
  'primary-foreground': hslColorSchema,
  secondary: hslColorSchema,
  'secondary-foreground': hslColorSchema,
  muted: hslColorSchema,
  'muted-foreground': hslColorSchema,
  accent: hslColorSchema,
  'accent-foreground': hslColorSchema,
  destructive: hslColorSchema,
  'destructive-foreground': hslColorSchema,
  border: hslColorSchema,
  input: hslColorSchema,
  ring: hslColorSchema,
});

/**
 * Optional gradient definitions
 */
const themeGradientsSchema = z
  .object({
    hero: gradientSchema.optional(),
    player: gradientSchema.optional(),
    sidebar: gradientSchema.optional(),
    waveform: gradientSchema.optional(),
  })
  .optional();

/**
 * Typography schema
 */
const themeTypographySchema = z
  .object({
    fontFamily: z.object({
      sans: z.array(z.string()).min(1, 'Sans font family must have at least one font'),
      mono: z.array(z.string()).min(1, 'Mono font family must have at least one font'),
    }),
    fontSize: z
      .object({
        base: z.string().optional(),
      })
      .optional(),
  })
  .optional();

/**
 * Semver version validation
 */
const semverSchema = z
  .string()
  .regex(
    /^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$/,
    'Version must be valid semver (e.g., "1.0.0")'
  );

/**
 * Theme ID validation (lowercase, alphanumeric with hyphens)
 */
const themeIdSchema = z
  .string()
  .min(1)
  .regex(
    /^[a-z0-9-]+$/,
    'Theme ID must be lowercase alphanumeric with hyphens only'
  );

/**
 * Complete theme schema
 */
export const themeSchema = z.object({
  id: themeIdSchema,
  name: z.string().min(1, 'Theme name is required'),
  version: semverSchema,
  author: z.string().optional(),
  description: z.string().optional(),
  colors: themeColorsSchema,
  gradients: themeGradientsSchema,
  typography: themeTypographySchema,
  isBuiltIn: z.boolean().optional(),
});

/**
 * Theme export schema (includes export metadata)
 */
export const themeExportSchema = themeSchema.extend({
  exportedAt: z.string().datetime(),
  exportedBy: z.string().optional(),
});

/**
 * Theme metadata schema (lightweight)
 */
export const themeMetadataSchema = z.object({
  id: themeIdSchema,
  name: z.string().min(1),
  version: semverSchema,
  author: z.string().optional(),
  description: z.string().optional(),
  isBuiltIn: z.boolean().optional(),
});

/**
 * Type exports inferred from schemas
 */
export type ThemeSchemaType = z.infer<typeof themeSchema>;
export type ThemeExportSchemaType = z.infer<typeof themeExportSchema>;
export type ThemeMetadataSchemaType = z.infer<typeof themeMetadataSchema>;
