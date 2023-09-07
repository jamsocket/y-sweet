import * as Y from 'yjs'
import { Debuggable, DebuggableEntry, EntityType } from '.'
import { DebuggableYjsMap, debuggableYjsItem } from './yjs'

/**
 * Return a view of a top-level Yjs type as the given constructor.
 *
 * viewAsType(doc, doc.get('foo'), Y.Array) is equivalent to doc.getArray('foo),
 * except that it does not associate 'foo' with the Array type in the document.
 * This ensures that viewAsType(doc, doc.get('foo'), Y.Map) can later succeed,
 * whereas doc.getMap('foo') after doc.getArray('foo') will throw an error.
 *
 * Adapted from:
 * https://github.com/yjs/yjs/blob/a1fda219e4af6466e2446e71aaa8afe050cd47fc/src/utils/Doc.js#L207
 *
 * @param doc
 * @param value
 * @param TypeConstructor
 * @returns
 */
function viewAsType<T extends Y.AbstractType<any>>(
  doc: Y.Doc,
  value: Y.AbstractType<any>,
  constructor: new () => T,
): T {
  const t = new constructor()
  t._map = value._map
  value._map.forEach((initial) => {
    for (let n: Y.Item | null = initial; n !== null; n = n.left) {
      n.parent = t
    }
  })
  t._start = value._start
  for (let n = t._start; n !== null; n = n.right) {
    n.parent = t
  }
  t._length = value._length
  t._integrate(doc, null)
  return t
}

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
        let item = this._doc.get(key)
        let array = viewAsType<Y.Array<any>>(this._doc, item, Y.Array)
        return { key, value: new DebuggableYjsArrayOrText(array) }

        // let text = viewAsType<Y.Text>(this._doc, item, Y.Text)
        // return { key, value: new DebuggableYjsText(text) }
      } else {
        let item = this._doc.getMap(key)
        let map = viewAsType<Y.Map<any>>(this._doc, item, Y.Map)
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
    return this.displayAsText ? 'text' : 'array'
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
