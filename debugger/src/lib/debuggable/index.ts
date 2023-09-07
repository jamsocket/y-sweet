export type EntityType = 'scalar' | 'list' | 'object' | 'text'

export type DebuggableEntry = {
  key: string | number
  value: Debuggable
}

export interface Debuggable {
  type(): EntityType
  typeName?(): string
  entries(): DebuggableEntry[]
  value(): any
  size(): number
  listen(listener: () => void): () => void

  toggleType?(): void
}
