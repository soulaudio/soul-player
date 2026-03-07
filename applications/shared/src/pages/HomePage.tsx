/**
 * Shared HomePage - Responsive Square Grid
 * Perfect 1:1 square cells that fill the viewport
 */

import { useEffect, useRef, useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { BackendAlbum } from '../contexts/BackendContext'
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext'
import { AlbumCard } from '../components/AlbumCard'
import { SkeletonGrid } from '../components/SkeletonGrid'
import { categorizeAlbumsByPlayback, selectAlbumsFromOrderedIds, shuffle } from '../lib/homePageUtils'
import { useAlbums } from '../hooks/queries/useAlbumQueries'
import { useRecentContexts } from '../hooks/queries/useLibraryQueries'
import { debounce } from '../utils/debounce'
import { debug } from '../utils/debug';

// Section type definition
interface BentoSection {
  id: string
  title: string
  type: 'wide' | 'medium' | 'square' | 'vertical' | 'small'
  rows: number // Total rows (including heading)
  cols: number // Total columns
  albumSize: 2 | 3 | 4 // 2x2, 3x3, or 4x4 albums
  gridColumn: string // CSS grid-column value
  gridRow: string // CSS grid-row value
}

export function HomePage() {
  // Toggle to show/hide debug grid lines
  const SHOW_DEBUG_GRID = false

  const { t } = useTranslation()
  const { setShowHeader } = useScrollVisibility()
  const containerRef = useRef<HTMLDivElement>(null)
  const [gridDimensions, setGridDimensions] = useState({ rows: 0, cols: 0 })

  // Performance: Defer expensive queries until component is visible
  // This prevents blocking the initial app render on macOS/Linux
  const [shouldLoadData, setShouldLoadData] = useState(false)

  useEffect(() => {
    // Defer loading slightly to let UI render first
    const timer = setTimeout(() => setShouldLoadData(true), 100)
    return () => clearTimeout(timer)
  }, [])

  // Fetch data using React Query hooks with deferred loading
  const { data: allAlbums = [], isLoading: albumsLoading } = useAlbums({
    enabled: shouldLoadData,
  })
  // Performance: Reduced from 100 to 30 - we only need recent history for home page sections
  const { data: contexts = [], isLoading: contextsLoading } = useRecentContexts(30, {
    enabled: shouldLoadData,
  })

  const isLoading = !shouldLoadData || albumsLoading || contextsLoading

  // Reset scroll visibility when HomePage mounts (fixes header hidden state from library pages)
  useEffect(() => {
    setShowHeader(true)
  }, [setShowHeader])

  // Performance: Create a Map for O(1) album lookups instead of O(n) array.find()
  const albumMap = useMemo(() => new Map(allAlbums.map(a => [a.id, a])), [allAlbums])

  // Categorize albums based on playback contexts
  const { recentAlbums, recentAlbumIds, timeCapsuleAlbumIds, onRepeatAlbumIds } = useMemo(() => {
    if (!allAlbums.length || !contexts.length) {
      return {
        recentAlbums: [],
        recentAlbumIds: new Set<number>(),
        timeCapsuleAlbumIds: new Set<number>(),
        onRepeatAlbumIds: [],
      }
    }

    // Use utility function to categorize albums
    const categories = categorizeAlbumsByPlayback(contexts)

    // Performance: Build recent albums array using Map lookup (O(1)) instead of array.find() (O(n))
    // This avoids O(n²) complexity when processing many recent albums
    const recentAlbumData: BackendAlbum[] = []
    const seenAlbumIds = new Set<number>()

    for (const albumId of categories.recentAlbumIds) {
      if (!seenAlbumIds.has(albumId)) {
        const album = albumMap.get(albumId)
        if (album) {
          recentAlbumData.push(album)
          seenAlbumIds.add(albumId)
        }
      }
    }

    return {
      recentAlbums: recentAlbumData,
      recentAlbumIds: categories.recentAlbumIds,
      timeCapsuleAlbumIds: categories.timeCapsuleAlbumIds,
      onRepeatAlbumIds: categories.onRepeatAlbumIds,
    }
  }, [allAlbums, contexts, albumMap])

  // TODO: Implement album navigation - should either navigate to album detail page
  // or start playing the album immediately. Consolidates 5 identical TODOs below.
  const handleAlbumClick = (albumId: number) => {
    debug.log('Clicked album:', albumId)
    // Future: navigate to /album/:id or trigger playback
  }

  useEffect(() => {
    const calculateGrid = () => {
      if (!containerRef.current) return

      const container = containerRef.current
      const width = container.clientWidth
      const height = container.clientHeight

      // Target cell size (adjust this value to change grid density)
      const targetCellSize = 50
      const gap = 8 // 0.5rem = 8px

      // Calculate how many columns fit
      let cols = Math.floor((width + gap) / (targetCellSize + gap))

      // Calculate actual cell size based on columns
      let actualCellSize = (width - (cols - 1) * gap) / cols

      // Remove columns if they would overflow - keep reducing until no overflow
      while (cols > 1 && (actualCellSize * cols + (cols - 1) * gap) > width) {
        cols--
        actualCellSize = (width - (cols - 1) * gap) / cols
      }

      // Calculate how many rows fit with the actual cell size (rows use same cell size as columns for 1:1 ratio)
      let rows = Math.floor((height + gap) / (actualCellSize + gap))

      // Remove rows if they would overflow - keep reducing until no overflow
      while (rows > 1 && (actualCellSize * rows + (rows - 1) * gap) > height) {
        rows--
      }

      setGridDimensions({ rows: Math.max(1, rows), cols: Math.max(1, cols) })
    }

    // Performance: Debounce resize calculations to prevent excessive recalculations
    // 150ms is enough to smooth out window resize but still feel responsive
    const debouncedCalculateGrid = debounce(calculateGrid, 150)

    calculateGrid() // Initial calculation (immediate)

    const resizeObserver = new ResizeObserver(debouncedCalculateGrid)
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current)
    }

    return () => resizeObserver.disconnect()
  }, [isLoading])

  const totalCells = gridDimensions.rows * gridDimensions.cols

  const { rows, cols } = gridDimensions
  const bottomSectionRows = 5 // 1 heading + 4 rows (flexible for 2×2, 3×3, or 4×4)

  // Track all used album IDs to prevent duplicates
  const usedAlbumIds = useRef<Set<number>>(new Set())

  // Get albums for "Jump back into" section - use recent albums if available
  // Generate larger pool (30) to support bigger viewports and resizes
  const jumpBackAlbums = useMemo(() => {
    usedAlbumIds.current.clear() // Reset on recalculation

    let albums: BackendAlbum[] = []
    if (recentAlbums.length === 0 && allAlbums.length > 0) {
      // Fallback to random albums if no recent history
      // Use Fisher-Yates shuffle (O(n)) instead of sort-based shuffle (O(n log n))
      const shuffled = shuffle(allAlbums)
      albums = shuffled.slice(0, Math.min(30, allAlbums.length))
    } else {
      // Return up to 30 recent albums - layout will show what fits
      albums = recentAlbums.slice(0, 30)
    }

    // Mark as used
    albums.forEach(album => usedAlbumIds.current.add(album.id))
    return albums
  }, [recentAlbums, allAlbums]) // Performance: Removed cols - not used in computation

  // Get albums for "On repeat" section - albums played 3+ times in last 2 weeks
  // Generate larger pool (30) to support bigger viewports and resizes
  const onRepeatAlbums = useMemo(() => {
    if (allAlbums.length === 0 || onRepeatAlbumIds.length === 0) return []

    // Use ordered selection to maintain play count order - generate pool of 30
    const selected = selectAlbumsFromOrderedIds(allAlbums, onRepeatAlbumIds, 30, usedAlbumIds.current)
    return selected
  }, [allAlbums, onRepeatAlbumIds]) // Performance: Removed cols - not used in computation

  // Get albums for "Time capsule" section - albums played 2-6 months ago but not recently
  // Generate larger pool (30) to support bigger viewports and resizes
  const timeCapsuleAlbums = useMemo(() => {
    if (allAlbums.length === 0 || timeCapsuleAlbumIds.size === 0) return []

    // Get albums from time capsule IDs
    const capsuleAlbums = allAlbums.filter(album => timeCapsuleAlbumIds.has(album.id))

    // Shuffle and take up to 30 - layout will show what fits
    // Use Fisher-Yates shuffle (O(n)) instead of sort-based shuffle (O(n log n))
    const shuffled = shuffle(capsuleAlbums)
    const selected = shuffled.slice(0, Math.min(30, capsuleAlbums.length))

    // Mark as used
    selected.forEach(album => usedAlbumIds.current.add(album.id))
    return selected
  }, [allAlbums, timeCapsuleAlbumIds]) // Performance: Removed cols - not used in computation

  // Get albums for "Don't forget about" section - albums NOT in recent history
  // Generate larger pool (30) to support bigger viewports and resizes
  const forgottenAlbums = useMemo(() => {
    if (allAlbums.length === 0) return []

    // Filter out recently played albums and time capsule albums
    const nonRecentAlbums = allAlbums.filter(
      album => !recentAlbumIds.has(album.id) && !usedAlbumIds.current.has(album.id)
    )

    // Shuffle and take up to 30 - layout will show what fits
    // Use Fisher-Yates shuffle (O(n)) instead of sort-based shuffle (O(n log n))
    const shuffled = shuffle(nonRecentAlbums)
    const selected = shuffled.slice(0, Math.min(30, nonRecentAlbums.length))

    // Mark as used
    selected.forEach(album => usedAlbumIds.current.add(album.id))
    return selected
  }, [allAlbums, recentAlbumIds, timeCapsuleAlbums]) // Performance: Removed cols - not used in computation

  // Get random albums for the bottom section - exclude already used albums
  // Generate larger pool (100) to support very wide viewports and resizes
  const crateDiggingAlbumsPool = useMemo(() => {
    if (allAlbums.length === 0) return []

    // Generate fixed pool of 100 albums to handle any viewport size
    // This avoids recalculation on window resize
    const maxAlbumsToGenerate = 100

    // Filter out already used albums
    const availableAlbums = allAlbums.filter(album => !usedAlbumIds.current.has(album.id))
    // Use Fisher-Yates shuffle (O(n)) instead of sort-based shuffle (O(n log n))
    const shuffled = shuffle(availableAlbums)
    const selected = shuffled.slice(0, Math.min(maxAlbumsToGenerate, availableAlbums.length))

    // Mark as used
    selected.forEach(album => usedAlbumIds.current.add(album.id))
    return selected
  }, [allAlbums, jumpBackAlbums, onRepeatAlbums, timeCapsuleAlbums, forgottenAlbums]) // Performance: Removed cols - not used in computation

  // Slice pool to actual needed size during render (not in memo)
  const albumRows = bottomSectionRows - 1 // Subtract heading row
  const albumsPerRow = Math.floor(cols / 2) // 2×2 albums
  const albumRowsAvailable = Math.floor(albumRows / 2)
  const albumsNeeded = albumsPerRow * albumRowsAvailable
  const crateDiggingAlbums = crateDiggingAlbumsPool.slice(0, albumsNeeded)

  // Generate bento sections based on grid dimensions
  const sections = useMemo(() => {
    if (rows === 0 || cols === 0) return []

    const sections: BentoSection[] = []
    const occupied: boolean[][] = Array.from({ length: rows }, () => Array(cols).fill(false))

    // Helper: Check if area is available
    const isAvailable = (startRow: number, startCol: number, numRows: number, numCols: number): boolean => {
      if (startRow + numRows > rows || startCol + numCols > cols) return false
      for (let r = startRow; r < startRow + numRows; r++) {
        for (let c = startCol; c < startCol + numCols; c++) {
          if (occupied[r][c]) return false
        }
      }
      return true
    }

    // Helper: Mark area as occupied
    const markOccupied = (startRow: number, startCol: number, numRows: number, numCols: number) => {
      for (let r = startRow; r < startRow + numRows; r++) {
        for (let c = startCol; c < startCol + numCols; c++) {
          occupied[r][c] = true
        }
      }
    }

    // 1. Top-left section: "Jump back into" - show 2-6 recent albums
    // Layouts: 2×2 (4 albums), 3×2 (6 albums), or 2×1 (2 albums) depending on space
    let jumpBackRows = 5
    let jumpBackCols = 8

    if (cols >= 12 && rows >= 9) {
      // XL: 3 albums wide × 2 tall with 2×2 albums = 6 albums (3×2 layout)
      jumpBackRows = 5 // 1 heading + 4 rows
      jumpBackCols = 6
      sections.push({
        id: 'jump-back',
        title: 'Jump back into',
        type: 'square',
        rows: jumpBackRows,
        cols: jumpBackCols,
        albumSize: 2,
        gridColumn: `1 / ${jumpBackCols + 1}`,
        gridRow: `1 / ${jumpBackRows + 1}`,
      })
      markOccupied(0, 0, jumpBackRows, jumpBackCols)
    } else if (cols >= 8 && rows >= 9) {
      // Large: 2 albums wide × 2 tall with 2×2 albums = 4 albums (2×2 layout)
      jumpBackRows = 5 // 1 heading + 4 rows
      jumpBackCols = 4
      sections.push({
        id: 'jump-back',
        title: 'Jump back into',
        type: 'square',
        rows: jumpBackRows,
        cols: jumpBackCols,
        albumSize: 2,
        gridColumn: `1 / ${jumpBackCols + 1}`,
        gridRow: `1 / ${jumpBackRows + 1}`,
      })
      markOccupied(0, 0, jumpBackRows, jumpBackCols)
    } else if (cols >= 6 && rows >= 7) {
      // Medium: 3 albums wide × 1 tall with 2×2 albums = 3 albums (3×1 layout)
      jumpBackRows = 3 // 1 heading + 2 rows
      jumpBackCols = 6
      sections.push({
        id: 'jump-back',
        title: 'Jump back into',
        type: 'square',
        rows: jumpBackRows,
        cols: jumpBackCols,
        albumSize: 2,
        gridColumn: `1 / ${jumpBackCols + 1}`,
        gridRow: `1 / ${jumpBackRows + 1}`,
      })
      markOccupied(0, 0, jumpBackRows, jumpBackCols)
    } else if (cols >= 4 && rows >= 5) {
      // Small: 2 albums wide × 1 tall with 2×2 albums = 2 albums (2×1 layout)
      jumpBackRows = 3
      jumpBackCols = 4
      sections.push({
        id: 'jump-back',
        title: 'Jump back into',
        type: 'square',
        rows: jumpBackRows,
        cols: jumpBackCols,
        albumSize: 2,
        gridColumn: `1 / ${jumpBackCols + 1}`,
        gridRow: `1 / ${jumpBackRows + 1}`,
      })
      markOccupied(0, 0, jumpBackRows, jumpBackCols)
    }

    // 2. Bottom section - smaller albums for browsing/discovery feel
    if (rows >= bottomSectionRows) {
      const startRow = rows - bottomSectionRows

      // Use smaller album sizes to show more options for crate digging
      let albumSize: 2 | 3 | 4 = 2
      if (cols >= 18) {
        albumSize = 2 // Many 2×2 albums for wide grids
      } else if (cols >= 10) {
        albumSize = 2 // 2×2 albums
      } else {
        albumSize = 2 // Always 2×2 for crate digging
      }

      sections.push({
        id: 'bottom',
        title: 'Do some crate digging',
        type: 'wide',
        rows: bottomSectionRows,
        cols: cols,
        albumSize,
        gridColumn: `1 / ${cols + 1}`,
        gridRow: `${startRow + 1} / ${rows + 1}`,
      })
      markOccupied(startRow, 0, bottomSectionRows, cols)
    }

    // 3. Fill remaining space - focus on fewer, larger albums (premium feel)
    // Prefer 4×4, then 3×3, then 2×2
    const generateTemplates = () => {
      const templates = []

      // XL sections with 4×4 albums (1-3 albums)
      if (cols >= 16) {
        templates.push({ type: 'wide' as const, rows: 9, cols: cols, albumSize: 4 as const }) // 2× albums wide
        templates.push({ type: 'wide' as const, rows: 5, cols: cols, albumSize: 4 as const }) // 1× albums wide
        templates.push({ type: 'wide' as const, rows: 9, cols: 12, albumSize: 4 as const }) // 2×2 = 4 albums
        templates.push({ type: 'wide' as const, rows: 5, cols: 12, albumSize: 4 as const }) // 1×3 = 3 albums
      }
      if (cols >= 12) {
        templates.push({ type: 'wide' as const, rows: 9, cols: cols, albumSize: 4 as const }) // 2× albums wide
        templates.push({ type: 'wide' as const, rows: 5, cols: cols, albumSize: 4 as const }) // 1× albums wide
        templates.push({ type: 'medium' as const, rows: 9, cols: 12, albumSize: 4 as const }) // 2×3 = 6 albums
        templates.push({ type: 'medium' as const, rows: 5, cols: 8, albumSize: 4 as const }) // 1×2 = 2 albums
      }

      // Large sections with 3×3 albums (2-4 albums)
      if (cols >= 12) {
        templates.push({ type: 'wide' as const, rows: 7, cols: cols, albumSize: 3 as const }) // 2× albums wide
        templates.push({ type: 'wide' as const, rows: 4, cols: cols, albumSize: 3 as const }) // 1× albums wide
        templates.push({ type: 'medium' as const, rows: 7, cols: 12, albumSize: 3 as const }) // 2×4 = 8 albums
        templates.push({ type: 'medium' as const, rows: 7, cols: 9, albumSize: 3 as const }) // 2×3 = 6 albums
        templates.push({ type: 'medium' as const, rows: 4, cols: 9, albumSize: 3 as const }) // 1×3 = 3 albums
        templates.push({ type: 'medium' as const, rows: 4, cols: 6, albumSize: 3 as const }) // 1×2 = 2 albums
      }
      if (cols >= 9) {
        templates.push({ type: 'wide' as const, rows: 7, cols: cols, albumSize: 3 as const }) // 2× albums
        templates.push({ type: 'wide' as const, rows: 4, cols: cols, albumSize: 3 as const }) // 1× albums
        templates.push({ type: 'medium' as const, rows: 7, cols: 9, albumSize: 3 as const }) // 2×3 = 6 albums
        templates.push({ type: 'square' as const, rows: 7, cols: 9, albumSize: 3 as const }) // 2×3 = 6 albums
        templates.push({ type: 'square' as const, rows: 7, cols: 6, albumSize: 3 as const }) // 2×2 = 4 albums
        templates.push({ type: 'square' as const, rows: 4, cols: 6, albumSize: 3 as const }) // 1×2 = 2 albums
      }

      // Medium sections with 4×4 or 3×3 albums (1-3 albums)
      templates.push({ type: 'square' as const, rows: 9, cols: 12, albumSize: 4 as const }) // 2×3 = 6 albums
      templates.push({ type: 'square' as const, rows: 9, cols: 8, albumSize: 4 as const }) // 2×2 = 4 albums
      templates.push({ type: 'square' as const, rows: 5, cols: 8, albumSize: 4 as const }) // 1×2 = 2 albums
      templates.push({ type: 'square' as const, rows: 5, cols: 9, albumSize: 3 as const }) // 1×3 = 3 albums
      templates.push({ type: 'square' as const, rows: 5, cols: 6, albumSize: 3 as const }) // 1×2 = 2 albums

      // Vertical sections (tall, narrow)
      templates.push({ type: 'vertical' as const, rows: 9, cols: 8, albumSize: 4 as const }) // 2×2 = 4 albums
      templates.push({ type: 'vertical' as const, rows: 9, cols: 4, albumSize: 4 as const }) // 2×1 = 2 albums
      templates.push({ type: 'vertical' as const, rows: 7, cols: 9, albumSize: 3 as const }) // 2×3 = 6 albums
      templates.push({ type: 'vertical' as const, rows: 7, cols: 6, albumSize: 3 as const }) // 2×2 = 4 albums
      templates.push({ type: 'vertical' as const, rows: 7, cols: 3, albumSize: 3 as const }) // 2×1 = 2 albums
      templates.push({ type: 'vertical' as const, rows: 4, cols: 6, albumSize: 3 as const }) // 1×2 = 2 albums

      // Compact 2×2 sections (only when larger sizes don't fit)
      templates.push({ type: 'square' as const, rows: 5, cols: 8, albumSize: 2 as const }) // 2×4 = 8 albums
      templates.push({ type: 'square' as const, rows: 5, cols: 6, albumSize: 2 as const }) // 2×3 = 6 albums
      templates.push({ type: 'square' as const, rows: 5, cols: 4, albumSize: 2 as const }) // 2×2 = 4 albums
      templates.push({ type: 'square' as const, rows: 3, cols: 6, albumSize: 2 as const }) // 1×3 = 3 albums
      templates.push({ type: 'square' as const, rows: 3, cols: 4, albumSize: 2 as const }) // 1×2 = 2 albums

      return templates
    }

    // Sort templates by area (rows × cols) in descending order to prioritize larger sections
    const sectionTemplates = generateTemplates().sort((a, b) => {
      const areaA = a.rows * a.cols
      const areaB = b.rows * b.cols
      return areaB - areaA // Larger sections first
    })

    let sectionCounter = 0
    const availableRows = rows >= bottomSectionRows ? rows - bottomSectionRows : rows

    // Section title suggestions
    const sectionTitles = [
      'On repeat',
      'Time capsule',
      "Don't forget about",
      'Recently played',
      'Your top picks',
      'Discover new music',
      'Fan favorites',
      'Hidden gems',
      'Trending now',
      'For you',
      'Fresh finds',
      'Popular this week',
      'Classic albums',
    ]

    // Track which special sections we've added
    let onRepeatSectionAdded = false
    let timeCapsuleSectionAdded = false
    let forgottenSectionAdded = false

    // Greedy Maximum Coverage Algorithm with Interesting Layout Rules
    // Balance between maximum coverage and visual variety
    const MIN_SECTIONS = 4
    const MAX_SECTIONS = 6
    const remainingSlotsMax = MAX_SECTIONS - sections.length

    // Helper: Count how many uncovered cells would be covered by placing a template at position
    const countNewCoverage = (startRow: number, startCol: number, template: { rows: number; cols: number }): number => {
      let count = 0
      for (let r = startRow; r < Math.min(startRow + template.rows, availableRows); r++) {
        for (let c = startCol; c < Math.min(startCol + template.cols, cols); c++) {
          if (!occupied[r][c]) count++
        }
      }
      return count
    }

    // Helper: Check if placement would leave unfillable gaps
    const wouldLeaveSmallGaps = (startRow: number, startCol: number, template: { rows: number; cols: number }): number => {
      // Simulate placement and count remaining small gaps
      const tempOccupied = occupied.map(row => [...row])
      for (let r = startRow; r < startRow + template.rows && r < availableRows; r++) {
        for (let c = startCol; c < startCol + template.cols && c < cols; c++) {
          tempOccupied[r][c] = true
        }
      }

      // Count isolated empty cells or small gaps that can't fit any section
      let smallGapPenalty = 0
      for (let r = 0; r < availableRows; r++) {
        for (let c = 0; c < cols; c++) {
          if (!tempOccupied[r][c]) {
            // Check if this cell is part of a small isolated gap (< 6 cells)
            let gapSize = 0
            let checkCols = 0
            while (c + checkCols < cols && !tempOccupied[r][c + checkCols]) {
              gapSize++
              checkCols++
              if (gapSize >= 6) break
            }
            if (gapSize < 6) smallGapPenalty += 10
          }
        }
      }
      return smallGapPenalty
    }

    // Helper: Calculate how efficiently a template fills the available space
    const calculateSpaceEfficiency = (
      _startRow: number,
      _startCol: number,
      template: typeof sectionTemplates[0]
    ): number => {
      // Calculate the actual area the section would occupy
      const sectionArea = template.rows * template.cols

      // Calculate how many complete albums fit (accounting for 1 heading row)
      const albumRows = template.rows - 1
      const albumsPerRow = Math.floor(template.cols / template.albumSize)
      const albumRowsCount = Math.floor(albumRows / template.albumSize)
      const albumsTotal = albumsPerRow * albumRowsCount

      // Calculate the space used by albums (in cells)
      const albumCellsUsed = albumsTotal * (template.albumSize * template.albumSize)

      // Calculate waste: section area minus actual album coverage
      const wastedCells = sectionArea - albumCellsUsed - template.cols // minus heading row

      // Return efficiency score (lower waste = higher score)
      const efficiencyRatio = 1 - (wastedCells / sectionArea)
      return efficiencyRatio * 100 // Scale to 0-100
    }

    // Helper: Calculate layout flow score (penalize awkward splits)
    const calculateLayoutFlow = (
      row: number,
      col: number,
      template: typeof sectionTemplates[0]
    ): number => {
      let flowScore = 0

      // Rule 1: Strongly prefer edge/corner positions over middle positions
      const isLeftEdge = col === 0
      const isTopEdge = row === 0
      const isRightEdge = col + template.cols >= cols
      const isBottomEdge = row + template.rows >= availableRows

      if (isLeftEdge || isRightEdge) flowScore += 20
      if (isTopEdge || isBottomEdge) flowScore += 20

      // Corners get extra bonus (creates natural flow)
      if ((isLeftEdge || isRightEdge) && (isTopEdge || isBottomEdge)) flowScore += 10

      // Rule 2: Penalize small sections in the middle (they split the layout)
      const sectionArea = template.rows * template.cols
      const isSmallSection = sectionArea < 24 // Less than ~24 cells
      const isMiddlePosition = !isLeftEdge && !isRightEdge && !isTopEdge && !isBottomEdge

      if (isSmallSection && isMiddlePosition) {
        flowScore -= 40 // Heavy penalty for small middle sections
      }

      // Rule 3: Reward larger sections (they create cleaner layouts)
      if (sectionArea > 50) flowScore += 15
      if (sectionArea > 70) flowScore += 10

      return flowScore
    }

    // Helper: Calculate "interestingness" score for visual variety
    const calculateInterestScore = (
      _row: number,
      _col: number,
      template: typeof sectionTemplates[0],
      placedSections: typeof sections
    ): number => {
      let score = 0

      // Rule 1: Prefer consistent album size, but allow flexibility for better space filling
      const mostCommonAlbumSize = placedSections.length > 0
        ? placedSections.reduce((acc, s) => s.albumSize === 2 ? acc + 1 : acc, 0) > placedSections.length / 2 ? 2 : 3
        : 2
      if (template.albumSize === mostCommonAlbumSize) {
        score += 10 // Reduced further to allow more flexibility
      }

      // Rule 2: Prefer different aspect ratios from recently placed sections
      if (placedSections.length > 2) {
        const lastSection = placedSections[placedSections.length - 1]
        const lastAspectRatio = lastSection.cols / lastSection.rows
        const newAspectRatio = template.cols / template.rows
        const aspectDiff = Math.abs(lastAspectRatio - newAspectRatio)
        score += aspectDiff * 3 // Reduced
      }

      // Rule 3: Prefer alternating orientations (wide vs tall vs square)
      if (placedSections.length > 2) {
        const lastType = placedSections[placedSections.length - 1].type
        if (lastType !== template.type) {
          score += 5 // Reduced
        }
      }

      return score
    }

    // Greedy placement with high priority on coverage
    let dynamicSectionsAdded = 0
    while (dynamicSectionsAdded < remainingSlotsMax) {
      let bestPlacement: {
        row: number
        col: number
        template: typeof sectionTemplates[0]
        score: number
      } | null = null

      // Evaluate all possible placements
      for (let row = 0; row < availableRows; row++) {
        for (let col = 0; col < cols; col++) {
          if (occupied[row][col]) continue

          // Try each template at this position
          for (const template of sectionTemplates) {
            if (isAvailable(row, col, template.rows, template.cols)) {
              const coverage = countNewCoverage(row, col, template)
              const interestScore = calculateInterestScore(row, col, template, sections)
              const gapPenalty = wouldLeaveSmallGaps(row, col, template)
              const efficiency = calculateSpaceEfficiency(row, col, template)
              const flowScore = calculateLayoutFlow(row, col, template)

              // Combined score:
              // 50% coverage (maximize space filled)
              // 20% efficiency (minimize dead space within sections)
              // 15% flow (prefer edges, avoid middle splits)
              // 5% visual interest (subtle variety)
              // minus gap penalty
              const totalScore = (coverage * 0.5) + (efficiency * 0.2) + (flowScore * 0.15) + (interestScore * 0.05) - gapPenalty

              // Keep track of best placement
              if (!bestPlacement || totalScore > bestPlacement.score) {
                bestPlacement = { row, col, template, score: totalScore }
              }
            }
          }
        }
      }

      // If no valid placement found, stop
      if (!bestPlacement) break

      // Place the best section
      const { row, col, template } = bestPlacement

      // Prioritize special sections in order: On repeat > Time capsule > Don't forget about > random
      let sectionId: string
      let title: string
      if (!onRepeatSectionAdded && onRepeatAlbums.length > 0) {
        sectionId = 'on-repeat'
        title = 'On repeat'
        onRepeatSectionAdded = true
      } else if (!timeCapsuleSectionAdded && timeCapsuleAlbums.length > 0) {
        sectionId = 'time-capsule'
        title = 'Time capsule'
        timeCapsuleSectionAdded = true
      } else if (!forgottenSectionAdded && forgottenAlbums.length > 0) {
        sectionId = 'forgotten'
        title = "Don't forget about"
        forgottenSectionAdded = true
      } else {
        sectionId = `section-${sectionCounter}`
        // Skip special section titles (first 3)
        title = sectionTitles[(sectionCounter + 3) % sectionTitles.length]
      }

      sections.push({
        id: sectionId,
        title,
        type: template.type,
        rows: template.rows,
        cols: template.cols,
        albumSize: template.albumSize,
        gridColumn: `${col + 1} / ${col + template.cols + 1}`,
        gridRow: `${row + 1} / ${row + template.rows + 1}`,
      })
      markOccupied(row, col, template.rows, template.cols)
      dynamicSectionsAdded++
      sectionCounter++
    }

    // Calculate space usage
    const totalAvailableSpace = availableRows * cols
    const occupiedSpaceInAvailable = occupied.slice(0, availableRows).reduce((sum, row) =>
      sum + row.filter(cell => cell).length, 0
    )
    const spaceUsagePercent = ((occupiedSpaceInAvailable / totalAvailableSpace) * 100).toFixed(1)

    // Log section count and space usage for debugging
    debug.log(`Generated ${sections.length} sections (min: ${MIN_SECTIONS}, max: ${MAX_SECTIONS}) - Space usage: ${spaceUsagePercent}%`)

    return sections
  }, [rows, cols])

  // Show loading skeleton while data is loading
  if (isLoading) {
    return (
      <div data-testid="home-page-loading" className="h-full w-full flex items-center justify-center">
        <SkeletonGrid count={12} type="album" />
      </div>
    )
  }

  // Map section title keys to i18n
  const sectionTitleMap: Record<string, string> = {
    'Jump back into': t('home.jumpBackInto'),
    'Do some crate digging': t('home.crateDigging'),
    'On repeat': t('home.onRepeat'),
    'Time capsule': t('home.timeCapsule'),
    "Don't forget about": t('home.dontForget'),
    'Recently played': t('home.recentlyPlayed'),
    'Your top picks': t('home.yourTopPicks'),
    'Discover new music': t('home.discoverNewMusic'),
    'Fan favorites': t('home.fanFavorites'),
    'Hidden gems': t('home.hiddenGems'),
    'Trending now': t('home.trendingNow'),
    'For you': t('home.forYou'),
    'Fresh finds': t('home.freshFinds'),
    'Popular this week': t('home.popularThisWeek'),
    'Classic albums': t('home.classicAlbums'),
  }
  const sectionTitle = (section: BentoSection) => sectionTitleMap[section.title] || section.title

  return (
    <div data-testid="home-page" className="h-full w-full overflow-hidden">
      <style>{`
        .grid-container {
          height: 100%;
          width: 100%;
          /* Match LibraryPageLayout padding: pr-6 pb-6 pt-6 (MainLayout already provides pl-6) */
          padding-right: 1.5rem;
          padding-bottom: 1.5rem;
          padding-top: 1.5rem;
          box-sizing: border-box;
          overflow: hidden;
        }

        .grid-content {
          display: grid;
          grid-template-columns: repeat(${cols}, 1fr);
          grid-template-rows: repeat(${rows}, 1fr);
          gap: 0.5rem;
          width: 100%;
          height: 100%;
        }

        /* Bento box sections */
        .bento-section {
          background: rgba(59, 130, 246, 0.1);
          border: 1px solid rgba(59, 130, 246, 0.3);
          border-radius: 0.5rem;
          padding: 0.5rem;
          transition: all 0.2s ease;
          overflow: hidden;
        }

        .bento-section:hover {
          background: rgba(59, 130, 246, 0.15);
          border-color: rgba(59, 130, 246, 0.5);
          z-index: 10;
        }

        /* Real sections - clean look without debug styling */
        .bento-section[data-section-id="jump-back"],
        .bento-section[data-section-id="on-repeat"],
        .bento-section[data-section-id="time-capsule"],
        .bento-section[data-section-id="forgotten"],
        .bento-section[data-section-id="bottom"] {
          background: transparent;
          border: none;
          padding: 0.5rem 0;
        }

        .bento-section[data-section-id="jump-back"]:hover,
        .bento-section[data-section-id="on-repeat"]:hover,
        .bento-section[data-section-id="time-capsule"]:hover,
        .bento-section[data-section-id="forgotten"]:hover,
        .bento-section[data-section-id="bottom"]:hover {
          background: transparent;
          border: none;
        }

        .bento-section-content {
          display: grid;
          height: 100%;
          width: 100%;
          gap: 0.5rem;
        }

        .bento-section-header {
          font-size: 16px;
          font-weight: 600;
          color: hsl(var(--foreground));
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          display: flex;
          align-items: flex-end;
          padding-bottom: 0.25rem;
          z-index: 1;
        }

        /* Larger headings for real sections */
        .bento-section[data-section-id="jump-back"] .bento-section-header,
        .bento-section[data-section-id="on-repeat"] .bento-section-header,
        .bento-section[data-section-id="time-capsule"] .bento-section-header,
        .bento-section[data-section-id="forgotten"] .bento-section-header,
        .bento-section[data-section-id="bottom"] .bento-section-header {
          font-size: 20px;
          font-weight: 700;
          padding-bottom: 0.5rem;
        }

        .bento-album-card {
          background: hsl(var(--muted));
          border-radius: 0.375rem;
          overflow: hidden;
          width: 100%;
          height: 100%;
        }

        .bento-album-card:hover {
          background: hsl(var(--muted) / 0.8);
        }

        /* Real album cards - fill grid cells exactly */
        .bento-album-real {
          background: transparent;
          padding: 0;
          display: block;
        }

        /* Override AlbumCard/MediaCard spacing for bento grid */
        .bento-album-real > div {
          width: 100% !important;
          height: 100% !important;
          margin: 0 !important;
        }

        /* Make artwork fill entire space */
        .bento-album-real > div > div {
          margin-bottom: 0 !important;
          width: 100% !important;
          height: 100% !important;
          border-radius: 0.375rem;
        }

        /* Hide text elements */
        .bento-album-real > div > p {
          display: none !important;
        }

        /* Debug grid cells */
        .grid-cell-debug {
          aspect-ratio: 1 / 1;
          background: rgba(59, 130, 246, 0.05);
          border: 1px solid rgba(59, 130, 246, 0.2);
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 10px;
          color: rgba(59, 130, 246, 0.4);
          font-family: monospace;
        }
      `}</style>

      <div ref={containerRef} className="grid-container">
        <div className="grid-content">
          {/* Debug grid cells - show all individual cells when enabled */}
          {SHOW_DEBUG_GRID && Array.from({ length: totalCells }).map((_, i) => {
            const row = Math.floor(i / cols) + 1
            const col = (i % cols) + 1

            return (
              <div
                key={i}
                className="grid-cell-debug"
                style={{
                  gridColumn: col,
                  gridRow: row
                }}
              >
                {i + 1}
              </div>
            )
          })}

          {/* Render all bento sections */}
          {sections
            .filter((section) => {
              // Only show sections with real data, hide placeholder sections
              const realSectionIds = ['jump-back', 'on-repeat', 'time-capsule', 'forgotten', 'bottom']
              return realSectionIds.includes(section.id)
            })
            .map((section) => {
            // Calculate how many albums fit
            const albumRows = section.rows - 1 // Subtract heading row
            const albumsPerRow = Math.floor(section.cols / section.albumSize)
            const albumRowsAvailable = Math.floor(albumRows / section.albumSize)
            const albumsTotal = albumsPerRow * albumRowsAvailable

            return (
              <div
                key={section.id}
                className="bento-section"
                data-section-id={section.id}
                data-testid={`home-section-${section.id}`}
                style={{
                  gridColumn: section.gridColumn,
                  gridRow: section.gridRow,
                }}
              >
                <div
                  className="bento-section-content"
                  style={{
                    gridTemplateColumns: `repeat(${section.cols}, 1fr)`,
                    gridTemplateRows: `repeat(${section.rows}, 1fr)`,
                  }}
                >
                  {/* Heading - spans full width, first row, aligned bottom */}
                  <div
                    className="bento-section-header"
                    data-testid={`home-section-header-${section.id}`}
                    style={{
                      gridColumn: `1 / ${section.cols + 1}`,
                      gridRow: '1 / 2',
                    }}
                  >
                    {sectionTitle(section)}
                  </div>

                  {/* Albums - each occupies albumSize × albumSize cells */}
                  {Array.from({ length: albumsTotal }).map((_, i) => {
                    const albumRow = Math.floor(i / albumsPerRow)
                    const albumCol = i % albumsPerRow

                    // Start position (row 2 onwards, after heading)
                    const startRow = 1 + 1 + (albumRow * section.albumSize) // 1 (base) + 1 (heading) + offset
                    const startCol = 1 + (albumCol * section.albumSize)

                    // For on-repeat section, show albums played most in last 2 weeks
                    if (section.id === 'on-repeat' && onRepeatAlbums[i]) {
                      const album = onRepeatAlbums[i]
                      return (
                        <div
                          key={i}
                          className="bento-album-card bento-album-real"
                          style={{
                            gridColumn: `${startCol} / ${startCol + section.albumSize}`,
                            gridRow: `${startRow} / ${startRow + section.albumSize}`,
                          }}
                          onClick={() => handleAlbumClick(album.id)}
                        >
                          <AlbumCard
                            album={{
                              id: album.id,
                              title: album.title,
                              artist_name: album.artist_name,
                              artist_id: album.artist_id,
                              year: album.year,
                              cover_art_path: album.cover_art_path,
                            }}
                            showArtist={false}
                            className="w-full h-full"
                            priority={i < 3}
                          />
                        </div>
                      )
                    }

                    // For jump-back section, show actual recent albums
                    if (section.id === 'jump-back' && jumpBackAlbums[i]) {
                      const album = jumpBackAlbums[i]
                      return (
                        <div
                          key={i}
                          className="bento-album-card bento-album-real"
                          style={{
                            gridColumn: `${startCol} / ${startCol + section.albumSize}`,
                            gridRow: `${startRow} / ${startRow + section.albumSize}`,
                          }}
                          onClick={() => handleAlbumClick(album.id)}
                        >
                          <AlbumCard
                            album={{
                              id: album.id,
                              title: album.title,
                              artist_name: album.artist_name,
                              artist_id: album.artist_id,
                              year: album.year,
                              cover_art_path: album.cover_art_path,
                            }}
                            showArtist={false}
                            className="w-full h-full"
                            priority={i < 3}
                          />
                        </div>
                      )
                    }

                    // For time capsule section, show albums from 2-6 months ago
                    if (section.id === 'time-capsule' && timeCapsuleAlbums[i]) {
                      const album = timeCapsuleAlbums[i]
                      return (
                        <div
                          key={i}
                          className="bento-album-card bento-album-real"
                          style={{
                            gridColumn: `${startCol} / ${startCol + section.albumSize}`,
                            gridRow: `${startRow} / ${startRow + section.albumSize}`,
                          }}
                          onClick={() => handleAlbumClick(album.id)}
                        >
                          <AlbumCard
                            album={{
                              id: album.id,
                              title: album.title,
                              artist_name: album.artist_name,
                              artist_id: album.artist_id,
                              year: album.year,
                              cover_art_path: album.cover_art_path,
                            }}
                            showArtist={false}
                            className="w-full h-full"
                            priority={false}
                          />
                        </div>
                      )
                    }

                    // For forgotten section, show albums not played recently
                    if (section.id === 'forgotten' && forgottenAlbums[i]) {
                      const album = forgottenAlbums[i]
                      return (
                        <div
                          key={i}
                          className="bento-album-card bento-album-real"
                          style={{
                            gridColumn: `${startCol} / ${startCol + section.albumSize}`,
                            gridRow: `${startRow} / ${startRow + section.albumSize}`,
                          }}
                          onClick={() => handleAlbumClick(album.id)}
                        >
                          <AlbumCard
                            album={{
                              id: album.id,
                              title: album.title,
                              artist_name: album.artist_name,
                              artist_id: album.artist_id,
                              year: album.year,
                              cover_art_path: album.cover_art_path,
                            }}
                            showArtist={false}
                            className="w-full h-full"
                            priority={false}
                          />
                        </div>
                      )
                    }

                    // For bottom section, show actual random albums
                    if (section.id === 'bottom' && crateDiggingAlbums[i]) {
                      const album = crateDiggingAlbums[i]
                      return (
                        <div
                          key={i}
                          className="bento-album-card bento-album-real"
                          style={{
                            gridColumn: `${startCol} / ${startCol + section.albumSize}`,
                            gridRow: `${startRow} / ${startRow + section.albumSize}`,
                          }}
                          onClick={() => handleAlbumClick(album.id)}
                        >
                          <AlbumCard
                            album={{
                              id: album.id,
                              title: album.title,
                              artist_name: album.artist_name,
                              artist_id: album.artist_id,
                              year: album.year,
                              cover_art_path: album.cover_art_path,
                            }}
                            showArtist={false}
                            className="w-full h-full"
                            priority={false}
                          />
                        </div>
                      )
                    }

                    // For other sections, show empty placeholder card
                    return (
                      <div
                        key={i}
                        className="bento-album-card"
                        style={{
                          gridColumn: `${startCol} / ${startCol + section.albumSize}`,
                          gridRow: `${startRow} / ${startRow + section.albumSize}`,
                        }}
                      />
                    )
                  })}
                </div>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
