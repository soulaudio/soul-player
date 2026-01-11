# E2E Test IDs Reference

This document lists all `data-testid` attributes required for E2E testing.

## Current Status: PARTIALLY IMPLEMENTED

**DSP Effect Components**: All DSP effect UI components have been implemented with `data-testid` attributes (see sections marked "IMPLEMENTED").

**Still needed**: Navigation components, page containers, and other audio settings components still need test IDs added.

## Priority Implementation Order

1. **Critical (Required for any test to pass)**:
   - `settings-button` - Without this, no test can navigate to settings
   - `settings-page` - Page container
   - `home-page`, `library-page`, `search-page` - Page containers

2. **High Priority (Core navigation)**:
   - `nav-library`, `home-button`, `search-button`, `queue-button`
   - Settings tabs: `settings-tab-*`

3. **Medium Priority (Audio settings tests)**:
   - Pipeline stages: `pipeline-stage-*`
   - Volume leveling: `volume-leveling-*`
   - DSP: `dsp-slot-*`

4. **Lower Priority (Advanced features)**:
   - Resampling, buffer, device selectors

## Navigation Components

### Header / MainLayout (`applications/shared/src/layouts/MainLayout.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `home-button` | Home button in header | Navigate to home page |
| `nav-library` | Library tab | Navigate to library |
| `search-button` | Search button | Open search page |
| `settings-button` | Settings gear button | Navigate to settings |
| `queue-button` | Queue button | Toggle queue sidebar |
| `import-button` | Import button | Open import dialog |
| `sources-button` | Sources button | Open sources dialog |

### Queue Sidebar (`applications/shared/src/components/QueueSidebar.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `queue-sidebar` | Sidebar container | Queue sidebar wrapper |
| `queue-close` | Close button | Close queue sidebar |
| `queue-track-{index}` | Track item | Individual track in queue |
| `queue-clear` | Clear button | Clear the queue |

## Page Components

### Home Page (`applications/desktop/src/pages/HomePage.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `home-page` | Page container | Home page wrapper |

### Library Page (`applications/desktop/src/pages/LibraryPage.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `library-page` | Page container | Library page wrapper |

### Search Page (`applications/desktop/src/pages/SearchPage.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `search-page` | Page container | Search page wrapper |
| `search-input` | Search input | Search text input field |
| `search-results` | Results container | Search results list |

### Settings Page (`applications/desktop/src/pages/SettingsPage.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `settings-page` | Page container | Settings page wrapper |
| `settings-tab-general` | Tab button | General settings tab |
| `settings-tab-library` | Tab button | Library settings tab |
| `settings-tab-sources` | Tab button | Sources settings tab |
| `settings-tab-audio` | Tab button | Audio settings tab |
| `settings-tab-shortcuts` | Tab button | Shortcuts settings tab |
| `settings-tab-about` | Tab button | About settings tab |

### Settings Tab Contents

| Test ID | Component | Description |
|---------|-----------|-------------|
| `general-settings-content` | Container | General settings content |
| `library-settings-content` | Container | Library settings content |
| `sources-settings-content` | Container | Sources settings content |
| `audio-settings-content` | Container | Audio settings content |
| `shortcuts-settings-content` | Container | Shortcuts settings content |
| `about-settings-content` | Container | About settings content |

## Audio Settings Components

### AudioSettingsPage (`applications/shared/src/components/settings/AudioSettingsPage.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `reset-audio-settings` | Button | Reset all audio settings |
| `confirm-reset-button` | Dialog button | Confirm reset action |

### Pipeline Stages (`applications/shared/src/components/settings/audio/PipelineStage.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `pipeline-stage-resampling` | Stage container | Resampling pipeline stage |
| `pipeline-stage-dsp` | Stage container | DSP effects pipeline stage |
| `pipeline-stage-volume-leveling` | Stage container | Volume leveling pipeline stage |
| `pipeline-stage-buffer` | Stage container | Buffer settings pipeline stage |
| `pipeline-stage-output` | Stage container | Audio output pipeline stage |

### Volume Leveling (`applications/shared/src/components/settings/audio/VolumeLevelingSettings.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `volume-leveling-disabled` | Button | Disabled mode option |
| `volume-leveling-replaygain-track` | Button | ReplayGain Track mode option |
| `volume-leveling-replaygain-album` | Button | ReplayGain Album mode option |
| `volume-leveling-ebu-r128` | Button | EBU R128 mode option |
| `preamp-slider` | Range input | Pre-amp adjustment slider |
| `prevent-clipping-checkbox` | Checkbox | Prevent clipping option |
| `analyze-all-button` | Button | Start library analysis |
| `stop-analysis-button` | Button | Stop analysis |

### DSP Config (`applications/shared/src/components/settings/audio/DspConfig.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `dsp-config` | Container | Main DSP config container |
| `effect-slot-{index}` | Container | Effect slot container (0-3) |
| `add-effect-btn-{index}` | Button | Add effect button for each slot |
| `edit-effect-btn-{index}` | Button | Edit button for each slot |
| `remove-effect-btn-{index}` | Button | Remove button for each slot |
| `effect-picker-{index}` | Container | Effect type picker dropdown for slot |
| `clear-all-btn` | Button | Clear all effects button |

### Parametric EQ Editor (`applications/shared/src/components/settings/audio/effects/ParametricEqEditor.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `parametric-eq-editor` | Container | Main editor container |
| `eq-band-{index}` | Container | Each band row |
| `eq-frequency-{index}` | Input | Frequency input for band |
| `eq-gain-{index}` | Range | Gain slider for band |
| `eq-q-{index}` | Input | Q control for band |
| `eq-add-band-btn` | Button | Add band button |
| `eq-preset-select` | Button | Preset dropdown trigger |

