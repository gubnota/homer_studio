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
