# UI Components Design Documentation

**Soul Player - Audio Settings UI Components**

Created: 2026-01-09

---

## Overview

This document details the UI components created for Soul Player's professional audio settings interface. The design features a sidebar navigation layout with a visual audio pipeline and comprehensive settings for each processing stage.

---

## Component Architecture

```
SettingsLayout
├── SettingsSidebar (Navigation)
└── AudioSettingsPage (Main Content)
    ├── PipelineVisualization
    ├── BackendSelector
    ├── DeviceSelector
    ├── DspConfigurator
    ├── UpsamplingSettings
    ├── VolumeLevelingSettings
    └── BufferSettings
```

---

## Component Specifications

### 1. **SettingsLayout**

**File**: `applications/shared/src/components/settings/SettingsLayout.tsx`

**Purpose**: Container layout with sidebar navigation

**Features**:
- Two-column layout (sidebar + main content)
- Responsive sidebar (56rem width)
- Scrollable main content area
- Maximum width constraint (4xl)

**Props**:
```typescript
interface SettingsLayoutProps {
  children: ReactNode;
}
```

---

### 2. **SettingsSidebar**

**File**: `applications/shared/src/components/settings/SettingsSidebar.tsx`

**Purpose**: Navigation sidebar for settings sections

**Features**:
- Active route highlighting
- Icon + label navigation items
- Localized labels
- Hover states
- Footer hint text

**Navigation Items**:
- 🔊 Audio
- 🎵 Library
- ⚡ Playback
- 🎨 Appearance
- ⌨️ Shortcuts
- ℹ️ About

**Styling**:
- Active: Primary background with primary foreground text
- Inactive: Muted foreground with hover states
- Border-top separator for footer

---

### 3. **AudioSettingsPage**

**File**: `applications/shared/src/components/settings/AudioSettingsPage.tsx`

**Purpose**: Main audio settings page with pipeline configuration

**Features**:
- State management for all audio settings
- Tauri backend integration
- Auto-save to database
- Device enumeration per backend
- Reset to defaults functionality

**State Structure**:
```typescript
interface AudioSettings {
  backend: 'default' | 'asio' | 'jack';
  device_name: string | null;
  dsp_enabled: boolean;
  dsp_slots: (string | null)[];
  upsampling_quality: 'disabled' | 'fast' | 'balanced' | 'high' | 'maximum';
  upsampling_target_rate: 'auto' | number;
  volume_leveling_mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128';
  preload_enabled: boolean;
  buffer_size: 'auto' | number;
}
```

**Sections**:
1. Pipeline Visualization
2. Audio Driver (Backend + Device)
3. DSP Effects
4. Upsampling/Resampling
5. Volume Leveling
6. Buffer Settings

---

### 4. **PipelineVisualization**

**File**: `applications/shared/src/components/settings/audio/PipelineVisualization.tsx`

**Purpose**: Visual representation of audio processing stages

**Features**:
- Horizontal flow diagram
- Active/inactive stage indication
- Arrow connectors between stages
- Optional stage badges (OFF)
- Hover tooltips with descriptions
- Summary info text

**Pipeline Stages**:
```
FILE → DECODE → DSP → UPSAMPLE → LEVEL → VOLUME → OUTPUT
```

**Styling**:
- Active stages: Primary border, primary/10 background, shadow
- Inactive stages: Dashed border, muted background, 50% opacity
- Gradient background (primary/5 to primary/10)

**Visual States**:
- **Active**: Solid primary border, clear text
- **Inactive (Optional)**: Dashed border, "OFF" badge
- **Always Active**: FILE, DECODE, VOLUME, OUTPUT

---

### 5. **BackendSelector**

**File**: `applications/shared/src/components/settings/audio/BackendSelector.tsx`

**Purpose**: Select audio backend/driver (WASAPI, ASIO, JACK)

**Features**:
- Backend availability detection
- System default indication
- Device count per backend
- ASIO setup instructions when unavailable
- Selected state with checkmark

**Backend Information Displayed**:
- Name (e.g., "WASAPI", "ASIO")
- Description (feature summary)
- System default badge
- Availability status
- Device count
- Setup requirements (for ASIO)

**Styling**:
- Selected: Primary border (2px), primary/5 background
- Hover: Primary/50 border, muted/30 background
- Unavailable: 50% opacity, disabled cursor

---

### 6. **DeviceSelector**

**File**: `applications/shared/src/components/settings/audio/DeviceSelector.tsx`

**Purpose**: Select output audio device

**Features**:
- Device list for selected backend
- Speaker icon per device
- Sample rate and channel count display
- System default indication
- Active device highlight
- Empty state handling

**Device Information Displayed**:
- Device name (truncated if long)
- Sample rate (e.g., "96,000 Hz")
- Channel count (e.g., "2 channels")
- System default badge
- Active device indicator

**Styling**:
- Selected: Primary border, primary/5 background
- Hover: Primary/50 border, muted/30 background
- Active device shown in footer

---

### 7. **DspConfigurator**

**File**: `applications/shared/src/components/settings/audio/DspConfigurator.tsx`

