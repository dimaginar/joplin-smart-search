import { invoke } from '@tauri-apps/api/core'
import SearchBar from './SearchBar'
import ResultsList from './ResultsList'
import DetailPanel from './DetailPanel'
import StatusIndicator from './StatusIndicator'
import { useAppStore } from '../store'

export default function MainLayout() {
  const { query, setQuery, selectedNote } = useAppStore()

  const handleReindex = async () => {
    try {
      await invoke('trigger_reindex')
    } catch (err) {
      console.error('trigger_reindex failed:', err)
    }
  }

  return (
    <div className="flex h-screen w-full flex-col bg-zinc-950 text-zinc-100">
      <header className="flex items-center gap-3 border-b border-zinc-800 px-6 py-4">
        <div className="flex-1">
          <SearchBar query={query} setQuery={setQuery} />
        </div>
        <button
          onClick={handleReindex}
          title="Rebuild index"
          className="flex-shrink-0 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-xs text-zinc-400 hover:border-zinc-500 hover:text-zinc-200 focus:outline-none focus:ring-2 focus:ring-indigo-500 transition-colors"
        >
          Refresh
        </button>
      </header>
      <div className="flex flex-1 overflow-hidden">
        <aside className="flex w-72 flex-col border-r border-zinc-800 bg-zinc-900/50">
          <div className="flex-1 overflow-y-auto p-4">
            <ResultsList />
          </div>
          <div className="border-t border-zinc-800 px-4 py-3">
            <StatusIndicator />
          </div>
        </aside>
        <main className="flex-1 overflow-y-auto bg-zinc-950 p-6">
          {selectedNote ? (
            <DetailPanel note={selectedNote} />
          ) : (
            <div className="flex h-full items-center justify-center text-zinc-500">
              Select a note to view details
            </div>
          )}
        </main>
      </div>
    </div>
  )
}
