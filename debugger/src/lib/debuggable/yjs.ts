import * as Y from 'yjs'
import { JsList, JsObject, Scalar, debuggableJsValue } from './builtins'
import { Debuggable, DebuggableEntry, EntityType } from '.'

export function debuggableYjsItem(item: Y.Item): Debuggable {
  if (item instanceof Y.Map) {
    return new DebuggableYjsMap(item)
  }

  if (item.content instanceof Y.ContentAny) {
    return debuggableJsValue(item.content.arr[0])
  }

  if (item.content instanceof Y.ContentType) {
    if (item.content.type instanceof Y.Map) {
      return new DebuggableYjsMap(item.content.type)
    } else {
      return new Scalar('unimplemented11')
    }
  }

  if (typeof item === 'string') {
    return new Scalar(item)
  }

  if (Array.isArray(item)) {
    return new JsList(item)
  }

  if (typeof item === 'object') {
    return new JsObject(item)
  }

  console.warn('debuggableYjsItem', item, typeof item)
  return new Scalar('unimplemented')
}

export class DebuggableYjsMap implements Debuggable {
  constructor(private readonly _item: Y.Map<any>) {}

  type(): EntityType {
    return 'object'
  }

  typeName(): string {
    return 'Y.Map'
  }

  entries(): DebuggableEntry[] {
    return Array.from(this._item._map.entries())
      .filter((v) => !(v[1].content instanceof Y.ContentDeleted))
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([key, value]) => ({ key, value: debuggableYjsItem(value) }))
  }

  value(): any {
    return null
  }

  size(): number {
    return this._item._map.size
  }

  listen(listener: () => void): () => void {
    this._item.observe(listener)

    return () => {
      this._item.unobserve(listener)
    }
  }
}

export class DebuggableYjsArray implements Debuggable {
  constructor(private readonly _array: Y.Array<any>) {}

  type(): EntityType {
    return 'list'
  }

  typeName(): string {
    return 'Y.Array'
  }

  entries(): DebuggableEntry[] {
    return this._array.map((value, index) => ({ key: index, value: debuggableYjsItem(value) }))
  }

  value(): any {
    return null
  }

  size(): number {
    return 0
  }

  listen(listener: () => void): () => void {
    this._array.observe(listener)

    return () => {
      this._array.unobserve(listener)
    }
  }
}

type QuillDeltaEntry = {
  insert: string
  attributes?: {
    [key: string]: any
  }
}

export class DebuggableYjsText implements Debuggable {
  constructor(private readonly _text: Y.Text) {}

  type(): EntityType {
    return 'list'
  }

  typeName(): string {
    return 'Y.Text'
  }

  entries(): DebuggableEntry[] {
    const delta: QuillDeltaEntry[] = this._text.toDelta()
    return delta.map((value, index) => ({ key: index, value: debuggableJsValue(value) }))
  }

  value(): any {
    return this._text.toDelta()
  }

  size(): number {
    return 0
  }

  listen(listener: () => void): () => void {
    this._text.observe(listener)

    return () => {
      this._text.unobserve(listener)
    }
  }
}
