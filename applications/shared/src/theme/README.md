# Soul Player Theme System

A comprehensive theming system for Soul Player with support for custom themes, live preview, and accessibility validation.

## Features

- **Multiple Themes**: Built-in light, dark, and ocean themes
- **Custom Themes**: Import/export custom themes via JSON
- **Live Preview**: Preview themes before applying
- **Accessibility**: WCAG 2.1 AA/AAA contrast validation
- **Gradients & Typography**: Support for custom gradients and fonts
- **Persistent Storage**: Themes saved to localStorage
- **Cross-Platform**: Works on desktop and mobile

## Quick Start

### 1. Wrap Your App with ThemeProvider

```tsx
import { ThemeProvider } from '@soul-player/shared/theme';

function App() {
  return (
    <ThemeProvider>
      <YourApp />
    </ThemeProvider>
  );
}
```

### 2. Use the ThemeSwitcher Component

```tsx
import { ThemeSwitcher } from '@soul-player/shared/theme';

function Settings() {
  return (
    <div>
      <h2>Settings</h2>
      <ThemeSwitcher showLivePreview={true} />
    </div>
  );
}
```

### 3. Use Theme Values in Your Components

```tsx
import { useTheme } from '@soul-player/shared/theme';

function MyComponent() {
  const { currentTheme } = useTheme();

  return (
    <div>
      <p>Current theme: {currentTheme.name}</p>
    </div>
  );
}
```

## Built-in Themes

### Light Theme
- Clean and bright default theme
- High contrast for readability
- ID: `light`

### Dark Theme
- Sleek dark theme for low-light environments
- Reduced eye strain
- ID: `dark`

### Ocean Theme
- Cool blue/teal palette
- Calming colors inspired by ocean depths
- ID: `ocean`

## Creating Custom Themes

### Theme JSON Structure

```json
{
  "id": "my-custom-theme",
  "name": "My Custom Theme",
  "version": "1.0.0",
  "author": "Your Name",
  "description": "A beautiful custom theme",
  "colors": {
    "background": "210 100% 97%",
    "foreground": "210 60% 15%",
    "primary": "200 90% 50%",
    "primary-foreground": "210 100% 98%",
    // ... all other color tokens
  },
  "gradients": {
    "hero": "linear-gradient(135deg, hsl(200 90% 50%), hsl(180 85% 55%))",
    "player": "linear-gradient(to bottom, hsl(210 100% 97%), hsl(200 80% 92%))"
  },
  "typography": {
    "fontFamily": {
      "sans": ["Inter", "system-ui", "sans-serif"],
      "mono": ["JetBrains Mono", "monospace"]
    },
    "fontSize": {
      "base": "16px"
    }
  }
}
```

### Required Color Tokens

All themes must include these color tokens:

- `background`, `foreground`
- `card`, `card-foreground`
- `popover`, `popover-foreground`
- `primary`, `primary-foreground`
- `secondary`, `secondary-foreground`
- `muted`, `muted-foreground`
- `accent`, `accent-foreground`
- `destructive`, `destructive-foreground`
- `border`, `input`, `ring`

### Color Format

Colors use HSL format without the `hsl()` wrapper:
- Format: `"hue saturation% lightness%"`
- Example: `"210 100% 50%"` (bright blue)

## API Reference

### useTheme Hook

```tsx
const {
  currentTheme,        // Currently active theme
  availableThemes,     // Array of all themes (built-in + custom)
  setTheme,           // Switch to a theme by ID
  importTheme,        // Import theme from JSON string
  exportTheme,        // Export theme to JSON string
  deleteTheme,        // Delete a custom theme
  previewTheme,       // Preview theme temporarily
} = useTheme();
```

### Importing a Theme

```tsx
const { importTheme } = useTheme();

const result = importTheme(jsonString);

if (result.valid) {
  console.log('Theme imported successfully!');
  console.log('Warnings:', result.warnings); // Accessibility warnings
} else {
  console.error('Import failed:', result.errors);
}
```

### Exporting a Theme

