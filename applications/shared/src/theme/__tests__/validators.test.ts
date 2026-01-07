/**
 * Comprehensive tests for theme validators
 * Focus: WCAG contrast calculations, accessibility checks, theme structure validation
 */

import { describe, it, expect } from 'vitest';
import {
  calculateContrastRatio,
  checkContrast,
  validateThemeStructure,
  validateThemeAccessibility,
  validateTheme,
} from '../validators';
import { lightTheme, darkTheme, oceanTheme } from '../themes';
import type { Theme } from '../types';

describe('calculateContrastRatio', () => {
  describe('WCAG standard test cases', () => {
    it('should calculate correct contrast for black text on white background', () => {
      // Pure white background (0 0% 100%) vs pure black text (0 0% 0%)
      // Expected ratio: 21:1 (maximum possible)
      const ratio = calculateContrastRatio('0 0% 0%', '0 0% 100%');
      expect(ratio).toBeCloseTo(21, 1);
    });

    it('should calculate correct contrast for white text on black background', () => {
      // Order shouldn't matter for contrast calculation
      const ratio = calculateContrastRatio('0 0% 100%', '0 0% 0%');
      expect(ratio).toBeCloseTo(21, 1);
    });

    it('should return 1:1 for identical colors', () => {
      const ratio = calculateContrastRatio('200 50% 50%', '200 50% 50%');
      expect(ratio).toBeCloseTo(1, 1);
    });

    it('should handle gray scale colors correctly', () => {
      // Medium gray on white should have specific ratio
      const ratio = calculateContrastRatio('0 0% 50%', '0 0% 100%');
      expect(ratio).toBeGreaterThan(1);
      expect(ratio).toBeLessThan(21);
    });
  });

  describe('real-world color combinations', () => {
    it('should calculate ratio for light theme foreground/background', () => {
      const ratio = calculateContrastRatio(
        lightTheme.colors.foreground,
        lightTheme.colors.background
      );
      // Dark text on white should have good contrast
      expect(ratio).toBeGreaterThan(7); // Should pass AAA
    });

    it('should calculate ratio for dark theme foreground/background', () => {
      const ratio = calculateContrastRatio(
        darkTheme.colors.foreground,
        darkTheme.colors.background
      );
      // Light text on dark should have good contrast
      expect(ratio).toBeGreaterThan(7); // Should pass AAA
    });

    it('should calculate ratio for ocean theme primary button', () => {
      const ratio = calculateContrastRatio(
        oceanTheme.colors['primary-foreground'],
        oceanTheme.colors.primary
      );
      // Should at least pass AA
      expect(ratio).toBeGreaterThanOrEqual(4.5);
    });
  });

  describe('edge cases', () => {
    it('should handle HSL values without percentage signs', () => {
      // Some implementations might accept "210 100 50" instead of "210 100% 50%"
      const ratio1 = calculateContrastRatio('210 100% 50%', '0 0% 100%');
      const ratio2 = calculateContrastRatio('210 100 50', '0 0 100');
      expect(ratio1).toBeCloseTo(ratio2, 1);
    });

    it('should handle extreme lightness values', () => {
      const ratio1 = calculateContrastRatio('0 0% 0%', '0 0% 100%'); // Min vs max
      const ratio2 = calculateContrastRatio('0 0% 1%', '0 0% 99%');  // Near min/max

      expect(ratio1).toBeGreaterThan(ratio2);
      expect(ratio1).toBeCloseTo(21, 1);
    });

    it('should handle saturated colors correctly', () => {
      // Fully saturated blue on white
      const ratio = calculateContrastRatio('240 100% 50%', '0 0% 100%');
      expect(ratio).toBeGreaterThan(1);
      expect(ratio).toBeLessThan(21);
    });
  });

  describe('hue variations', () => {
    it('should calculate different ratios for different hues at same lightness', () => {
      const red = calculateContrastRatio('0 100% 50%', '0 0% 100%');
      const green = calculateContrastRatio('120 100% 50%', '0 0% 100%');
      const blue = calculateContrastRatio('240 100% 50%', '0 0% 100%');

      // Green should have different luminance than red and blue
      // due to how human eye perceives colors
      expect(green).not.toBeCloseTo(red, 0);
      expect(green).not.toBeCloseTo(blue, 0);
    });
  });
});

