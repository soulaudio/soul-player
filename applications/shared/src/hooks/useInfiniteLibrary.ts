/**
 * useInfiniteLibrary - Infinite scroll pagination for library items
 *
 * Uses TanStack Query's useInfiniteQuery for efficient data fetching and caching.
 * Automatically manages pagination, loading states, and data aggregation.
 *
 * @see https://tanstack.com/query/latest
 * @see https://idevbrandon.medium.com/building-infinite-scroll-in-react-with-useinfinitequery-07eb3635f1d8
 */

import { useInfiniteQuery } from '@tanstack/react-query'
import { useBackend, BackendAlbum, BackendArtist, BackendPlaylist } from '../contexts/BackendContext'

const PAGE_SIZE = 50 // Items per page

type LibraryItem = BackendAlbum | BackendArtist | BackendPlaylist

interface UseInfiniteLibraryOptions {
  type: 'albums' | 'artists' | 'playlists'
  searchQuery?: string
}

interface PageData<T> {
  items: T[]
  totalCount: number
  nextCursor: number | undefined
}

export function useInfiniteLibrary<T extends LibraryItem>({
  type,
  searchQuery = '',
}: UseInfiniteLibraryOptions) {
  const backend = useBackend()

  const query = useInfiniteQuery({
    queryKey: [type, 'infinite', searchQuery],
    queryFn: async ({ pageParam = 0 }) => {
      // Fetch all items (we'll do client-side pagination for now)
      let allItems: T[]

      switch (type) {
        case 'albums':
          allItems = (await backend.getAllAlbums()) as T[]
          break
        case 'artists':
          allItems = (await backend.getAllArtists()) as T[]
          break
        case 'playlists':
          allItems = (await backend.getAllPlaylists()) as T[]
          break
      }

      // Filter by search query
      if (searchQuery) {
        allItems = allItems.filter((item) =>
          'title' in item
            ? item.title?.toLowerCase().includes(searchQuery.toLowerCase())
            : 'name' in item
            ? item.name?.toLowerCase().includes(searchQuery.toLowerCase())
            : true
        )
      }

      // Paginate
      const start = pageParam * PAGE_SIZE
      const end = start + PAGE_SIZE
      const pageItems = allItems.slice(start, end)
      const hasMore = end < allItems.length

      return {
        items: pageItems,
        totalCount: allItems.length,
        nextCursor: hasMore ? pageParam + 1 : undefined,
      } as PageData<T>
    },
    getNextPageParam: (lastPage) => lastPage.nextCursor,
    initialPageParam: 0,
    staleTime: 1000 * 60 * 5, // Cache for 5 minutes
  })

  // Flatten all pages into single array
  const items = query.data?.pages.flatMap((page) => page.items) ?? []
  const totalCount = query.data?.pages[0]?.totalCount ?? 0

  return {
    items,
    totalCount,
    isLoading: query.isLoading,
    isFetchingNextPage: query.isFetchingNextPage,
    hasNextPage: query.hasNextPage,
    fetchNextPage: query.fetchNextPage,
    error: query.error,
  }
}
