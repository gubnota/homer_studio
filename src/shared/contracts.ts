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
  audioDurationMs: number | null
  audioOrigin: string | null
  reviewStatus: 'pending' | 'approved' | 'changes_requested' | null
  cues: LineCue[]
}

export interface LineCue { order: number; text: string; startMs: number; endMs: number }

export interface ExportRecord {
  id: string
  audioPath: string
  timestampsPath: string
  durationMs: number
  createdAtMs: number
  sourceRevision: number
  sourceUpdatedAtMs: number
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
  exports: ExportRecord[]
}

export interface TextCandidate {
  text: string
  pieces: string[]
  provider: string
  model: string
}

export interface VoiceSample { id: string; name: string; durationMs: number }
export interface Voice { id: string; name: string; samples: VoiceSample[]; selectedSampleId: string | null; builtIn: boolean }
export interface VoicePreset { id: string; name: string; voiceId: string; rate: number; builtIn: boolean }

export type LlmSettings =
  | { provider: 'none' }
  | { provider: 'llama_cpp'; executable_path: string; model_path: string; context_size: number; max_tokens: number; gpu_layers: number }
  | { provider: 'ollama'; base_url: string; model: string; context_size: number; max_tokens: number }

export interface Settings {
  schemaVersion: 1
  llm: LlmSettings
  speech: { provider: 'macos_say' | 'chatterbox_turbo'; voiceId: string; rate: number }
  ffmpegPath: string | null
  ffprobePath: string | null
  ollamaPath: string | null
  voicePresets: VoicePreset[]
  selectedVoicePresetId: string | null
  sounds: { chatterboxUrl: string; sfxUrl: string }
}

export type SoundCategory = 'speech' | 'vocal_gesture' | 'sound_effect'
export interface SoundRequest { prompt: string; category: SoundCategory; durationSeconds: number; seed: number | null; negativePrompt?: string | null }
export interface SoundAsset {
  id: string; prompt: string; category: SoundCategory; provider: string; model: string
  requestedDurationSeconds: number; durationMs: number; seed: number | null; createdAtMs: number
  masterPath: string; previewPath: string
  voiceId?: string | null; negativePrompt?: string | null
}
export interface WorkerHealth {
  protocolVersion: number; engine: string; model: string; ready: boolean
  categories: SoundCategory[]; maxDurationSeconds: number; message: string
}

export interface ToolDiagnostic {
  key: 'speech' | 'ffmpeg' | 'ffprobe' | 'llama' | 'ollama' | 'ollama_service'
  name: string
  path: string | null
  available: boolean
  status: 'configured' | 'invalid_configuration' | 'found_automatically' | 'not_found' | 'service_reachable' | 'service_unavailable' | 'service_ready' | 'no_models' | 'model_not_installed'
  configuredPath: string | null
  detectedPath: string | null
}
export interface JobEvent { atMs: number; status: string; progress: number; message: string }
export interface JobRecord { id: string; kind: string; label: string; status: 'queued' | 'running' | 'completed' | 'cancelled' | 'failed'; progress: number; message: string | null; createdAtMs: number; events: JobEvent[] }
