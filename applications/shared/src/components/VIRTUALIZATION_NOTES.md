# Virtualization Strategy for Responsive Grids

## Current Approach

We use a **hybrid strategy** for rendering media grids (albums, artists, playlists):

### For Collections ≤ 200 items:
- **Use regular CSS grid** (non-virtualized)
- Handles responsive layouts naturally
- No need to estimate row heights
- Performance is good for collections under ~500 items

### For Collections > 200 items:
- **Use TanStack Virtual** with fixed `rowHeight` estimates
- Fixed values per scale: Artists `[220, 280, 340, 400]`, Albums `[240, 300, 360, 420]`
- Albums +20px vs Artists due to 2-line subtitle (`line-clamp-2` vs `line-clamp-1`)

## Why This Approach?

### Problem with Fixed rowHeight:
Responsive grids have **variable card widths** at different breakpoints:
- Mobile (2 cols): Cards are ~300px wide
- Tablet (4 cols): Cards are ~200px wide
- Desktop (6 cols): Cards are ~150px wide

Since images are `aspect-square`, card height = card width + text height. This creates a mismatch between the fixed `rowHeight` estimate and actual card height, causing:
- **Gaps**: When actual card < estimated rowHeight
- **Overlaps**: When actual card > estimated rowHeight

### Solution Options Research:

1. **Disable virtualization** (Chosen for ≤200 items)
   - ✅ Simple and robust
   - ✅ Perfect responsive behavior
   - ✅ Good performance for most libraries
   - Source: [TanStack Virtual Discussion #732](https://github.com/TanStack/virtual/discussions/732)

2. **Dynamic measurement** with `measureElement` (For future >200 items optimization)
   - ✅ Accurate heights
   - ❌ More complex
   - ❌ Requires re-measuring on resize
   - Source: [TanStack Virtual Variable Example](https://tanstack.com/virtual/latest/docs/framework/react/examples/variable)

3. **CellMeasurer pattern** from react-virtualized
   - ✅ Most accurate
   - ❌ Renders items twice (measure + actual)
   - ❌ Performance overhead
   - Source: [CellMeasurer docs](https://github.com/bvaughn/react-virtualized/blob/master/docs/CellMeasurer.md)

4. **Dynamic rowHeight calculation**
   - ❌ Tried and failed
   - ❌ Container width estimation is unreliable
   - ❌ Doesn't account for all CSS factors

## Future Improvements

If we need better virtualization for >200 items:

### Option A: Implement `measureElement`
```typescript
const virtualizer = useVirtualizer({
  count: rowCount,
  getScrollElement: () => scrollElement,
  estimateSize: () => rowHeight,
  measureElement: (element) => element.getBoundingClientRect().height,
  overscan: 3,
})
```

### Option B: Use ResizeObserver
```typescript
useEffect(() => {
  const observer = new ResizeObserver(() => {
    virtualizer.measure()
  })
  // Observe grid container
  if (gridRef.current) observer.observe(gridRef.current)
  return () => observer.disconnect()
}, [])
```

### Option C: Increase threshold to 500
- Most libraries won't hit this
- Virtual scrolling only for truly massive collections

## Consistency Requirements

**CRITICAL**: Album and Artist pages must use:
- ✅ Same `gridClass` responsive breakpoints
- ✅ Same virtualization threshold
- ✅ Same layout components (`LibraryPageLayout`, `VirtualizedGrid`)
- ✅ Same card structure (both use `MediaCard`)
- ✅ Fixed subtitle heights (`min-h-[2.625rem]` for albums, consistent single-line for artists)

## References

- [TanStack Virtual Discussions](https://github.com/TanStack/virtual/discussions/732)
- [Borstch: Virtualized Grid with Dynamic Sizes](https://borstch.com/snippet/creating-a-virtualized-grid-with-dynamic-sizes)
- [React Virtualized CellMeasurer](https://github.com/bvaughn/react-virtualized/blob/master/docs/CellMeasurer.md)
- [Smart Grid Article](https://medium.com/@mukuljainx/smart-grid-lightweight-alternate-to-ag-grid-9d8c3d38c351)

## Last Updated
2026-02-11 - Increased threshold from 100 to 200 items based on research
