/**
 * VirtualizedGrid - High-performance virtualized grid with infinite scroll
 *
 * Uses TanStack Virtual for efficient rendering of large datasets.
 * Only renders visible items in viewport for 60fps scrolling performance.
 *
 * @see https://tanstack.com/virtual/latest
 * @see https://tanstack.com/virtual/latest/docs/framework/react/examples/infinite-scroll
 * @see https://borstch.com/blog/development/infinite-scroll-made-easy-with-tanstack-virtual-a-step-by-step-react-guide
 */

import { useEffect, ReactNode } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext'

interface VirtualizedGridProps<T> {
  /** All items to display */
  items: T[]
  /** Total count (for proper scrollbar sizing) */
  totalCount: number
  /** Render function for each item */
  renderItem: (item: T, index: number) => ReactNode
  /** Grid columns class (e.g., 'grid-cols-2 sm:grid-cols-3') */
  gridClass: string
  /** Number of columns (needed for row calculation) */
  columnCount: number
  /** Estimated row height in pixels (card height + gap) */
  rowHeight: number
  /** Loading state */
  isLoading?: boolean
  /** Callback when reaching end (for loading more) */
  onLoadMore?: () => void
  /** Loading threshold (trigger load when this many rows from bottom) */
  loadMoreThreshold?: number
}

export function VirtualizedGrid<T>({
  items,
  totalCount,
  renderItem,
  gridClass,
  columnCount,
  rowHeight,
  isLoading,
  onLoadMore,
  loadMoreThreshold = 5,
}: VirtualizedGridProps<T>) {
  // Use the scroll container managed by LibraryPageLayout so we don't create
  // a nested overflow-y-auto that breaks scroll events and doubles padding.
  const { scrollContainerRef } = useScrollVisibility()

  // Calculate number of rows based on total count and columns
  const rowCount = Math.ceil(totalCount / columnCount)

  // Get items for a specific row
  const getRowItems = (rowIndex: number): T[] => {
    const startIndex = rowIndex * columnCount
    const endIndex = Math.min(startIndex + columnCount, items.length)
    return items.slice(startIndex, endIndex)
  }

  // Setup virtualizer — uses LibraryPageLayout's scroll container as scroll element
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => rowHeight,
    overscan: 3,
  })

  const virtualItems = virtualizer.getVirtualItems()

  // Load more when approaching end
  useEffect(() => {
    if (!onLoadMore || isLoading) return

    const lastItem = virtualItems[virtualItems.length - 1]
    if (!lastItem) return

    // Trigger load more when within threshold rows of the end
    if (lastItem.index >= rowCount - loadMoreThreshold) {
      onLoadMore()
    }
  }, [virtualItems, rowCount, loadMoreThreshold, onLoadMore, isLoading])

  // Render directly — no own scroll wrapper. LibraryPageLayout's scroll container
  // (scrollContainerRef) is the scroll element, so scroll events and padding are
  // handled there. This avoids nested overflow-y-auto and duplicate pr-6/pb-6.
  return (
    <div
      style={{
        height: `${virtualizer.getTotalSize()}px`,
        width: '100%',
        position: 'relative',
      }}
    >
      {virtualItems.map((virtualRow) => {
        const rowItems = getRowItems(virtualRow.index)

        return (
          <div
            key={virtualRow.index}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
              {rowItems.map((item, colIndex) => {
                const absoluteIndex = virtualRow.index * columnCount + colIndex
                return (
                  <div key={absoluteIndex}>
                    {renderItem(item, absoluteIndex)}
                  </div>
                )
              })}
            </div>
          </div>
        )
      })}

      {/* Loading indicator at bottom */}
      {isLoading && (
        <div className="flex justify-center py-8">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full" />
        </div>
      )}
    </div>
  )
}
