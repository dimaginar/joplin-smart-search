import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../store'
import type { Note } from '../types'

export default function ResultsList() {
  const { results, selectedNote, setSelectedNote, query } = useAppStore()

  const handleSelect = async (id: string) => {
    try {
      const note = await invoke<Note>('get_note', { id })
      setSelectedNote(note)
    } catch (err) {
      console.error('get_note failed:', err)
    }
  }

  return (
    <div className="space-y-2 overflow-y-auto pr-2">
      {results.length === 0 && (
        <p className="text-sm text-zinc-500">
          {query.trim() === '' ? 'Start typing to search.' : 'No notes matched your query.'}
        </p>
      )}
      {results.map((result) => (
        <button
          key={result.note.id}
          type="button"
          onClick={() => handleSelect(result.note.id)}
          className={`w-full text-left cursor-pointer rounded-lg px-3 py-3 transition-colors ${
            selectedNote?.id === result.note.id
              ? 'bg-indigo-900/30 border-l-2 border-indigo-500'
              : 'hover:bg-zinc-800 border-l-2 border-transparent'
          }`}
        >
          <h3 className="font-medium text-zinc-100">{result.note.title}</h3>
          <p className="text-xs text-zinc-400">{(result.score * 100).toFixed(0)}% match</p>
        </button>
      ))}
    </div>
  )
}