**Purpose**: Configure DSP effects chain (4 slots)

**Features**:
- Enable/disable toggle
- 4 effect slots
- Effect picker dropdown
- Effect configuration button
- Add/remove effects
- Warning when disabled
- DSD bypass notice

**Available Effects**:
- Parametric EQ (5-band equalizer)
- Compressor (Dynamic range control)
- Crossfeed (Headphone spatialization)
- Convolution (Room correction / reverb)

**Styling**:
- Enabled: Toggle active (primary background)
- Empty slot: Dashed border "Add Effect" button
- Filled slot: Muted/30 background with effect info
- Expanded slot: Border-top separator with effect picker

---

### 8. **UpsamplingSettings**

**File**: `applications/shared/src/components/settings/audio/UpsamplingSettings.tsx`

**Purpose**: Configure upsampling/resampling quality and target rate

**Features**:
- 5 quality presets
- Target sample rate selection
- Advanced settings (collapsible)
- Bandwidth slider
- Anti-aliasing slider
- CPU usage indicators
- Info box with explanation

**Quality Presets**:
- **Disabled**: No resampling (0% CPU)
- **Fast**: Linear interpolation (<1% CPU)
- **Balanced**: Cubic interpolation (~2% CPU)
- **High**: r8brain algorithm (~5% CPU) [RECOMMENDED]
- **Maximum**: r8brain maximum (~10% CPU)

**Target Sample Rates**:
- Auto (Match Device)
- 88.2 kHz (×2 for 44.1k)
- 96 kHz
- 176.4 kHz (×4 for 44.1k)
- 192 kHz
- 352.8 kHz (×8 for 44.1k)
- 384 kHz

**Advanced Settings**:
- Bandwidth: 74% - 99.5% (slider)
- Anti-aliasing: -60dB to -120dB (slider)

**Styling**:
- Recommended badge on "High Quality"
- Collapsible advanced section
- Info box with blue accent
- Range sliders with labels

---

### 9. **VolumeLevelingSettings**

**File**: `applications/shared/src/components/settings/audio/VolumeLevelingSettings.tsx`

**Purpose**: Configure loudness normalization (ReplayGain / EBU R128)

**Features**:
- 4 normalization modes
- Pre-amp adjustment slider
- Prevent clipping checkbox
- Mode-specific info boxes
- Tag requirement warnings

**Normalization Modes**:
- **Disabled**: No normalization
- **ReplayGain (Track)**: Normalize each track (-18 LUFS)
- **ReplayGain (Album)**: Preserve album dynamics (-18 LUFS)
- **EBU R128**: Professional standard (-23 LUFS)

**Pre-amp Range**: -12 dB to +12 dB (0.5 dB steps)

**Info Boxes**:
- Mode explanation (blue accent)
- Tag requirement notice (amber accent)

**Styling**:
- Target level badges (muted background)
- Pre-amp slider with center indicator
- Checkbox for clipping prevention
- Contextual info boxes per mode

---

### 10. **BufferSettings**

**File**: `applications/shared/src/components/settings/audio/BufferSettings.tsx`

**Purpose**: Configure audio buffering and pre-loading

**Features**:
- Buffer size selection
- Latency indicators
- Pre-loading toggle
- Pros/cons list
- Memory usage estimate
- ASIO-specific notice

**Buffer Sizes**:
- **Auto**: System default (~50ms)
- **128 samples**: Ultra-low latency (~3ms @ 44.1kHz)
- **256 samples**: Low latency (~6ms @ 44.1kHz)
- **512 samples**: Balanced (~12ms @ 44.1kHz)
- **1024 samples**: Safe - RECOMMENDED (~23ms @ 44.1kHz)
- **2048 samples**: Maximum stability (~46ms @ 44.1kHz)

**Pre-loading Details**:
- **Pros**: Eliminates jitter, stable timing, Audirvana-style
- **Cons**: Memory usage (~30-60 MB), slight delay
- **Memory estimate**: 3-min track = ~31 MB

**Styling**:
- Dropdown with latency info
- Expanded pre-loading details when enabled
- Blue info box for audiophile tip
- ASIO notice in muted background

---

## Design System

### Color Scheme

**Primary Colors**:
- `primary`: Main accent color
- `primary-foreground`: Text on primary background
- `primary/5`, `primary/10`, `primary/20`: Alpha variants

**Background Colors**:
- `background`: Main background
- `card`: Card background
- `muted`: Muted background
- `muted/30`, `muted/50`: Alpha variants

**Border Colors**:
- `border`: Standard borders
- `primary/50`: Hover state borders

**Text Colors**:
- `foreground`: Primary text
- `muted-foreground`: Secondary text
- `destructive`: Error/warning text

**State Colors**:
- `blue-500/*`: Info boxes
- `amber-500/*`: Warning boxes

### Typography

**Headings**:
- h1: `text-3xl font-bold`
- h2: `text-xl font-semibold`
- h3: `text-sm font-semibold uppercase tracking-wide`

**Body Text**:
- Normal: `text-sm`
- Small: `text-xs`
- Description: `text-sm text-muted-foreground`

