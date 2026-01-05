# Theme System Usage Examples

## Basic Setup

### App.tsx

```tsx
import React from 'react';
import { ThemeProvider } from '@soul-player/shared/theme';
import { MainLayout } from './components/MainLayout';

function App() {
  return (
    <ThemeProvider>
      <MainLayout />
    </ThemeProvider>
  );
}

export default App;
```

## Example 1: Simple Theme Switcher

```tsx
import { ThemeSwitcher } from '@soul-player/shared/theme';

function SettingsPage() {
  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-4">Settings</h1>

      <section className="mb-6">
        <h2 className="text-lg font-semibold mb-3">Appearance</h2>
        <ThemeSwitcher showLivePreview={true} />
      </section>
    </div>
  );
}
```

## Example 2: Theme Gallery

```tsx
import { useTheme, ThemePreview } from '@soul-player/shared/theme';

function ThemeGallery() {
  const { availableThemes, currentTheme, setTheme } = useTheme();

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-4">Choose Your Theme</h1>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {availableThemes.map((theme) => (
          <ThemePreview
            key={theme.id}
            theme={theme}
            isActive={currentTheme.id === theme.id}
            onClick={() => setTheme(theme.id)}
          />
        ))}
      </div>
    </div>
  );
}
```

## Example 3: Import/Export Theme Buttons

```tsx
import { useTheme } from '@soul-player/shared/theme';
import { useState } from 'react';

function ThemeManagement() {
  const { importTheme, exportTheme, currentTheme } = useTheme();
  const [importError, setImportError] = useState<string | null>(null);

  const handleImport = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'application/json';

    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      const text = await file.text();
      const result = importTheme(text);

      if (result.valid) {
        alert(`Theme "${result.theme?.name}" imported successfully!`);
        setImportError(null);
      } else {
        setImportError(result.errors.join('\n'));
      }
    };

    input.click();
  };

  const handleExport = () => {
    const json = exportTheme(currentTheme.id);
    if (!json) return;

    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${currentTheme.id}-theme.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="p-6">
      <h2 className="text-lg font-semibold mb-3">Theme Management</h2>

      <div className="flex gap-3">
        <button
          onClick={handleImport}
          className="px-4 py-2 bg-primary text-primary-foreground rounded"
        >
          Import Theme
        </button>

        <button
          onClick={handleExport}
          className="px-4 py-2 bg-secondary text-secondary-foreground rounded"
        >
          Export Current Theme
        </button>
      </div>

      {importError && (
        <div className="mt-4 p-3 bg-destructive/10 border border-destructive rounded">
          <p className="text-destructive font-semibold">Import Error:</p>
          <pre className="text-sm mt-1">{importError}</pre>
        </div>
      )}
    </div>
  );
}
```

## Example 4: Using Gradients

```tsx
function HeroSection() {
  return (
    <section className="bg-gradient-hero min-h-screen flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-foreground">
          Soul Player
        </h1>
        <p className="text-xl text-muted-foreground mt-4">
          Your music, beautifully themed
        </p>
      </div>
    </section>
  );
}

function PlayerUI() {
  return (
    <div className="bg-gradient-player p-6 rounded-lg">
      <div className="bg-card p-4 rounded">
        <h3 className="text-card-foreground font-semibold">
          Now Playing
        </h3>
      </div>
    </div>
  );
}
```

## Example 5: Dynamic Theme Creation

```tsx
import { useTheme } from '@soul-player/shared/theme';
import { useState } from 'react';

function ThemeCreator() {
  const { importTheme } = useTheme();
  const [themeName, setThemeName] = useState('');
  const [primaryColor, setPrimaryColor] = useState({ h: 200, s: 90, l: 50 });

  const createTheme = () => {
    const themeJson = {
      id: themeName.toLowerCase().replace(/\s+/g, '-'),
      name: themeName,
      version: '1.0.0',
      author: 'Custom Theme Creator',
      colors: {
        background: '0 0% 100%',
        foreground: '222.2 84% 4.9%',
        primary: `${primaryColor.h} ${primaryColor.s}% ${primaryColor.l}%`,
        'primary-foreground': '210 40% 98%',
        // ... other required colors with sensible defaults
        secondary: '210 40% 96.1%',
        'secondary-foreground': '222.2 47.4% 11.2%',
        muted: '210 40% 96.1%',
        'muted-foreground': '215.4 16.3% 46.9%',
        accent: `${primaryColor.h} ${primaryColor.s}% ${primaryColor.l}%`,
        'accent-foreground': '210 40% 98%',
        destructive: '0 84.2% 60.2%',
        'destructive-foreground': '210 40% 98%',
        border: '214.3 31.8% 91.4%',
        input: '214.3 31.8% 91.4%',
        ring: `${primaryColor.h} ${primaryColor.s}% ${primaryColor.l}%`,
        card: '0 0% 100%',
        'card-foreground': '222.2 84% 4.9%',
        popover: '0 0% 100%',
        'popover-foreground': '222.2 84% 4.9%',
      },
    };

    const result = importTheme(JSON.stringify(themeJson));

    if (result.valid) {
      alert('Theme created successfully!');
    } else {
      alert('Error: ' + result.errors.join(', '));
    }
  };

  return (
    <div className="p-6">
      <h2 className="text-lg font-semibold mb-3">Create Custom Theme</h2>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium mb-1">
            Theme Name
          </label>
          <input
            type="text"
            value={themeName}
            onChange={(e) => setThemeName(e.target.value)}
            className="w-full px-3 py-2 border border-border rounded"
            placeholder="My Awesome Theme"
          />
        </div>

        <div>
          <label className="block text-sm font-medium mb-1">
            Primary Color
          </label>
          <div className="flex gap-3">
            <input
              type="range"
              min="0"
              max="360"
              value={primaryColor.h}
              onChange={(e) => setPrimaryColor({
                ...primaryColor,
                h: parseInt(e.target.value)
              })}
            />
            <div
              className="w-12 h-12 rounded border"
              style={{
                backgroundColor: `hsl(${primaryColor.h} ${primaryColor.s}% ${primaryColor.l}%)`
              }}
            />
          </div>
        </div>

        <button
          onClick={createTheme}
          disabled={!themeName}
          className="px-4 py-2 bg-primary text-primary-foreground rounded disabled:opacity-50"
        >
          Create Theme
        </button>
      </div>
    </div>
  );
}
```

