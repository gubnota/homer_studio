export interface CommandError { code: string; message: string }

export interface Segment {
  id: string
  order: number
  text: string
  selectedTake: string | null
}

export interface Chapter {
  id: string
  title: string
  order: number
  sourcePath: string
  processedPath: string | null
  sourceText: string
  processedText: string | null
  segments: Segment[]
  audioPath: string | null
  audioStale: boolean
}

export interface ProjectSnapshot {
  schemaVersion: 1
  id: string
  title: string
  revision: number
  createdAtMs: number
  updatedAtMs: number
  rootPath: string
  chapters: Chapter[]
}

export type LlmSettings =
  | { provider: 'none' }
  | { provider: 'llama_cpp'; executable_path: string; model_path: string; context_size: number; max_tokens: number; gpu_layers: number }
  | { provider: 'ollama'; base_url: string; model: string; context_size: number; max_tokens: number }

export interface Settings {
  schemaVersion: 1
  llm: LlmSettings
  speech: { provider: 'macos_say'; voiceId: string; rate: number }
  ffmpegPath: string | null
  ffprobePath: string | null
}

export interface ToolDiagnostic { name: string; path: string | null; available: boolean }
export interface JobRecord { id: string; kind: string; label: string; status: 'queued' | 'running' | 'completed' | 'cancelled' | 'failed'; progress: number; message: string | null; createdAtMs: number }