**Labels**:
- `text-sm font-medium`

### Spacing

**Sections**: `space-y-8` (2rem)
**Components**: `space-y-6` (1.5rem)
**Form Elements**: `space-y-3` (0.75rem)
**Inline**: `gap-2`, `gap-3`, `gap-4`

### Borders

**Standard**: `border border-border rounded-lg`
**Selected**: `border-2 border-primary`
**Dashed**: `border border-dashed`

### Transitions

**Standard**: `transition-all duration-150`
**Hover States**: `hover:bg-muted hover:border-primary/50`

---

## Integration Points

### Tauri Commands (To Be Implemented)

```typescript
// Backend Management
invoke<AudioBackend[]>('get_audio_backends')
invoke<AudioDevice[]>('get_audio_devices', { backend })
invoke('set_audio_device', { backend, deviceName })

// Settings Persistence
invoke<string | null>('get_user_setting', { key })
invoke('set_user_setting', { key, value })
```

### Database Storage

Settings stored in `user_settings` table:

```sql
key: 'audio.pipeline'
value: JSON.stringify({
  backend: 'default',
  device_name: 'Speakers',
  dsp_enabled: true,
  dsp_slots: ['eq', 'compressor', null, null],
  upsampling_quality: 'high',
  upsampling_target_rate: 'auto',
  volume_leveling_mode: 'replaygain_track',
  preload_enabled: true,
  buffer_size: 'auto'
})
```

---

## Accessibility

### Keyboard Navigation
- All interactive elements focusable
- Tab order follows visual flow
- Enter/Space activates buttons
- Escape closes dropdowns

### Screen Readers
- Semantic HTML elements
- ARIA labels where needed
- Descriptive text for icons
- Form labels properly associated

### Visual Accessibility
- Sufficient color contrast
- Focus indicators
- Hover states
- Icon + text labels (not icon-only)

---

## Responsive Design

### Breakpoints

**Desktop** (default):
- Sidebar: 14rem (56px)
- Content max-width: 56rem (4xl)

**Mobile** (not yet implemented):
- Sidebar collapses to hamburger menu
- Full-width content
- Stacked form elements

---

## File Structure

```
applications/shared/src/components/settings/
├── index.ts                              # Exports
├── SettingsLayout.tsx                     # Layout container
├── SettingsSidebar.tsx                    # Navigation sidebar
├── AudioSettingsPage.tsx                  # Main audio settings
└── audio/
    ├── PipelineVisualization.tsx         # Pipeline diagram
    ├── BackendSelector.tsx               # Backend selection
    ├── DeviceSelector.tsx                # Device selection
    ├── DspConfigurator.tsx               # DSP effects
    ├── UpsamplingSettings.tsx            # Upsampling config
    ├── VolumeLevelingSettings.tsx        # Volume leveling
    └── BufferSettings.tsx                # Buffer config
```

---

## Usage Example

```tsx
import { SettingsLayout, AudioSettingsPage } from '@soul-player/shared/settings';

// In your router
<Route path="/settings/audio" element={
  <SettingsLayout>
    <AudioSettingsPage />
  </SettingsLayout>
} />
```

---

## Next Steps

### Implementation Tasks

1. **Backend Integration** ✅ (backend.rs created)
   - [x] Create `AudioBackend` enum
   - [ ] Implement device enumeration
   - [ ] Implement backend switching

2. **Tauri Commands** 📋 TODO
   - [ ] `get_audio_backends()`
   - [ ] `get_audio_devices(backend)`
   - [ ] `set_audio_device(backend, deviceName)`
   - [ ] Settings persistence commands

3. **Routing** 📋 TODO
   - [ ] Update router with settings routes
   - [ ] Add sidebar navigation to desktop app
   - [ ] Migrate existing SettingsPage content

4. **Localization** 📋 TODO
   - [ ] Add translation keys for all labels
   - [ ] Add tooltips translation keys
   - [ ] Test with multiple languages

5. **Testing** 📋 TODO
   - [ ] Component unit tests
   - [ ] Integration tests with Tauri backend
   - [ ] Visual regression tests
   - [ ] Accessibility audit

---

## Design Philosophy

### Audiophile-First
- Professional terminology
- Detailed explanations
- Advanced options available but hidden by default
- Clear tradeoffs (quality vs performance)

### User-Friendly
- Recommended defaults highlighted
- Info boxes explain complex concepts
- Visual pipeline shows signal flow
- Auto-save eliminates "Apply" friction

### Performance-Conscious
- CPU usage indicators
- Latency measurements
- Memory estimates
- Clear guidance on presets

### Professional
- Clean, minimal design
- Consistent spacing and typography
- Proper information hierarchy
- Accessible and keyboard-friendly

---

## References

- **Audirvana UI**: Inspiration for pipeline visualization and settings layout
- **foobar2000 DSP**: Reference for effect chain management
- **Tailwind CSS**: Utility-first styling framework
- **Radix UI**: Accessible component primitives (for future enhancements)

---

**Last Updated**: 2026-01-09
**Status**: UI Design Complete ✅
**Next Phase**: Backend Integration