```tsx
const { exportTheme } = useTheme();

const json = exportTheme('ocean');

if (json) {
  // Download as file or copy to clipboard
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'ocean-theme.json';
  a.click();
}
```

### Live Preview

```tsx
const { previewTheme } = useTheme();

// Preview a theme temporarily
const restore = previewTheme('ocean');

// Later, restore the previous theme
if (restore) {
  restore();
}
```

## Using Gradients in Components

Gradients are available as Tailwind utility classes:

```tsx
<div className="bg-gradient-hero">Hero Section</div>
<div className="bg-gradient-player">Player UI</div>
<div className="bg-gradient-sidebar">Sidebar</div>
<div className="bg-gradient-waveform">Waveform</div>
```

Or use CSS variables directly:

```tsx
<div style={{ background: 'var(--gradient-hero)' }}>
  Custom gradient usage
</div>
```

## Validation

### WCAG Contrast Validation

The theme system automatically checks contrast ratios:

- **AA Standard**: 4.5:1 for normal text (required)
- **AAA Standard**: 7:1 for normal text (recommended)

Import validation will:
- **Error** if contrast fails AA
- **Warn** if contrast passes AA but fails AAA

### Manual Validation

```tsx
import { validateTheme, checkContrast } from '@soul-player/shared/theme';

// Validate complete theme
const result = validateTheme(themeObject);
console.log(result.valid, result.errors, result.warnings);

// Check specific color pair
const contrast = checkContrast('210 100% 50%', '0 0% 100%');
console.log(contrast.ratio); // e.g., 5.2
console.log(contrast.passes.aa); // true/false
console.log(contrast.passes.aaa); // true/false
```

## Components

### ThemeSwitcher

Simple dropdown for theme selection with optional live preview.

```tsx
<ThemeSwitcher
  showLivePreview={true}
  className="my-custom-class"
/>
```

### ThemePreview

Visual preview card showing theme colors.

```tsx
<ThemePreview
  theme={oceanTheme}
  isActive={currentTheme.id === 'ocean'}
  onClick={() => setTheme('ocean')}
/>
```

## Storage

Themes are persisted to localStorage:

- **Current theme**: `soul-player-current-theme`
- **Custom themes**: `soul-player-custom-themes`

## Advanced Usage

### Accessing ThemeManager Directly

```tsx
import { themeManager } from '@soul-player/shared/theme';

// Get all themes
const themes = themeManager.getAllThemes();

// Set theme
themeManager.setCurrentTheme('dark');

// Preview theme
const restore = themeManager.previewTheme('ocean');
```

### Adding New Built-in Themes

1. Create theme file in `themes/` directory:

```tsx
// themes/sunset.ts
import type { Theme } from '../types';

export const sunsetTheme: Theme = {
  id: 'sunset',
  name: 'Sunset',
  version: '1.0.0',
  isBuiltIn: true,
  colors: { /* ... */ },
};
```

2. Export from `themes/index.ts`:

```tsx
export { sunsetTheme } from './sunset';
export const builtInThemes = [
  lightTheme,
  darkTheme,
  oceanTheme,
  sunsetTheme
];
```

3. Add CSS in `index.css`:

```css
[data-theme='sunset'] {
  --background: /* ... */;
  /* ... other variables */
}
```

## TypeScript Support

Full TypeScript support with type exports:

```tsx
import type {
  Theme,
  ThemeColors,
  ThemeValidationResult
} from '@soul-player/shared/theme';
```

## Browser Compatibility

Works in all modern browsers that support:
- CSS custom properties
- localStorage
- ES6+

## Troubleshooting

### Theme not applying on first load

Make sure `ThemeProvider` wraps your entire app and is mounted before other components.

### Custom fonts not loading

Ensure fonts are imported in your CSS or available via CDN:

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
```

### Theme colors look wrong

Check that you're using the HSL format correctly (without `hsl()` wrapper):
- ✅ Correct: `"210 100% 50%"`
- ❌ Wrong: `"hsl(210, 100%, 50%)"`

## License

Part of Soul Player - see main project LICENSE
