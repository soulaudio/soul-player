'use client'

import { useState, useEffect, useCallback, useRef } from 'react'
import { useRouter } from 'next/navigation'
import { Search, X, FileText, Loader2 } from 'lucide-react'

interface SearchResult {
  url: string
  title: string
  excerpt: string
}

// Pagefind types
interface PagefindResult {
  url: string
  excerpt: string
  meta: {
    title?: string
  }
  data: () => Promise<{
    url: string
    excerpt: string
    meta: { title?: string }
    content: string
  }>
}

interface PagefindSearchResponse {
  results: { id: string; data: () => Promise<PagefindResult> }[]
}

interface Pagefind {
  search: (query: string) => Promise<PagefindSearchResponse>
  init: () => Promise<void>
}

export function DocsSearch() {
  const [isOpen, setIsOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResult[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [pagefind, setPagefind] = useState<Pagefind | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const router = useRouter()

  // Load Pagefind dynamically (only available after build)
  useEffect(() => {
    const loadPagefind = async () => {
      if (typeof window === 'undefined') return

      try {
        // Pagefind is generated at build time - load via dynamic import with full URL
        // This bypasses Next.js module resolution
        const pagefindPath = `${window.location.origin}/_pagefind/pagefind.js`
        const pf = await import(/* webpackIgnore: true */ pagefindPath)
        if (pf.init) await pf.init()
        setPagefind(pf as unknown as Pagefind)
      } catch {
        // Pagefind not available (development mode or not built yet)
        // This is expected during development
      }
    }
    loadPagefind()
  }, [])

  // Keyboard shortcut to open search
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+K or Ctrl+K to open
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setIsOpen(true)
      }
      // Escape to close
      if (e.key === 'Escape') {
        setIsOpen(false)
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [])

  // Focus input when modal opens
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus()
    }
  }, [isOpen])

  // Search with Pagefind
  const performSearch = useCallback(async (searchQuery: string) => {
    if (!searchQuery.trim() || !pagefind) {
      setResults([])
      return
    }

    setIsLoading(true)
    try {
      const search = await pagefind.search(searchQuery)
      const searchResults: SearchResult[] = []

      // Get first 5 results
      for (const result of search.results.slice(0, 5)) {
        const data = await result.data()
        searchResults.push({
          url: data.url,
          title: data.meta?.title || 'Untitled',
          excerpt: data.excerpt || '',
        })
      }

      setResults(searchResults)
      setSelectedIndex(0)
    } catch (error) {
      console.error('Search error:', error)
      setResults([])
    } finally {
      setIsLoading(false)
    }
  }, [pagefind])

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(() => {
      performSearch(query)
    }, 200)
    return () => clearTimeout(timer)
  }, [query, performSearch])

  // Handle result selection
  const handleSelect = (url: string) => {
    router.push(url)
    setIsOpen(false)
    setQuery('')
  }

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex((i) => Math.min(i + 1, results.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter' && results[selectedIndex]) {
      handleSelect(results[selectedIndex].url)
    }
  }

  return (
    <>
      {/* Search trigger button */}
      <button
        onClick={() => setIsOpen(true)}
        className="flex items-center gap-2 px-3 py-1.5 text-sm text-muted-foreground bg-muted/50 rounded-md border border-border hover:bg-muted transition-colors"
      >
        <Search className="w-4 h-4" />
        <span className="hidden sm:inline">Search...</span>
        <kbd className="hidden sm:inline-flex items-center gap-1 px-1.5 py-0.5 text-xs bg-background rounded border border-border">
          <span className="text-xs">⌘</span>K
        </kbd>
      </button>

      {/* Search modal */}
      {isOpen && (
        <div
          className="fixed inset-0 z-[100] flex items-start justify-center pt-[20vh]"
          onClick={() => setIsOpen(false)}
        >
          {/* Backdrop */}
          <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" />

          {/* Modal */}
          <div
            className="relative w-full max-w-lg mx-4 bg-background border border-border rounded-lg shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Search input */}
            <div className="flex items-center gap-3 px-4 border-b border-border">
              <Search className="w-5 h-5 text-muted-foreground shrink-0" />
              <input
                ref={inputRef}
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={pagefind ? 'Search documentation...' : 'Search coming soon...'}
                disabled={!pagefind}
                className="flex-1 py-4 bg-transparent text-foreground placeholder:text-muted-foreground focus:outline-none disabled:opacity-50"
              />
              {isLoading && <Loader2 className="w-5 h-5 text-muted-foreground animate-spin" />}
              <button
                onClick={() => setIsOpen(false)}
                className="p-1 text-muted-foreground hover:text-foreground transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Results */}
            {results.length > 0 && (
              <ul className="max-h-80 overflow-y-auto p-2">
                {results.map((result, index) => (
                  <li key={result.url}>
                    <button
                      onClick={() => handleSelect(result.url)}
                      className={`w-full flex items-start gap-3 p-3 rounded-md text-left transition-colors ${
                        index === selectedIndex
                          ? 'bg-accent text-foreground'
                          : 'hover:bg-muted'
                      }`}
                    >
                      <FileText className="w-5 h-5 text-muted-foreground shrink-0 mt-0.5" />
                      <div className="min-w-0">
                        <div className="font-medium truncate">{result.title}</div>
                        <div
                          className="text-sm text-muted-foreground line-clamp-2"
                          dangerouslySetInnerHTML={{ __html: result.excerpt }}
                        />
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            )}

            {/* No results */}
            {query && !isLoading && results.length === 0 && pagefind && (
              <div className="p-8 text-center text-muted-foreground">
                No results found for "{query}"
              </div>
            )}

            {/* Footer */}
            <div className="flex items-center justify-between px-4 py-2 text-xs text-muted-foreground border-t border-border">
              <div className="flex items-center gap-4">
                <span className="flex items-center gap-1">
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">↑↓</kbd>
                  navigate
                </span>
                <span className="flex items-center gap-1">
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">↵</kbd>
                  select
                </span>
                <span className="flex items-center gap-1">
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">esc</kbd>
                  close
                </span>
              </div>
              <span>Powered by Pagefind</span>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
