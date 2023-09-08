import * as Y from 'yjs'
import { Debuggable, DebuggableEntry, EntityType } from '.'
import { DebuggableYjsMap, debuggableYjsItem } from './yjs'

export class DebuggableYDoc implements Debuggable {
  keys: string[]

  constructor(private readonly _doc: Y.Doc) {
    this.keys = Array.from(this._doc.share.keys())
  }

  type(): EntityType {
    return 'object'
  }

  typeName(): string {
    return 'Y.Doc'
  }

  entries(): DebuggableEntry[] {
    return this.keys.map((key) => {
      let abstractItem = this._doc.get(key)

      if (abstractItem._start !== null) {
        let array = this._doc.getArray(key)
        return { key, value: new DebuggableYjsArrayOrText(array) }
      } else {
        let map = this._doc.getMap(key)
        return { key, value: new DebuggableYjsMap(map) }
      }
    })
  }

  value(): any {
    return null
  }

  size(): number {
    return this._doc.getMap().size
  }

  listen(listener: () => void): () => void {
    // We can only listen for EVERY change to a document,
    // but we only want to know when a top-level key has been
    // added. So on every update, we make a list of the keys
    // and check if it's changed.
    let callback = () => {
      let keys = Array.from(this._doc.share.keys())
      // Since keys can only be added, we can simply compare the
      // lengths to know if it has changed.
      if (keys.length !== this.keys.length) {
        this.keys = keys
        listener()
      }
    }

    this._doc.on('update', callback)

    return () => {
      this._doc.off('update', callback)
    }
  }
}

function collectLinkedList(node: Y.Item | null): Y.Item[] {
  const result: Y.Item[] = []
  while (node !== null) {
    if (!(node.content instanceof Y.ContentDeleted)) {
      result.push(node)
    }
    node = node.right
  }
  return result
}

class DebuggableYjsArrayOrText implements Debuggable {
  type(): EntityType {
    return this.displayAsText ? 'text' : 'list'
  }

  typeName(): string {
    return this.displayAsText ? 'Y.Text' : 'Y.Array'
  }

  listener?: () => void

  displayAsText: boolean

  constructor(private readonly _item: Y.Array<any>) {
    this.displayAsText = false
  }

  toggleType() {
    this.displayAsText = !this.displayAsText
    if (this.listener) {
      this.listener()
    }
  }

  entries(): DebuggableEntry[] {
    if (!this.displayAsText) {
      return this._item.map((value, index) => ({ key: index, value: debuggableYjsItem(value) }))
    }
    return []
  }

  value() {
    if (this.displayAsText) {
      let items = collectLinkedList(this._item._start)
      return items
    }
    return null
  }

  size(): number {
    return 0
  }

  listen(listener: () => void): () => void {
    this.listener = listener
    this._item.observe(listener)

    return () => {
      this.listener = undefined
      this._item.unobserve(listener)
    }
  }
}
