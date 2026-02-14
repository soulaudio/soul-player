# Seek Implementation Simplification Roadmap

## Overview

Soul Player's seek implementation can be simplified by removing ~230 lines of unnecessary code without sacrificing UX.

---

## Priority 1: Remove Progress Interpolation (128 lines, HIGH IMPACT)

### What to Remove
- Delete: `applications/shared/src/hooks/useInterpolatedProgress.ts` (128 lines)
- Delete: `applications/shared/src/hooks/__tests__/useInterpolatedProgress.test.ts` (80+ lines)

### Changes in ProgressBar.tsx (Lines 19-26)
```typescript
// REMOVE:
import { useInterpolatedProgress } from '../../hooks/useInterpolatedProgress';
const interpolatedProgress = useInterpolatedProgress();
const { progress, duration } = interpolatedProgress;

// REPLACE WITH:
const { progress, duration } = usePlayerStore(state => ({
  progress: state.progress,
  duration: state.duration,
}));
```

### Why Safe
- Spotify doesn't interpolate, users don't notice difference
- Optimistic update handles seek feedback anyway
- Reduces complexity without UX regression

**Effort: 30 minutes | Risk: Low**

---

## Priority 2: Remove Seeking Spinner (15 lines, LOW IMPACT)

### Changes in useSeekBar.ts
Remove lines 38-39 and simplify handleSeek:
```typescript
// REMOVE: const [isSeeking, setIsSeeking] = useState(false);

// SIMPLIFY:
const handleSeek = useCallback((position: number) => {
  const { duration } = usePlayerStore.getState();
  const clampedPosition = Math.max(0, Math.min(position, duration - 0.1));
  usePlayerStore.setState({ progress: (clampedPosition / duration) * 100 });
  commands.seek(clampedPosition).catch(error => {
    debug.error('[useSeekBar] Seek failed:', error);
  });
}, [commands]);
```

### Changes in ProgressBar.tsx
Remove lines 148-163 (seeking spinner block)

### Why Safe
- 120ms timer doesn't match actual backend completion
- Optimistic update gives instant feedback
- Simpler and more correct

**Effort: 20 minutes | Risk: Low**

---

## Priority 3: Remove Ignore Window (5 lines, MINIMAL IMPACT)

### Changes in TauriPlayerCommandsProvider.tsx

Remove line 27:
```typescript
const IGNORE_WINDOW_MS = 120;
```

Remove lines 38-39:
```typescript
const ignoringPositionUpdatesRef = useRef(false);
const ignoreTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
```

Remove line 313 check:
```typescript
if (ignoringPositionUpdatesRef.current) return;
```

Simplify seek (502-518):
```typescript
async seek(position: number) {
  await invoke('seek_to', { position });
}
```

### Why Safe
- Optimistic update already prevents visual issues
- Fragile band-aid replaced by simpler approach

**Effort: 10 minutes | Risk: Low**

---

## Priority 4: Simplify State Management (50 lines, MEDIUM IMPACT)

### Changes in ProgressBar.tsx

Replace lines 29-32:
```typescript
// BEFORE:
const [isDragging, setIsDragging] = useState(false);
const [dragPosition, setDragPosition] = useState<number | null>(null);
const [isHovering, setIsHovering] = useState(false);

// AFTER:
type ProgressState = 'idle' | 'dragging' | 'seeking';
const [state, setState] = useState<ProgressState>('idle');
const [dragPosition, setDragPosition] = useState<number | null>(null);
```

Update cursor (line 114):
```typescript
cursor: state === 'dragging' ? 'grabbing' : state === 'seeking' ? 'wait' : 'pointer',
```

Replace handle rendering (122-175) with single block:
```typescript
{(state === 'dragging' || state === 'seeking') && (
  <ProgressHandle state={state} position={displayProgress} />
)}
```

Remove hover handlers (119-120):
```typescript
// REMOVE: onMouseEnter and onMouseLeave
```

**Effort: 45 minutes | Risk: Medium**

---

## Results

**Total saved: ~278 lines**

| Phase | Time | Lines | Risk |
|-------|------|-------|------|
| Remove interpolation | 30m | 208 | Low |
| Remove spinner | 20m | 15 | Low |
| Remove ignore window | 10m | 5 | Low |
| Simplify state | 45m | 50 | Medium |
| **TOTAL** | **105m** | **278** | **Low** |

---

## Testing

After each phase:
```bash
cargo xtask check precommit
yarn test:shared
```

Manual testing:
- [ ] Click to seek
- [ ] Drag to seek
- [ ] Progress updates during playback
- [ ] Seek completes quickly
- [ ] No errors

