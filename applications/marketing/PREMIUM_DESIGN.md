# Premium Hero Design

Inspired by Launch UI, this is a modern, premium landing page with demo-first design.

## 🎨 Design Principles

### Visual Hierarchy
1. **Badge** - Announcement at top
2. **Headline** - Large gradient text as focal point
3. **Description** - Supporting copy
4. **CTA** - Download button
5. **Demo** - Full-width showcase with backdrop

### Key Features

✅ **Demo-First Layout** - Product showcase is the hero
✅ **Grainy Gradient Backdrop** - Atmospheric depth behind demo
✅ **Staggered Animations** - Premium feel with delayed reveals
✅ **Generous Spacing** - Breathing room between elements
✅ **Minimalist Design** - Restrained, focused on what matters

## 🏗️ Structure

```tsx
<PremiumHero>
  ├── Background Gradients (radial layers)
  ├── Badge ("Self-hosted music player")
  ├── Headline ("Your Music, Your Way")
  ├── Description
  ├── Download Button (OS detection)
  └── Demo Showcase
      ├── Grainy Gradient Backdrop
      ├── Desktop App Demo (DemoModeWrapper)
      ├── Decorative Blur Elements
      └── Feature Pills (platforms, features)
</PremiumHero>
```

## 🎭 Animation Timing

- **Badge**: Immediate (0ms)
- **Headline**: 100ms delay
- **CTA**: 300ms delay
- **Demo**: 500ms delay

Creates a cascade effect that feels premium and intentional.

## 🌈 Gradient System

### Background Layers
```css
1. Large radial (800px) - violet-700 at 30% opacity
2. Small radial (400px) - violet-400 at 20% opacity
```

### Demo Backdrop
```css
1. Grain texture overlay (15% opacity)
2. Radial gradient glow (25% to 15% to transparent)
3. Decorative blur spots (violet/purple)
```

## 🎨 Color Palette

- **Primary Gradient**: violet-200 → violet-300 → violet-400
- **Text**: zinc-400 (body), zinc-300 (emphasis)
- **Borders**: zinc-800/50 (subtle)
- **Backgrounds**: zinc-900/30 with backdrop-blur

## 📦 Components Used

### New Components
- **`PremiumHero.tsx`** - Main hero section
- **`Badge.tsx`** - Announcement badge component

### Existing Components
- **`DownloadButton.tsx`** - OS detection + dropdown
- **`DemoModeWrapper.tsx`** - Non-interactive wrapper

## 🔄 Next Steps: Import Desktop Component

To show the real desktop app in the demo:

### 1. Export a Demo Component from Desktop App

In `applications/desktop/src/`:

```tsx
// components/DemoView.tsx
export function DemoView() {
  return (
    <div className="w-full h-full">
      {/* Your actual player UI */}
      <PlayerInterface demo={true} />
    </div>
  )
}
```

### 2. Add to Shared Package

In `applications/shared/src/index.ts`:

```tsx
export { DemoView } from '../desktop/src/components/DemoView'
```

### 3. Import in Marketing Site

In `PremiumHero.tsx`:

```tsx
import { DemoView } from '@soul-player/shared'

// Replace placeholder with:
<DemoModeWrapper className="aspect-video">
  <DemoView />
</DemoModeWrapper>
```

## 🎯 Design Philosophy (Launch UI Inspired)

### What Makes It Premium

1. **Restraint**: Not everything needs to animate or glow
2. **Spacing**: Generous white space = luxury
3. **Typography**: Hierarchy through weight, not decoration
4. **Subtlety**: Effects enhance, don't distract
5. **Performance**: Animations are GPU-accelerated

### What to Avoid

❌ **Over-animation** - Too much motion is distracting
❌ **Cluttered spacing** - Cramped layouts feel cheap
❌ **Garish colors** - Stick to the palette
❌ **Unnecessary elements** - Every pixel serves a purpose

## 📊 Responsive Behavior

- **Mobile**: Stacks vertically, maintains spacing
- **Tablet**: Same layout, optimized sizing
- **Desktop**: Full effect with all gradients and animations

### Breakpoints
- `text-5xl` → `md:text-7xl` → `lg:text-8xl`
- `px-6` → `container mx-auto`
- `gap-3` → `gap-8` on larger screens

## 🎬 Animation Details

### Fade In Keyframes
```css
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(20px);  /* Subtle lift */
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

### Timing Functions
- **Duration**: 800ms
- **Easing**: `ease-out` (natural deceleration)
- **Delays**: 0ms, 100ms, 300ms, 500ms

## 🔧 Customization

### Change Gradient Colors

In `PremiumHero.tsx`:

```tsx
// Main gradient
background: 'radial-gradient(circle, rgba(124, 58, 237, 0.3) 0%, transparent 70%)'

// Change to blue:
background: 'radial-gradient(circle, rgba(59, 130, 246, 0.3) 0%, transparent 70%)'
```

### Adjust Animation Speed

In `globals.css`:

```css
.animate-fade-in {
  animation: fadeIn 1.2s ease-out forwards;  /* Slower */
}
```

### Change Demo Aspect Ratio

```tsx
<div className="aspect-[16/10]">  {/* Wider */}
  <DemoModeWrapper>
```

## 📝 Best Practices

1. **Keep It Clean**: Don't add more gradients than necessary
2. **Test on Mobile**: Ensure touch targets are 44px minimum
3. **Performance**: Use `will-change` sparingly
4. **Accessibility**: Maintain color contrast ratios
5. **Loading**: Show skeleton state during demo load

## 🚀 Performance Optimizations

- Animations use `transform` and `opacity` (GPU-accelerated)
- Gradients are CSS-based (no images)
- Backdrop blur is contained to specific areas
- Grain texture is SVG data URI (no HTTP request)

---

## 🎨 Visual Reference

Launch UI principles applied:
- ✅ Demo-first showcase
- ✅ Centered vertical composition
- ✅ Staggered animation reveals
- ✅ Generous spacing and breathing room
- ✅ Minimalist with intentional effects
- ✅ Premium feel through restraint

**Result**: A modern, conversion-optimized landing page that feels expensive and professional.