## Example 6: Theme-Aware Component

```tsx
import { useTheme } from '@soul-player/shared/theme';

function PlayerControls() {
  const { currentTheme } = useTheme();

  // Adjust UI based on current theme
  const isDarkTheme = currentTheme.id === 'dark';

  return (
    <div className={`
      p-4 rounded-lg
      ${isDarkTheme ? 'shadow-2xl' : 'shadow-md'}
    `}>
      <button className="px-4 py-2 bg-primary text-primary-foreground rounded">
        Play
      </button>
    </div>
  );
}
```

## Example 7: Accessibility Info Display

```tsx
import { useTheme, checkContrast } from '@soul-player/shared/theme';

function ThemeAccessibilityInfo() {
  const { currentTheme } = useTheme();

  const bgFgContrast = checkContrast(
    currentTheme.colors.foreground,
    currentTheme.colors.background
  );

  const primaryContrast = checkContrast(
    currentTheme.colors['primary-foreground'],
    currentTheme.colors.primary
  );

  return (
    <div className="p-6">
      <h2 className="text-lg font-semibold mb-3">Accessibility Info</h2>

      <div className="space-y-3">
        <div className="p-3 bg-muted rounded">
          <p className="font-medium">Text on Background</p>
          <p className="text-sm text-muted-foreground">
            Contrast: {bgFgContrast.ratio.toFixed(2)}:1
          </p>
          <p className="text-sm">
            {bgFgContrast.passes.aa ? '✅' : '❌'} WCAG AA
            {' | '}
            {bgFgContrast.passes.aaa ? '✅' : '❌'} WCAG AAA
          </p>
        </div>

        <div className="p-3 bg-muted rounded">
          <p className="font-medium">Primary Button</p>
          <p className="text-sm text-muted-foreground">
            Contrast: {primaryContrast.ratio.toFixed(2)}:1
          </p>
          <p className="text-sm">
            {primaryContrast.passes.aa ? '✅' : '❌'} WCAG AA
            {' | '}
            {primaryContrast.passes.aaa ? '✅' : '❌'} WCAG AAA
          </p>
        </div>
      </div>
    </div>
  );
}
```

## Example 8: Theme Preview Before Applying

```tsx
import { useTheme } from '@soul-player/shared/theme';
import { useState } from 'react';

function ThemeSelector() {
  const { availableThemes, currentTheme, setTheme, previewTheme } = useTheme();
  const [previewingTheme, setPreviewingTheme] = useState<string | null>(null);
  const [restoreFunction, setRestoreFunction] = useState<(() => void) | null>(null);

  const handlePreview = (themeId: string) => {
    // Clear any existing preview
    if (restoreFunction) {
      restoreFunction();
    }

    // Start new preview
    const restore = previewTheme(themeId);
    setRestoreFunction(() => restore);
    setPreviewingTheme(themeId);
  };

  const handleApply = () => {
    if (previewingTheme) {
      setTheme(previewingTheme);
      setPreviewingTheme(null);
      setRestoreFunction(null);
    }
  };

  const handleCancel = () => {
    if (restoreFunction) {
      restoreFunction();
    }
    setPreviewingTheme(null);
    setRestoreFunction(null);
  };

  return (
    <div className="p-6">
      <h2 className="text-lg font-semibold mb-3">Select Theme</h2>

      <div className="grid grid-cols-3 gap-3 mb-4">
        {availableThemes.map((theme) => (
          <button
            key={theme.id}
            onClick={() => handlePreview(theme.id)}
            className={`
              p-3 rounded border-2
              ${previewingTheme === theme.id ? 'border-primary' : 'border-border'}
            `}
          >
            {theme.name}
          </button>
        ))}
      </div>

      {previewingTheme && (
        <div className="flex gap-3">
          <button
            onClick={handleApply}
            className="px-4 py-2 bg-primary text-primary-foreground rounded"
          >
            Apply Theme
          </button>
          <button
            onClick={handleCancel}
            className="px-4 py-2 bg-secondary text-secondary-foreground rounded"
          >
            Cancel
          </button>
        </div>
      )}
    </div>
  );
}
```

## Common Patterns

### Using Theme Colors Directly

```tsx
<div style={{ color: `hsl(var(--primary))` }}>
  Custom styled text
</div>
```

### Conditional Styling Based on Theme

```tsx
const { currentTheme } = useTheme();

<div className={
  currentTheme.id === 'dark'
    ? 'border-white/10'
    : 'border-black/10'
}>
  Theme-aware borders
</div>
```

### Gradient Backgrounds

```tsx
<div className="bg-gradient-waveform p-6">
  Waveform visualization
</div>
```
