# Volume Control Best Practices

## Overview

The VolumeControl component implements industry best practices for audio volume sliders based on WCAG accessibility guidelines and common patterns from major music players (Spotify, Apple Music, YouTube Music).

## Research Sources

- [WCAG 1.4.2 Audio Control](https://wcag.dock.codes/documentation/wcag142/)
- [ARIA Slider Role Best Practices](https://www.digitala11y.com/slider-role/)
- [Music Player UI Design Patterns](https://demo.wallsneedlove.com/blog/improve-music-assistant-volume-slider)
- [Logarithmic Volume Scaling](https://johnleonardfrench.music/the-right-way-to-make-a-volume-slider-in-unity-using-logarithmic-conversion/)
- [Volume Control Anti-patterns](https://www.adhamdannaway.com/blog/ui-design/how-not-to-design-a-volume-control)

## Key Features

### 1. Accessibility (WCAG Compliant)

**ARIA Attributes:**
- `aria-label="Volume"` on slider for screen readers
- `aria-valuemin="0"`, `aria-valuemax="100"` define range
- `aria-valuenow` and `aria-valuetext` provide current value
- `aria-pressed` on mute button indicates toggle state
- `aria-live="polite"` for dynamic volume percentage updates

**Keyboard Navigation:**
- Native `<input type="range">` provides full keyboard support
- Arrow Up/Down: Increase/decrease by step (1%)
- Page Up/Down: Larger increments
- Home/End: Jump to min/max
- All without custom JavaScript

**Focus Management:**
- Visible focus indicators using `focus-visible`
- Minimum 44x44px touch targets (WCAG 2.1 Level AAA)
- Clear visual feedback on hover and focus

### 2. Perceptual Volume Scaling (Backend Implementation)

**Problem:** Linear volume scaling (0-100%) doesn't match human perception.
- Humans perceive volume logarithmically
- Linear sliders feel "jumpy" - small changes at low volumes have huge perceptual impact

**Solution:** Backend handles logarithmic conversion (soul-playback Volume module)
```rust
// Backend: Volume level (0-100) → Logarithmic gain conversion
fn calculate_linear_gain(level: u8) -> f32 {
    // Maps 0-100% to -60 dB to 0 dB
    let db = (level as f32 - 100.0) * 0.6; // 0.6 = 60/100
    10.0_f32.powf(db / 20.0)
}
```

**Implementation Flow:**
1. Backend stores volume as level (0-100) with internal logarithmic scaling
2. Backend emits level (0-100) via playback events
3. Frontend receives level (0-100), stores as 0-1 for consistency
4. Component displays level as-is on slider (0-1)
5. User adjusts slider (0-1 level)
6. Component passes level (0-1) to onChange
7. Backend converts 0-1 → 0-100 and applies logarithmic gain internally

**Technical Details:**
- Backend uses decibel (dB) conversion: `dB = 20 * log10(linear)`
- Maps 0-100 level to -60dB to 0dB logarithmic gain range
- Provides smooth, natural-feeling volume control
- Matches professional audio software behavior (Spotify, Apple Music)
- UI displays level percentage (0-100%), backend applies logarithmic scaling
- No frontend conversion needed - prevents double-scaling bugs

### 3. Platform-Appropriate Behavior

**Desktop:**
- Click anywhere on slider to jump to that volume (intentional action)
- Mouse wheel support for fine adjustments
- Direct manipulation with draggable thumb

**Mobile (Future):**
- Tap should increment/decrement (prevent accidental jumps)
- Drag for precise control
- Prevent volume changes during scrolling

### 4. Visual Design

**Components:**
- Mute/Unmute toggle button with icon
- Horizontal slider with track and fill
- Draggable thumb (visible on hover)
- Percentage display (0-100)

**Feedback:**
- Smooth transitions (100ms duration)
- Hover states on all interactive elements
- Color changes: muted/primary states
- Real-time percentage updates

### 5. Mute Functionality

**Toggle Behavior:**
- Single click mutes/unmutes
- Preserves volume level when muting
- Restores previous volume on unmute
- Shows VolumeX icon when muted or volume = 0

**Keyboard Shortcut:**
- "M" key for mute/unmute (handled by parent)
- Indicated in button tooltip: "Mute (M)"

## Common Patterns from Popular Apps

### Spotify
- Horizontal slider with percentage
- Logarithmic scaling
- Mute toggle with volume memory
- Hover shows draggable thumb

### Apple Music
- Similar horizontal design
- Integration with system volume
- Smooth animations
- Accessible keyboard controls

### YouTube Music
- Consistent volume feature across devices
- Normalization to -14 LUFS
- Similar UI patterns

## Implementation Details

### Component Props

```typescript
interface VolumeControlProps {
  volume: number;          // 0-1 linear scale
  isMuted: boolean;        // Mute state
  onVolumeChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onMuteToggle: () => void;
  onWheel?: (e: React.WheelEvent) => void;  // Optional wheel support
}
```

### Parent Component Responsibilities

The parent (PlayerPanel) handles:
- Volume state management (Zustand store)
- Debouncing (150ms) to avoid excessive backend calls
- Mute state and volume restoration
- Integration with audio backend

### Testing

**33 comprehensive tests covering:**
- Rendering and visual states
- ARIA attributes and accessibility
- User interactions (click, keyboard, wheel)
- Volume percentage calculations
- Edge cases (0, 1, rapid changes)
- Logarithmic conversion accuracy
- Round-trip conversion stability

Run tests:
```bash
cd applications/shared
npx vitest run src/components/sidebar/__tests__/VolumeControl.test.tsx
```

## Accessibility Checklist

- ✅ WCAG 1.4.2: Independent volume control
- ✅ WCAG 2.4.7: Focus visible
- ✅ WCAG 2.5.5: Target size (44x44px)
- ✅ WCAG 4.1.2: Name, role, value (ARIA)
- ✅ Keyboard accessible (no mouse required)
- ✅ Screen reader compatible
- ✅ Color contrast (primary/muted)
- ✅ Perceivable volume changes (aria-live)

## Future Enhancements

1. **Mobile Optimization**
   - Tap to increment/decrement on mobile
   - Block during scroll
   - Haptic feedback

2. **Advanced Features**
   - Volume normalization indicator
   - Peak level meter
   - Per-device volume memory
   - Volume limit protection

3. **Customization**
   - User-configurable wheel sensitivity
   - Alternative visual styles (vertical, circular)
   - Custom step sizes

## References

### Accessibility
- WCAG 2.1: https://www.w3.org/WAI/WCAG21/Understanding/
- ARIA Authoring Practices: https://wai-aria-practices.netlify.app/

### Audio & Volume Control
- Web Audio API: https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API
- GainNode (Web Audio): https://developer.mozilla.org/en-US/docs/Web/API/GainNode
- Programming Volume Controls: https://www.dr-lex.be/info-stuff/volumecontrols.html
- Web Audio API - Volume and Loudness: https://www.oreilly.com/library/view/web-audio-api/9781449332679/ch03.html
- Logarithmic Volume Control: https://medium.com/zattoo_tech/logarithmic-volume-control-bd89cc1f2135
- Volume Controls (Relative vs Absolute): http://www.audioerudite.com/p/relative-vs-absolute-volume-controls.html
