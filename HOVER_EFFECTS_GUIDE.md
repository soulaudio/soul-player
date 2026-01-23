# Hover Effects Style Guide

## Overview

Soul Player uses **opacity-based hover effects** instead of full color changes to create subtle, consistent interactions across all themes.

## Principles

1. **Use opacity reduction** - Reduce opacity instead of changing to a different color
2. **Maintain contrast** - All themes have 80%+ contrast, opacity effects preserve readability
3. **Keep transitions smooth** - Use `transition-opacity` for hover states
4. **Consistency** - Apply the same patterns across all components

## Standard Patterns

### Text Hover Effects

**Before (color change):**
```tsx
className="text-muted-foreground hover:text-foreground"
```

**After (opacity-based):**
```tsx
className="text-muted-foreground hover:opacity-80 transition-opacity"
```

### Button Hover Effects

**Before (background color change):**
```tsx
// Default variant
className="bg-primary hover:bg-primary/90"

// Ghost variant
className="hover:bg-accent hover:text-accent-foreground"
```

**After (opacity-based):**
```tsx
// Default variant
className="bg-primary hover:opacity-90"

// Ghost variant
className="hover:bg-foreground/10"
```

### Interactive Elements

**Before (color and background change):**
```tsx
className="text-muted-foreground hover:text-foreground hover:bg-accent"
```

**After (opacity-based):**
```tsx
className="text-muted-foreground hover:opacity-80 hover:bg-foreground/10 transition-opacity"
```

### Cards and Media

**Before (gradient color change):**
```tsx
className="bg-gradient-to-br from-primary/20 to-primary/5 group-hover:from-primary/30 group-hover:to-primary/10"
```

**After (opacity-based on container):**
```tsx
className="bg-gradient-to-br from-primary/20 to-primary/5 group-hover:opacity-90 transition-opacity"
```

## Opacity Values

- **80%** (`opacity-80`) - Standard hover for text and icons
- **90%** (`opacity-90`) - Subtle hover for buttons and large elements
- **50%** (`opacity-50`) - Disabled states

## Background Overlays

For hover overlays on interactive elements:
- Use `hover:bg-foreground/10` for subtle highlight
- Use `hover:bg-black/70` for dark overlays (play buttons, etc.)

## Components Updated

### Core Components (Completed)
- ✓ `button.tsx` - Base button component
- ✓ `MediaCard.tsx` - Album/artist/playlist cards
- ✓ `LeftSidebar.tsx` - Navigation and player controls

### Components to Update (Future)
The following components still use color-change hover effects and should be updated gradually:
- Navigation items in other layouts
- Settings page elements
- Dialog and modal buttons
- Dropdown menu items
- Form inputs and controls

## Theme Contrast Verification

All four themes have been verified for sufficient contrast:

| Theme | Background Lightness | Foreground Lightness | Contrast | Status |
|-------|---------------------|---------------------|----------|--------|
| Day   | 98%                 | 15%                 | 83%      | ✓ Excellent |
| Night | 8%                  | 92%                 | 84%      | ✓ Excellent |
| Ocean | 8%                  | 88%                 | 80%      | ✓ Excellent |
| Earth | 7%                  | 88%                 | 81%      | ✓ Excellent |

All themes exceed WCAG AAA standards (~70% required).

## Migration Checklist

When updating a component:
1. ✓ Find all `hover:text-foreground` → Replace with `hover:opacity-80`
2. ✓ Find all `hover:bg-accent` → Replace with `hover:bg-foreground/10`
3. ✓ Change `transition-colors` → `transition-opacity` (if only opacity changes)
4. ✓ Keep transform effects like `hover:scale-105` (they work well with opacity)
5. ✓ Test in all four themes (Day, Night, Ocean, Earth)

## Examples from Codebase

### Button Component
```tsx
// applications/shared/src/components/ui/button.tsx
{
  'bg-primary text-primary-foreground shadow hover:opacity-90': variant === 'default',
  'hover:bg-foreground/10': variant === 'ghost',
  'border border-input bg-transparent hover:bg-foreground/10': variant === 'outline',
}
```

### MediaCard Title
```tsx
// applications/shared/src/components/MediaCard.tsx
<p className={`font-medium truncate group-hover:opacity-80 transition-opacity cursor-pointer`}>
  {title}
</p>
```

### Navigation Item
```tsx
// applications/shared/src/components/LeftSidebar.tsx
className={cn(
  "w-full text-left px-3 py-1 text-xl font-semibold tracking-wide transition-opacity",
  isActive(item.path) ? 'text-primary' : 'text-muted-foreground hover:opacity-80'
)}
```

## Why Opacity-Based?

1. **Consistency** - Same visual pattern across all themes
2. **Subtlety** - Less jarring than full color changes
3. **Theme Independence** - Works automatically with any theme colors
4. **Accessibility** - Maintains contrast ratios in all themes
5. **Performance** - Simpler CSS transitions

---

**Last Updated:** 2026-01-23
**Related Files:** CLAUDE.md, applications/shared/src/theme/themes/*.ts
