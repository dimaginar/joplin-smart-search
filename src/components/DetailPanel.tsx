import { invoke } from '@tauri-apps/api/core'
import type { Note } from '../types'

const UUID_RE = /^[0-9a-f]{32}$/

export default function DetailPanel({ note }: { note: Note }) {
  const canOpen = UUID_RE.test(note.id)

  const handleOpen = () => {
    if (!canOpen) return
    invoke('open_in_joplin', { noteId: note.id }).catch(console.error)
  }

  return (
    <div className="flex h-full flex-col">
      <h1 className="mb-4 text-2xl font-bold text-zinc-100">{note.title}</h1>
      <div className="mb-6 flex-1 rounded-lg border border-zinc-800 bg-zinc-900/50 p-4 overflow-y-auto">
        {note.body.trim() ? (
          <p className="text-sm leading-relaxed text-zinc-300 whitespace-pre-wrap">{note.body}</p>
        ) : (
          <p className="text-sm text-zinc-500 italic">No content</p>
        )}
      </div>
      <div className="mt-auto">
        {canOpen ? (
          <button
            type="button"
            onClick={handleOpen}
            className="inline-flex items-center justify-center rounded-lg bg-indigo-600 px-4 py-3 text-sm font-medium text-white hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-zinc-950 transition-colors"
          >
            Open in Joplin
          </button>
        ) : (
          <span className="text-xs text-red-400">Invalid note ID</span>
        )}
      </div>
    </div>
  )
}
