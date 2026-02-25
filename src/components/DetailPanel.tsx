import { useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Note } from '../types'
import { renderMarkdown } from '../lib/renderMarkdown'
import { useAppStore } from '../store'

const UUID_RE = /^[0-9a-f]{32}$/
const JOPLIN_NOTE_RE = /^joplin-note:\/\/([0-9a-f]{32})$/

export default function DetailPanel({ note }: { note: Note }) {
  const canOpen = UUID_RE.test(note.id)
  const setSelectedNote = useAppStore((s) => s.setSelectedNote)

  const html = note.body.trim() ? renderMarkdown(note.body) : ''

  const handleOpen = () => {
    if (!canOpen) return
    invoke('open_in_joplin', { noteId: note.id }).catch(console.error)
  }

  const handlePreviewClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement
      const anchor = target.closest('a')
      if (!anchor) return

      e.preventDefault()
      const href = anchor.getAttribute('href') ?? ''

      const joplinMatch = JOPLIN_NOTE_RE.exec(href)
      if (joplinMatch) {
        const noteId = joplinMatch[1]
        invoke<Note>('get_note', { id: noteId })
          .then((fetchedNote) => setSelectedNote(fetchedNote))
          .catch(console.error)
        return
      }

      if (href.startsWith('http://') || href.startsWith('https://')) {
        invoke('open_external_url', { url: href }).catch(console.error)
      }
    },
    [setSelectedNote],
  )

  return (
    <div className="flex h-full flex-col">
      <h1 className="mb-4 text-2xl font-bold text-zinc-100">{note.title}</h1>
      <div className="mb-6 flex-1 rounded-lg border border-zinc-800 bg-zinc-900/50 p-4 overflow-y-auto">
        {html ? (
          <div
            className="prose-preview"
            onClick={handlePreviewClick}
            dangerouslySetInnerHTML={{ __html: html }}
          />
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