### Graphic EQ Editor (`applications/shared/src/components/settings/audio/effects/GraphicEqEditor.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `graphic-eq-editor` | Container | Main editor container |
| `graphic-eq-band-{index}` | Container | Each band slider (0-9) |
| `graphic-eq-preset-select` | Button | Preset dropdown trigger |
| `graphic-eq-reset-btn` | Button | Reset to flat button |

### Compressor Editor (`applications/shared/src/components/settings/audio/effects/CompressorEditor.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `compressor-editor` | Container | Main editor container |
| `compressor-threshold` | Range | Threshold control |
| `compressor-ratio` | Range | Ratio control |
| `compressor-attack` | Range | Attack control |
| `compressor-release` | Range | Release control |
| `compressor-knee` | Range | Knee control |
| `compressor-makeup` | Range | Makeup gain control |
| `compressor-preset-select` | Button | Preset dropdown trigger |

### Limiter Editor (`applications/shared/src/components/settings/audio/effects/LimiterEditor.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `limiter-editor` | Container | Main editor container |
| `limiter-ceiling` | Range | Ceiling/threshold control |
| `limiter-release` | Range | Release control |
| `limiter-preset-select` | Button | Preset dropdown trigger |

### Crossfeed Editor (`applications/shared/src/components/settings/audio/effects/CrossfeedEditor.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `crossfeed-editor` | Container | Main editor container |
| `crossfeed-preset-{name}` | Button | Preset card (natural, relaxed, meier, custom) |
| `crossfeed-level` | Range | Level slider |
| `crossfeed-cutoff` | Range | Cutoff frequency slider (advanced section) |

### Stereo Enhancer Editor (`applications/shared/src/components/settings/audio/effects/StereoEnhancerEditor.tsx`) - IMPLEMENTED

| Test ID | Component | Description |
|---------|-----------|-------------|
| `stereo-editor` | Container | Main editor container |
| `stereo-width` | Range | Width slider |
| `stereo-balance` | Range | Balance slider |
| `stereo-mid-gain` | Range | Mid gain control (advanced) |
| `stereo-side-gain` | Range | Side gain control (advanced) |
| `stereo-preset-select` | Select | Preset dropdown |

### Resampling Settings (`applications/shared/src/components/settings/audio/UpsamplingSettings.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `resampling-quality-selector` | Container | Quality options container |
| `resampling-quality-fast` | Button | Fast quality option |
| `resampling-quality-balanced` | Button | Balanced quality option |
| `resampling-quality-high` | Button | High quality option |
| `resampling-quality-maximum` | Button | Maximum quality option |
| `resampling-backend-selector` | Select | Backend selection dropdown |
| `resampling-target-rate` | Select | Target sample rate dropdown |

### Buffer Settings (`applications/shared/src/components/settings/audio/BufferSettings.tsx`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `buffer-settings` | Container | Buffer settings wrapper |
| `buffer-size-selector` | Select | Buffer size dropdown |
| `preload-enabled-checkbox` | Checkbox | Enable preloading |
| `crossfade-enabled-checkbox` | Checkbox | Enable crossfade |
| `crossfade-duration-slider` | Range | Crossfade duration |
| `crossfade-curve-selector` | Select | Crossfade curve type |

### Backend/Device Selectors (`applications/shared/src/components/settings/audio/`)

| Test ID | Component | Description |
|---------|-----------|-------------|
| `backend-selector` | Container | Audio backend selection |
| `backend-option-{name}` | Button | Backend option (default, asio, jack) |
| `device-selector` | Container | Audio device selection |
| `device-option-{name}` | Button | Device option |

## Implementation Notes

### Adding Test IDs to Components

Add `data-testid` attributes to React components like this:

```tsx
// Button example
<button
  data-testid="settings-button"
  onClick={() => navigate('/settings')}
  className="..."
>
  Settings
</button>

// Container example
<div data-testid="audio-settings-content" className="...">
  {/* content */}
</div>

// Dynamic IDs example (for lists)
{slots.map((slot, index) => (
  <button
    key={index}
    data-testid={`dsp-slot-${index}-add`}
    onClick={() => addEffect(index)}
  >
    Add Effect
  </button>
))}
```

### Priority Components

**High Priority** (required for basic test suite):
1. `MainLayout.tsx` - Navigation buttons
2. `SettingsPage.tsx` - Settings tabs and page
3. `VolumeLevelingSettings.tsx` - Volume leveling modes
4. `DspConfig.tsx` - DSP effect chain

**Medium Priority** (for expanded coverage):
1. `AudioSettingsPage.tsx` - Reset button
2. `UpsamplingSettings.tsx` - Quality options
3. `BufferSettings.tsx` - Buffer configuration
4. `QueueSidebar.tsx` - Queue interaction

**Lower Priority** (for comprehensive coverage):
1. Page containers (HomePage, LibraryPage, etc.)
2. Backend/Device selectors
3. Individual effect configurations

## Testing Considerations

1. **Unique IDs**: Ensure each `data-testid` is unique within the page context
2. **Dynamic Content**: For lists, include index or unique identifier in test ID
3. **Conditional Rendering**: Test IDs should be on elements that are always present in the DOM when visible
4. **Performance**: `data-testid` attributes have no performance impact in production, but can be stripped in production builds if desired
