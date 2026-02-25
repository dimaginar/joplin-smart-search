export interface Note {
  id: string
  title: string
  body: string
  updated_time: number
}

export interface NoteMetadata {
  id: string
  title: string
  updated_time: number
}

export interface SearchResult {
  note: NoteMetadata
  score: number
}

export interface IndexStatus {
  total_notes: number
  indexed_notes: number
  is_ready: boolean
  is_downloading_model: boolean
  download_progress: number
  error: string | null
}