describe('checkContrast', () => {
  describe('WCAG AA compliance (4.5:1)', () => {
    it('should pass AA for high contrast combinations', () => {
      const result = checkContrast('222.2 84% 4.9%', '0 0% 100%');
      expect(result.passes.aa).toBe(true);
      expect(result.ratio).toBeGreaterThanOrEqual(4.5);
    });

    it('should fail AA for low contrast combinations', () => {
      // Light gray on white - should fail
      const result = checkContrast('0 0% 80%', '0 0% 100%');
      expect(result.passes.aa).toBe(false);
      expect(result.ratio).toBeLessThan(4.5);
    });

    it('should include exact ratio in result', () => {
      const result = checkContrast('0 0% 0%', '0 0% 100%');
      expect(result.ratio).toBeDefined();
      expect(typeof result.ratio).toBe('number');
    });

    it('should include color values in result', () => {
      const fg = '222.2 84% 4.9%';
      const bg = '0 0% 100%';
      const result = checkContrast(fg, bg);

      expect(result.textColor).toBe(fg);
      expect(result.backgroundColor).toBe(bg);
    });
  });

  describe('WCAG AAA compliance (7:1)', () => {
    it('should pass AAA for very high contrast combinations', () => {
      const result = checkContrast('0 0% 0%', '0 0% 100%');
      expect(result.passes.aaa).toBe(true);
      expect(result.ratio).toBeGreaterThanOrEqual(7);
    });

    it('should fail AAA but pass AA for medium contrast', () => {
      // Test color that passes AA but not AAA
      const result = checkContrast('0 0% 40%', '0 0% 100%');

      expect(result.ratio).toBeGreaterThanOrEqual(4.5);
      expect(result.ratio).toBeLessThan(7);
      expect(result.passes.aa).toBe(true);
      expect(result.passes.aaa).toBe(false);
    });
  });

  describe('built-in theme validation', () => {
    it('should validate light theme main text passes AA', () => {
      const result = checkContrast(
        lightTheme.colors.foreground,
        lightTheme.colors.background
      );
      expect(result.passes.aa).toBe(true);
    });

    it('should validate dark theme main text passes AA', () => {
      const result = checkContrast(
        darkTheme.colors.foreground,
        darkTheme.colors.background
      );
      expect(result.passes.aa).toBe(true);
    });

    it('should validate ocean theme main text passes AA', () => {
      const result = checkContrast(
        oceanTheme.colors.foreground,
        oceanTheme.colors.background
      );
      expect(result.passes.aa).toBe(true);
    });
  });
});

