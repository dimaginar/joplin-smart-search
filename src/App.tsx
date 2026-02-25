import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'
import { useAppStore } from './store'
import type { IndexStatus } from './types'
import SetupScreen from './components/SetupScreen'
import IndexingScreen from './components/IndexingScreen'
import MainLayout from './components/MainLayout'

export default function App() {
  const { dbPath, indexStatus, setDbPath, setIndexStatus } = useAppStore()
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let unlistenFn: (() => void) | undefined

    Promise.all([
      invoke<string | null>('detect_db_path'),
      invoke<IndexStatus>('get_index_status'),
    ])
      .then(([path, status]) => {
        if (path) setDbPath(path)
        setIndexStatus(status)
      })
      .catch((err) => console.error('startup invoke failed:', err))
      .finally(() => setLoading(false))

    listen<IndexStatus>('index-status', (event) => {
      setIndexStatus(event.payload)
    }).then((unlisten) => {
      unlistenFn = unlisten
    })

    return () => {
      unlistenFn?.()
    }
  }, [setDbPath, setIndexStatus])

  if (loading) return null

  if (dbPath === null) {
    return <SetupScreen />
  }

  if (!indexStatus.is_ready) {
    return <IndexingScreen />
  }

  return <MainLayout />
}
