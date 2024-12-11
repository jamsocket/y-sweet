import { Doc } from 'yjs'
import * as Y from 'yjs'
import { getOrCreateKey } from './keystore'
import { decryptData, encryptData } from './encryption'

const DB_PREFIX = 'y-sweet-'
const OBJECT_STORE_NAME = 'updates'

/** Maximum number of independent updates to store in IndexedDB.
 * If this is exceeded, all of the updates are compacted into one.
 */
const MAX_UPDATES_IN_STORE = 10

function openIndexedDB(name: string): Promise<IDBDatabase> {
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(name, 2)
    request.onupgradeneeded = (event: IDBVersionChangeEvent) => {
      const db = (event.target! as any).result as IDBDatabase
      db.createObjectStore(OBJECT_STORE_NAME, { autoIncrement: true })
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = reject
  })
}

export class IndexedDBProvider {
  db: Promise<IDBDatabase>
  objectCount: number = 0
  encryptionKey: Promise<CryptoKey>

  constructor(
    private doc: Doc,
    private docId: string,
  ) {
    this.db = openIndexedDB(DB_PREFIX + this.docId)
    this.init()

    this.encryptionKey = getOrCreateKey()

    this.handleUpdate = this.handleUpdate.bind(this)
    doc.on('update', this.handleUpdate)
  }

  async init() {
    // load keys from IndexedDB into doc
    const updates = await this.getAllValues()

    this.objectCount = updates.length

    this.doc.transact(() => {
      for (const update of updates) {
        Y.applyUpdate(this.doc, update)
      }
    }, this)
  }

  destroy() {
    this.doc.off('update', this.handleUpdate)
  }

  async handleUpdate(update: Uint8Array, origin: any) {
    if (origin === this) {
      return
    }

    await this.setValue(update)

    this.objectCount += 1
    if (this.objectCount > MAX_UPDATES_IN_STORE) {
      this.saveWholeState()
    }
  }

  async getAllValues(): Promise<Array<Uint8Array>> {
    let db = await this.db
    const transaction = db.transaction(OBJECT_STORE_NAME)
    const objectStore = transaction.objectStore(OBJECT_STORE_NAME)
    const request = objectStore.getAll()
    const key = await this.encryptionKey

    return new Promise<Array<Uint8Array>>(async (resolve, reject) => {
      request.onsuccess = async () => {
        try {
          const decryptedResults = await Promise.all(
            request.result.map((data) => decryptData(data, key)),
          )
          resolve(decryptedResults)
        } catch (error) {
          reject(error)
        }
      }
      request.onerror = reject
    })
  }

  async saveWholeState() {
    const update = Y.encodeStateAsUpdate(this.doc)
    const key = await this.encryptionKey
    const encryptedUpdate = await encryptData(update, key)
    let db = await this.db
    let transaction = db.transaction(OBJECT_STORE_NAME, 'readwrite')
    let objectStore = transaction.objectStore(OBJECT_STORE_NAME)
    objectStore.clear()
    objectStore.add(encryptedUpdate)
  }

  async setValue(value: Uint8Array): Promise<void> {
    let db = await this.db
    const key = await this.encryptionKey
    const encryptedValue = await encryptData(value, key)
    let objectStore = db.transaction(OBJECT_STORE_NAME, 'readwrite').objectStore(OBJECT_STORE_NAME)
    const request = objectStore.put(encryptedValue)

    return new Promise<void>((resolve, reject) => {
      request.onsuccess = () => resolve()
      request.onerror = reject
    })
  }
}