describe('validateThemeStructure', () => {
  describe('valid theme validation', () => {
    it('should validate complete built-in light theme', () => {
      const result = validateThemeStructure(lightTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate complete built-in dark theme', () => {
      const result = validateThemeStructure(darkTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate complete built-in ocean theme', () => {
      const result = validateThemeStructure(oceanTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate minimal valid theme', () => {
      const minimalTheme: Theme = {
        id: 'minimal',
        name: 'Minimal',
        version: '1.0.0',
        colors: {
          background: '0 0% 100%',
          foreground: '0 0% 0%',
          card: '0 0% 100%',
          'card-foreground': '0 0% 0%',
          popover: '0 0% 100%',
          'popover-foreground': '0 0% 0%',
          primary: '200 100% 50%',
          'primary-foreground': '0 0% 100%',
          secondary: '200 100% 50%',
          'secondary-foreground': '0 0% 100%',
          muted: '0 0% 90%',
          'muted-foreground': '0 0% 40%',
          accent: '200 100% 50%',
          'accent-foreground': '0 0% 100%',
          destructive: '0 100% 50%',
          'destructive-foreground': '0 0% 100%',
          border: '0 0% 90%',
          input: '0 0% 90%',
          ring: '200 100% 50%',
        },
      };

      const result = validateThemeStructure(minimalTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('invalid theme ID', () => {
    it('should reject theme with uppercase ID', () => {
      const invalidTheme = {
        ...lightTheme,
        id: 'MyTheme', // Invalid: uppercase
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should reject theme with spaces in ID', () => {
      const invalidTheme = {
        ...lightTheme,
        id: 'my theme', // Invalid: spaces
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });

    it('should reject theme with special characters in ID', () => {
      const invalidTheme = {
        ...lightTheme,
        id: 'my_theme!', // Invalid: underscore and exclamation
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });

    it('should accept theme with hyphens in ID', () => {
      const validTheme = {
        ...lightTheme,
        id: 'my-theme-123', // Valid
      };

      const result = validateThemeStructure(validTheme);
      expect(result.valid).toBe(true);
    });
  });

  describe('invalid version numbers', () => {
    it('should reject non-semver version', () => {
      const invalidTheme = {
        ...lightTheme,
        version: '1.0', // Invalid: not semver
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });

    it('should accept semver with prerelease', () => {
      const validTheme = {
        ...lightTheme,
        version: '1.0.0-beta.1',
      };

      const result = validateThemeStructure(validTheme);
      expect(result.valid).toBe(true);
    });

    it('should accept semver with build metadata', () => {
      const validTheme = {
        ...lightTheme,
        version: '1.0.0+20250105',
      };

      const result = validateThemeStructure(validTheme);
      expect(result.valid).toBe(true);
    });
  });

  describe('missing required color tokens', () => {
    it('should reject theme missing background color', () => {
      const invalidTheme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          background: undefined as any,
        },
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });

    it('should reject theme missing primary color', () => {
      const { primary, ...colorsWithoutPrimary } = lightTheme.colors;
      void primary; // Explicitly void unused variable
      const invalidTheme = {
        ...lightTheme,
        colors: colorsWithoutPrimary as any,
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });
  });

  describe('invalid color formats', () => {
    it('should reject invalid HSL color format', () => {
      const invalidTheme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          background: 'hsl(210, 100%, 50%)', // Invalid: includes hsl() wrapper
        },
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });

    it('should reject RGB color instead of HSL', () => {
      const invalidTheme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          background: 'rgb(255, 255, 255)', // Invalid: RGB not HSL
        },
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });

    it('should reject hex color', () => {
      const invalidTheme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          background: '#ffffff', // Invalid: hex not HSL
        },
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });
  });

  describe('gradient validation', () => {
    it('should accept valid linear gradient', () => {
      const validTheme = {
        ...lightTheme,
        gradients: {
          hero: 'linear-gradient(135deg, hsl(200 90% 50%), hsl(180 85% 55%))',
        },
      };

      const result = validateThemeStructure(validTheme);
      expect(result.valid).toBe(true);
    });

    it('should accept valid radial gradient', () => {
      const validTheme = {
        ...lightTheme,
        gradients: {
          hero: 'radial-gradient(circle, hsl(200 90% 50%), hsl(180 85% 55%))',
        },
      };

      const result = validateThemeStructure(validTheme);
      expect(result.valid).toBe(true);
    });

    it('should reject invalid gradient syntax', () => {
      const invalidTheme = {
        ...lightTheme,
        gradients: {
          hero: 'gradient(blue, red)', // Invalid: not a valid CSS gradient
        },
      };

      const result = validateThemeStructure(invalidTheme);
      expect(result.valid).toBe(false);
    });
  });
});

describe('validateThemeAccessibility', () => {
  describe('built-in theme accessibility', () => {
    it('should validate light theme has accessible contrast ratios', () => {
      const result = validateThemeAccessibility(lightTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate dark theme has accessible contrast ratios', () => {
      const result = validateThemeAccessibility(darkTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate ocean theme has accessible contrast ratios', () => {
      const result = validateThemeAccessibility(oceanTheme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('inaccessible themes', () => {
    it('should reject theme with poor main text contrast', () => {
      const inaccessibleTheme: Theme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          foreground: '0 0% 90%', // Light gray on white - poor contrast
          background: '0 0% 100%',
        },
      };

      const result = validateThemeAccessibility(inaccessibleTheme);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors[0]).toContain('Main text on background');
    });

    it('should reject theme with poor primary button contrast', () => {
      const inaccessibleTheme: Theme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          primary: '200 100% 70%', // Light blue
          'primary-foreground': '200 100% 80%', // Even lighter blue - poor contrast
        },
      };

      const result = validateThemeAccessibility(inaccessibleTheme);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('Primary button'))).toBe(true);
    });
  });

  describe('warning generation', () => {
    it('should generate warnings for themes that pass AA but not AAA', () => {
      // Create a theme that passes AA (4.5:1) but not AAA (7:1)
      const marginalTheme: Theme = {
        ...lightTheme,
        colors: {
          ...lightTheme.colors,
          foreground: '0 0% 40%', // Medium gray that should pass AA but not AAA
          background: '0 0% 100%',
        },
      };

      const result = validateThemeAccessibility(marginalTheme);

      // Should still be valid (AA is the requirement)
      expect(result.valid).toBe(true);

      // But should have warnings about not meeting AAA
      expect(result.warnings.length).toBeGreaterThan(0);
    });
  });
});

describe('validateTheme (combined validation)', () => {
  it('should validate complete valid theme', () => {
    const result = validateTheme(lightTheme);
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it('should catch both structure and accessibility errors', () => {
    const invalidTheme = {
      ...lightTheme,
      id: 'Invalid ID!', // Structure error
      colors: {
        ...lightTheme.colors,
        foreground: '0 0% 95%', // Accessibility error
      },
    };

    const result = validateTheme(invalidTheme);
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it('should return structure errors if structure is invalid', () => {
    const invalidTheme = {
      id: 'test',
      name: 'Test',
      version: 'invalid', // Invalid version
      colors: {}, // Missing all colors
    };

    const result = validateTheme(invalidTheme);
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it('should aggregate all validation issues', () => {
    const problematicTheme = {
      ...lightTheme,
      id: 'Bad_ID!',
      version: '1.0',
      colors: {
        ...lightTheme.colors,
        foreground: '0 0% 95%',
        background: '0 0% 100%',
      },
    };

    const result = validateTheme(problematicTheme);
    expect(result.valid).toBe(false);
    // Should have multiple errors
    expect(result.errors.length).toBeGreaterThan(1);
  });
});
