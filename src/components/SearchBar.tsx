import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../store'
import type { SearchResult } from '../types'

export default function SearchBar({ query, setQuery }: { query: string; setQuery: (q: string) => void }) {
  const { setResults } = useAppStore()
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const latestRequestId = useRef(0)

  useEffect(() => {
    if (query.trim() === '') {
      setResults([])
      return
    }

    if (debounceTimer.current) clearTimeout(debounceTimer.current)

    debounceTimer.current = setTimeout(async () => {
      const requestId = ++latestRequestId.current
      try {
        const results = await invoke<SearchResult[]>('search_notes', { query })
        if (requestId === latestRequestId.current) {
          setResults(results)
        }
      } catch (error) {
        if (error === 'index_not_ready' || error === 'model_not_loaded') return
        console.error('search_notes failed:', error)
      }
    }, 350)

    return () => {
      if (debounceTimer.current) clearTimeout(debounceTimer.current)
    }
  }, [query, setResults])

  return (
    <div className="relative">
      <input
        type="text"
        placeholder="Search notes by concept..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-4 py-3 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
    </div>
  )
}
