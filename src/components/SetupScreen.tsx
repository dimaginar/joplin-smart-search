import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useAppStore } from '../store'

export default function SetupScreen() {
  const { setDbPath } = useAppStore()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleBrowse = async () => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      const result = await open({ filters: [{ name: 'SQLite', extensions: ['sqlite', 'db'] }], multiple: false })
      const path = typeof result === 'string' ? result : null
      if (path) {
        await invoke('set_joplin_db_path', { path })
        setDbPath(path)
      }
    } catch (err) {
      console.error('Failed to set DB path:', err)
      setError(typeof err === 'string' ? err : 'Failed to open database.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex h-screen w-full flex-col items-center justify-center bg-zinc-950 text-zinc-100">
      <h1 className="mb-2 text-xl font-bold">Joplin database not found</h1>
      <p className="mb-6 text-sm text-zinc-400">Locate your Joplin SQLite database to get started.</p>
      <button
        onClick={handleBrowse}
        disabled={busy}
        className="rounded-lg bg-indigo-600 px-6 py-3 font-medium text-white hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-zinc-950 disabled:opacity-50"
      >
        {busy ? 'Opening…' : 'Browse for database'}
      </button>
      {error && <p className="mt-4 text-sm text-red-400">{error}</p>}
    </div>
  )
}
