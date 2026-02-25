import { useAppStore } from '../store'

export default function StatusIndicator() {
  const { indexStatus } = useAppStore()

  const { is_ready, is_downloading_model, error, total_notes } = indexStatus

  let color: string
  let label: string

  if (error) {
    color = 'bg-red-500'
    label = error
  } else if (is_ready) {
    color = 'bg-green-500'
    label = total_notes > 0 ? `Index ready · ${total_notes} notes` : 'Index active'
  } else if (is_downloading_model) {
    color = 'bg-yellow-500'
    label = 'Downloading model…'
  } else {
    color = 'bg-yellow-500'
    label = 'Indexing…'
  }

  return (
    <div className="flex items-center gap-2 text-xs text-zinc-400">
      <span className={`h-2 w-2 rounded-full ${color}`} />
      {label}
    </div>
  )
}