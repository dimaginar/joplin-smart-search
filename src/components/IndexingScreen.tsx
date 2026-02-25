import { useAppStore } from '../store'

export default function IndexingScreen() {
  const { indexStatus } = useAppStore()
  const { indexed_notes, total_notes, is_downloading_model, download_progress, error } = indexStatus

  if (error) {
    return (
      <div className="flex h-screen w-full flex-col items-center justify-center bg-zinc-950 text-zinc-100">
        <h1 className="mb-2 text-xl font-bold text-red-400">Indexing failed</h1>
        <p className="max-w-sm text-center text-sm text-zinc-400">{error}</p>
      </div>
    )
  }

  const progressValue = is_downloading_model
    ? download_progress * 100
    : total_notes > 0 ? indexed_notes : undefined

  const progressMax = is_downloading_model
    ? 100
    : total_notes > 0 ? total_notes : undefined

  const label = is_downloading_model
    ? `Downloading model… ${Math.round(download_progress * 100)}%`
    : total_notes > 0
      ? `${indexed_notes} / ${total_notes} notes indexed`
      : 'Preparing…'

  return (
    <div className="flex h-screen w-full flex-col items-center justify-center bg-zinc-950 text-zinc-100">
      <h1 className="mb-6 text-xl font-bold">
        {is_downloading_model ? 'Downloading model…' : 'Building index…'}
      </h1>
      <div className="w-64">
        <progress
          className="h-2 w-full rounded-lg bg-zinc-800 accent-indigo-500"
          value={progressValue}
          max={progressMax}
        />
        <p className="mt-3 text-sm text-zinc-400">{label}</p>
      </div>
    </div>
  )
}
