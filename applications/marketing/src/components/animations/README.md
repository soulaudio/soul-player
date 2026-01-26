# Animation Components

Reusable animation components for the Soul Player marketing site.

## ScrollRotate3D

A performant scroll-based 3D rotation component that adds depth and interactivity to elements as users scroll through the page.

### Features

- **Intersection Observer**: Only animates when element is in viewport
- **Smooth Interpolation**: Uses linear interpolation (lerp) for buttery-smooth transitions
- **Scroll Progress Tracking**: Calculates precise scroll position within viewport (0-1)
- **3D Transforms**: Applies perspective, rotateY, rotateX, and scale based on scroll
- **Performance Optimized**: Uses CSS transforms and requestAnimationFrame (no framer-motion overhead)
- **Data Attributes**: Follows CLAUDE.md styling guidelines with `data-state` attribute
- **TypeScript**: Fully typed with sensible defaults

### Usage

#### Basic Example

```tsx
import { ScrollRotate3D } from '@/components/animations'

function MySection() {
  return (
    <ScrollRotate3D>
      <div className="card">
        <h2>My Rotating Card</h2>
        <p>This will rotate as you scroll!</p>
      </div>
    </ScrollRotate3D>
  )
}
```

#### Custom Configuration

```tsx
<ScrollRotate3D
  initialRotateY={20}      // Start rotation (when entering viewport)
  maxRotateY={-20}         // End rotation (when exiting viewport)
  initialRotateX={-8}      // Initial X tilt
  maxRotateX={8}           // Final X tilt
  perspective={1000}       // Perspective depth
  minScale={0.92}          // Scale at viewport edges
  maxScale={1.05}          // Scale at viewport center
  smoothing={0.1}          // Interpolation speed (0-1)
>
  <YourContent />
</ScrollRotate3D>
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `children` | `ReactNode` | Required | Content to animate |
| `className` | `string` | `''` | Additional CSS classes |
| `initialRotateY` | `number` | `15` | Y-axis rotation when entering viewport (degrees) |
| `initialRotateX` | `number` | `-5` | X-axis rotation when entering viewport (degrees) |
| `maxRotateY` | `number` | `-15` | Y-axis rotation when exiting viewport (degrees) |
| `maxRotateX` | `number` | `5` | X-axis rotation when exiting viewport (degrees) |
| `perspective` | `number` | `1200` | CSS perspective value (pixels) |
| `minScale` | `number` | `0.95` | Scale at viewport edges |
| `maxScale` | `number` | `1` | Scale at viewport center |
| `smoothing` | `number` | `0.08` | Lerp smoothing factor (0-1, lower = smoother) |

### Examples

See `ScrollRotate3D.example.tsx` for comprehensive examples including:

1. **Default configuration** - Standard settings for most use cases
2. **Subtle effect** - Minimal movement for professional layouts
3. **Dramatic effect** - Pronounced rotation for hero sections
4. **Product screenshots** - Optimized settings for showcasing images

### How It Works

1. **Intersection Observer** detects when element enters/exits viewport
2. **Scroll listener** calculates scroll progress through viewport (0 = bottom, 1 = top)
3. **Target values** are computed based on scroll progress
4. **Animation loop** (requestAnimationFrame) smoothly interpolates current values toward targets using lerp
5. **Transforms applied** via inline styles for maximum performance

### Performance Notes

- Uses `willChange: 'transform'` for GPU acceleration
- Only runs when element is in viewport (via Intersection Observer)
- No CSS transitions - all animation via JS for precise control
- Transform-only animations (no layout reflows)
- Passive scroll listener for better scrolling performance

### Styling Guidelines

Follows CLAUDE.md best practices:

- Uses CSS variables from theme (via inline styles or Tailwind)
- Uses `data-state` attribute (`visible` | `hidden`) for conditional styling
- No hardcoded colors - always uses theme tokens
- Transform-based animations (GPU accelerated)

### Data Attributes

The component sets a `data-state` attribute you can use for conditional styling:

```tsx
<ScrollRotate3D className="my-element">
  {/* ... */}
</ScrollRotate3D>
```

```css
/* CSS targeting based on visibility */
.my-element[data-state="visible"] {
  /* Styles when in viewport */
}

.my-element[data-state="hidden"] {
  /* Styles when out of viewport */
}
```

### Integration with Existing Components

Works seamlessly with other marketing site components like `FadeIn`:

```tsx
import { FadeIn, ScrollRotate3D } from '@/components/animations'

function FeatureShowcase() {
  return (
    <FadeIn direction="up" delay={0.2}>
      <ScrollRotate3D>
        <div className="feature-card">
          <h3>Amazing Feature</h3>
          <p>Fades in, then rotates on scroll</p>
        </div>
      </ScrollRotate3D>
    </FadeIn>
  )
}
```

### Common Use Cases

#### Hero Section Screenshot

```tsx
<ScrollRotate3D
  initialRotateY={10}
  maxRotateY={-10}
  initialRotateX={-3}
  maxRotateX={3}
>
  <img src="/hero-screenshot.png" alt="Soul Player Interface" />
</ScrollRotate3D>
```

#### Feature Cards

```tsx
<ScrollRotate3D
  initialRotateY={8}
  maxRotateY={-8}
  minScale={0.97}
  maxScale={1.02}
>
  <FeatureCard title="Your Music" description="..." />
</ScrollRotate3D>
```

#### Support/CTA Sections

```tsx
<ScrollRotate3D
  initialRotateY={12}
  maxRotateY={-12}
  perspective={1000}
>
  <SupportCard />
</ScrollRotate3D>
```

### Troubleshooting

**Element not rotating:**
- Ensure element is tall enough to scroll through viewport
- Check that parent containers don't have `overflow: hidden` preventing scroll
- Verify Intersection Observer is supported (modern browsers only)

**Jittery animation:**
- Increase `smoothing` value (try 0.15-0.2) for snappier response
- Decrease `smoothing` value (try 0.05) for smoother motion

**Too much/too little rotation:**
- Adjust `initialRotateY`, `maxRotateY`, `initialRotateX`, `maxRotateX` values
- Try starting with subtle values (±5-10deg) and increasing gradually

**Performance issues:**
- Component already uses best practices (transforms, RAF, Intersection Observer)
- Reduce number of simultaneous instances on page
- Simplify child component DOM structure

## FadeIn

Existing fade-in animation component using react-awesome-reveal. See `FadeIn.tsx` for details.
