# Accessibility - WCAG 2.1 Compliance

This document details the accessibility standards and testing procedures for the Soul Player marketing site.

## WCAG 2.1 AA Compliance

All color combinations in the marketing site meet **WCAG 2.1 Level AA** standards for contrast ratios.

### Contrast Requirements

- **Normal text**: Minimum 4.5:1 contrast ratio
- **Large text** (18pt+/14pt bold+): Minimum 3.0:1 contrast ratio
- **AAA standard** (aspirational): 7.0:1 for normal text, 4.5:1 for large text

## Theme Color Palettes

### Dark Theme
- **Background**: `rgb(0, 0, 0)` - Pure black
- **Soul Player heading**: `rgb(244, 244, 245)` - zinc-100 (19.11:1 contrast)
- **Main text**: `rgb(212, 212, 216)` - zinc-300 (14.21:1 contrast)
- **Description/labels**: `rgb(161, 161, 170)` - zinc-400 (8.19:1 contrast)
- **Badges**: `rgb(161, 161, 170)` - zinc-400 (8.19:1 contrast)
- **Gradient text**: `rgb(124, 58, 237)` to `rgb(117, 49, 227)` - violet-600 to violet-650 custom (3.69:1 and 3.30:1 contrast - **finely tuned** for balance and WCAG AA compliance)

**Status**: ✅ 100% WCAG AA compliant, 90% WCAG AAA compliant

### Light Theme
- **Background**: `rgb(250, 249, 255)` - Very light lavender
- **Soul Player heading**: `rgb(24, 24, 27)` - zinc-900 (16.92:1 contrast)
- **Main text**: `rgb(39, 39, 42)` - zinc-800 (14.23:1 contrast)
- **Description**: `rgb(82, 82, 91)` - zinc-600 (7.38:1 contrast)
- **Theme label**: `rgb(82, 82, 91)` - zinc-600 (7.38:1 contrast)
- **Badges**: `rgb(113, 113, 122)` - zinc-500 (4.62:1 contrast)
- **Soul Audio subtitle**: `rgb(63, 63, 70)` - zinc-700 (9.98:1 contrast)
- **Theme button (inactive)**: `rgb(82, 82, 91)` - zinc-600 (7.38:1 contrast)
- **Gradient text**: `rgb(109, 40, 217)` to `rgb(88, 28, 135)` - violet-700 to violet-900 (6.73:1 and 8.97:1 contrast)

**Status**: ✅ 100% WCAG AA compliant, 80% WCAG AAA compliant

### Ocean Theme (Light Theme)
- **Background**: `rgb(224, 242, 254)` - sky-100 (LIGHT background)
- **Soul Player heading**: `rgb(24, 24, 27)` - zinc-900 (same as light theme)
- **Main text**: `rgb(39, 39, 42)` - zinc-800 (same as light theme)
- **Description**: `rgb(82, 82, 91)` - zinc-600 (same as light theme)
- **Badges**: `rgb(82, 82, 91)` - zinc-600 (better contrast on sky-100 background)
- **Gradient text**: `rgb(109, 40, 217)` to `rgb(88, 28, 135)` - violet-700 to violet-900 (same as light theme)

**Status**: ✅ 100% WCAG AA compliant, 100% WCAG AAA compliant

## Running Contrast Tests

To validate WCAG compliance across all themes:

\`\`\`bash
yarn test:contrast
# or if yarn not configured:
node contrast-report.mjs
\`\`\`

This will generate a detailed report showing:
- Contrast ratios for all text elements
- WCAG AA and AAA compliance status
- Any failures with recommended fixes

### Example Output

\`\`\`
================================================================================
  WCAG 2.1 CONTRAST VALIDATION REPORT
  Soul Player Marketing Site - Theme Analysis
================================================================================

📋 Dark Theme
────────────────────────────────────────────────────────────────────────────────
Background: rgb(0, 0, 0)

  ✅ Soul Player (h2)
     Color: rgb(244, 244, 245)
     Contrast Ratio: 19.11:1
     Standard: Large Text (3:1 AA, 4.5:1 AAA)
     WCAG AA: ✓ PASS | WCAG AAA: ✓ PASS

...

================================================================================
  SUMMARY
================================================================================
Total Checks: 18
WCAG AA Compliance: 18/18 (100.0%)
WCAG AAA Compliance: 17/18 (94.4%)

✨ All color combinations meet WCAG AA standards!
================================================================================
\`\`\`

## Color Contrast Utilities

The site includes a comprehensive contrast checking utility at `src/utils/contrastChecker.ts` with functions for:

- **`getContrastRatio(color1, color2)`**: Calculate WCAG contrast ratio
- **`meetsWCAG_AA(ratio, isLargeText)`**: Check AA compliance
- **`meetsWCAG_AAA(ratio, isLargeText)`**: Check AAA compliance
- **`getContrastAssessment(ratio, isLargeText)`**: Get detailed assessment

Example usage:

\`\`\`typescript
import { getContrastRatio, meetsWCAG_AA } from '@/utils/contrastChecker'

const background = { r: 0, g: 0, b: 0 }
const text = { r: 212, g: 212, b: 216 }

const ratio = getContrastRatio(text, background)
const passes = meetsWCAG_AA(ratio, false)

console.log(\`Contrast: \${ratio.toFixed(2)}:1, Passes AA: \${passes}\`)
// Output: Contrast: 14.21:1, Passes AA: true
\`\`\`

## Design Guidelines

When adding new colors or themes:

1. **Always test contrast** using the contrast checker utility
2. **Target WCAG AA minimum** (4.5:1 normal, 3.0:1 large text)
3. **Aim for AAA when possible** for improved readability
4. **Update tests** in `contrast-report.mjs` for new themes
5. **Document changes** in this file

## Smooth Theme Transitions

All text elements include `transition-colors duration-700` for smooth 700ms transitions when switching themes. This applies to:

- Soul Player branding text
- Main heading and description text
- Theme labels and badges
- Background gradients

The backdrop gradient also includes `transition-all duration-700` for smooth color and opacity changes.

## Resources

- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [Color Review](https://color.review/) - Real-time contrast checker
