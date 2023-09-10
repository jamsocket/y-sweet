import { Debuggable, DebuggableEntry, EntityType } from '.'
import { Awareness } from 'y-protocols/awareness'
import { debuggableJsValue } from './builtins'

export class DebuggableAwareness implements Debuggable {
  constructor(private readonly _awareness: Awareness) {}

  type(): EntityType {
    return 'object'
  }

  typeName(): string {
    return 'Map'
  }

  entries(): DebuggableEntry[] {
    return Array.from(this._awareness.getStates().entries()).map(([key, value]) => ({
      key,
      value: debuggableJsValue(value),
    }))
  }

  value(): any {
    return null
  }

  size(): number {
    return this._awareness.getStates().size
  }

  listen(listener: () => void): () => void {
    this._awareness.on('change', listener)

    return () => {
      this._awareness.off('change', listener)
    }
  }
}
