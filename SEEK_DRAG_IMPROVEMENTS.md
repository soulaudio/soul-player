# Seek Bar Drag Improvements

## Issues Fixed

### 1. Dragging Outside Window/Screen
**Problem:** When dragging the seek bar and moving the mouse outside the window, the drag would get stuck. The mouseup event wouldn't fire, leaving the drag state active.

**Solution:**
Added multiple event listeners to handle edge cases:
- `document.mouseleave` - Detects when mouse leaves the document
- `window.blur` - Detects when window loses focus (e.g., Alt+Tab during drag)
- Boundary check: Only triggers if mouse actually leaves window boundaries

**Code:**
```typescript
const handleMouseLeave = (e: MouseEvent) => {
  // Only trigger if mouse actually leaves the window
  if (e.clientY <= 0 || e.clientX <= 0 ||
      e.clientX >= window.innerWidth || e.clientY >= window.innerHeight) {
    console.log('[ProgressBar] Mouse left window - ending seek');
    handleMouseUp();
  }
};

document.addEventListener('mouseleave', handleMouseLeave);
window.addEventListener('blur', handleMouseUp);
```

### 2. Transition Animation During Drag
**Problem:** The progress bar had a CSS transition (`transition-all duration-100`) that was active during dragging. This caused:
- Laggy/delayed visual feedback when dragging
- Progress bar "chasing" the mouse instead of following instantly

**Expected Behavior:**
- **While dragging:** Immediate updates, no transition (instant feedback)
- **When clicking:** Smooth transition to target position
- **During playback:** Smooth transition as song plays

**Solution:**
Conditionally apply the transition class based on `isDragging` state:

```typescript
<div
  className={`absolute inset-y-0 left-0 bg-primary rounded-full ${
    isDragging ? '' : 'transition-all duration-100'
  }`}
  style={{ width: `${Math.max(0, Math.min(100, displayProgress))}%` }}
/>
```

**Result:**
- Dragging: No transition class → instant updates
- Not dragging: Has transition class → smooth animation

### 3. Seek Handle Pointer Events
**Bonus Fix:** Added `pointerEvents: 'none'` to the seek handle to prevent it from interfering with mouse events on the progress bar.

## Files Modified

### `applications/desktop/src/components/player/ProgressBar.tsx`

**Changes:**
1. Added `handleMouseLeave` to detect mouse leaving window
2. Added `window.blur` listener for focus loss
3. Centralized cleanup function that removes all listeners
4. Conditionally apply transition class based on `isDragging`
5. Added `pointerEvents: 'none'` to seek handle

**Event Listeners Added:**
```typescript
document.addEventListener('mousemove', handleMouseMove);
document.addEventListener('mouseup', handleMouseUp);
document.addEventListener('mouseleave', handleMouseLeave);  // NEW
window.addEventListener('blur', handleMouseUp);             // NEW
```

**Cleanup:**
All listeners are properly removed in a centralized cleanup function to prevent memory leaks.

## Testing

### Test Drag Outside Window:
1. Start playing a song
2. Click and hold on the progress bar
3. While holding, drag mouse outside the window
4. Release mouse button
5. **Expected:** Seek completes, drag state ends

### Test Transition Behavior:
1. **During drag:**
   - Click and drag the progress bar
   - **Expected:** Bar follows mouse instantly (no lag)

2. **When clicking:**
   - Click on a position on the progress bar
   - **Expected:** Bar smoothly transitions to clicked position

3. **During playback:**
   - Let song play normally
   - **Expected:** Bar smoothly advances with the music

### Test Focus Loss:
1. Start dragging the seek bar
2. While dragging, press Alt+Tab to switch windows
3. **Expected:** Drag ends, seek completes

## Edge Cases Handled

1. **Mouse leaves top of window:** Boundary check catches `clientY <= 0`
2. **Mouse leaves bottom:** Boundary check catches `clientY >= window.innerHeight`
3. **Mouse leaves left/right:** Boundary checks catch `clientX` boundaries
4. **Window loses focus:** `blur` event listener handles this
5. **Multiple rapid drags:** Cleanup function removes old listeners before adding new ones
6. **Component unmount during drag:** `useEffect` cleanup removes listeners

## Performance Impact

Minimal:
- Event listeners are only active during drag (not constantly)
- All listeners properly cleaned up
- No memory leaks
- Transition removal during drag improves visual performance

## Related Issues Fixed

This also resolves:
- Seek bar appearing "laggy" during drag
- Getting stuck in drag mode
- Visual jank during rapid seeking
