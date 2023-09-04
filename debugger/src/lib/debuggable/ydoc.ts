import * as Y from 'yjs'
import { Debuggable, DebuggableEntry, EntityType } from '.'
import { DebuggableYjsArray, DebuggableYjsMap } from './yjs'

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
function viewAsType<T extends Y.AbstractType<any>>(doc: Y.Doc, value: Y.AbstractType<any>, constructor: new () => T): T {
    const t = new constructor()
    t._map = value._map
    value._map.forEach(initial => {
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
  
    type: EntityType = 'object'
    typeName = 'Y.Doc'
  
    entries(): DebuggableEntry[] {
      return this.keys.map((key) => {
        let abstractItem = this._doc.get(key)
  
        if (abstractItem._start !== null) {
          let item = this._doc.get(key)
          let array = viewAsType<Y.Array<any>>(this._doc, item, Y.Array)
          return { key, value: new DebuggableYjsArray(array) }
          
  
          // let item = this._doc.getText(key)
          // return { key, value: new DebuggableYjsText(item) }
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
  