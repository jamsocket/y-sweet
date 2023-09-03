import * as Y from 'yjs'
import { Scalar, debuggableJsValue } from './builtins'
import { Debuggable, DebuggableEntry, EntityType } from '.'

export class DebuggableYDoc implements Debuggable {
  constructor(private readonly _doc: Y.Doc) {}

  type: EntityType = 'object'
  typeName = 'Y.Doc'

  entries(): DebuggableEntry[] {
    const keys = Array.from(this._doc.share.keys())
    return keys.map((key) => {
      let abstractItem = this._doc.get(key)

      if (abstractItem._start !== null) {
        let item = this._doc.getArray(key)
        return { key, value: new DebuggableYjsArray(item) }

        // let item = this._doc.getText(key)
        // return { key, value: new DebuggableYjsText(item) }
      } else {
        let item = this._doc.getMap(key)
        return { key, value: new DebuggableYjsMap(item) }
      }
    })
  }

  value(): any {
    return null
  }

  size(): number {
    return this._doc.getMap().size
  }
}

function debuggableYjsItem(item: Y.Item): Debuggable {
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

  console.warn('debuggableYjsItem', item, typeof item)
  return new Scalar('unimplemented')
}

class DebuggableYjsMap implements Debuggable {
  constructor(private readonly _item: Y.Map<any>) {}

  type: EntityType = 'object'
  typeName = 'Y.Map'

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
}

class DebuggableYjsArray implements Debuggable {
  constructor(private readonly _array: Y.Array<any>) {}

  type: EntityType = 'list'
  typeName = 'Y.Array'

  entries(): DebuggableEntry[] {
    return this._array.map((value, index) => ({ key: index, value: debuggableYjsItem(value) }))
  }

  value(): any {
    return null
  }

  size(): number {
    return 0
  }
}

type QuillDeltaEntry = {
  insert: string
  attributes?: {
    [key: string]: any
  }
}

class DebuggableYjsText implements Debuggable {
  constructor(private readonly _text: Y.Text) {}

  type: EntityType = 'list'
  typeName = 'Y.Text'

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
}
