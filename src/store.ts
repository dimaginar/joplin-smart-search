import { create } from 'zustand'
import type { IndexStatus, SearchResult, Note } from './types'

interface AppStore {
  dbPath: string | null
  indexStatus: IndexStatus
  results: SearchResult[]
  selectedNote: Note | null
  query: string

  setDbPath: (p: string) => void
  setIndexStatus: (s: IndexStatus) => void
  setResults: (r: SearchResult[]) => void
  setSelectedNote: (n: Note | null) => void
  setQuery: (q: string) => void
}

export const useAppStore = create<AppStore>((set) => ({
  dbPath: null,
  indexStatus: {
    total_notes: 0,
    indexed_notes: 0,
    is_ready: false,
    is_downloading_model: false,
    download_progress: 0,
    error: null,
  },
  results: [],
  selectedNote: null,
  query: '',
  setDbPath: (p) => set({ dbPath: p }),
  setIndexStatus: (s) => set({ indexStatus: s }),
  setResults: (r) => set({ results: r }),
  setSelectedNote: (n) => set({ selectedNote: n }),
  setQuery: (q) => set({ query: q }),
}))