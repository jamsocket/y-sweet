import { Doc } from 'yjs'
import * as Y from 'yjs'
import { getOrCreateKey } from './keystore'
import { decryptData, encryptData } from './encryption'

const DB_PREFIX = 'y-sweet-'
const OBJECT_STORE_NAME = 'updates'

/**
 * Maximum number of independent updates to store in IndexedDB.
 * If this is exceeded, all of the updates are compacted into one.
 */
const MAX_UPDATES_IN_STORE = 50

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

export async function createIndexedDBProvider(doc: Doc, docId: string): Promise<IndexedDBProvider> {
  const db = await openIndexedDB(DB_PREFIX + docId)
  const encryptionKey = await getOrCreateKey()

  let provider = new IndexedDBProvider(doc, db, encryptionKey)
  return provider
}

export class IndexedDBProvider {
  objectCount: number = 0

  constructor(
    private doc: Doc,
    private db: IDBDatabase,
    private encryptionKey: CryptoKey,
  ) {
    this.handleUpdate = this.handleUpdate.bind(this)
    doc.on('update', this.handleUpdate)

    this.init()
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
    const transaction = this.db.transaction(OBJECT_STORE_NAME)
    const objectStore = transaction.objectStore(OBJECT_STORE_NAME)
    const request = objectStore.getAll()

    let result = await new Promise<Array<Uint8Array>>((resolve, reject) => {
      request.onsuccess = async () => {
        try {
          resolve(request.result)
        } catch (error) {
          reject(error)
        }
      }
      request.onerror = reject
    })

    return await Promise.all(result.map((data) => decryptData(data, this.encryptionKey)))
  }

  async saveWholeState() {
    const update = Y.encodeStateAsUpdate(this.doc)
    const encryptedUpdate = await encryptData(update, this.encryptionKey)
    let transaction = this.db.transaction(OBJECT_STORE_NAME, 'readwrite')
    let objectStore = transaction.objectStore(OBJECT_STORE_NAME)
    objectStore.clear()
    objectStore.add(encryptedUpdate)
  }

  async setValue(value: Uint8Array): Promise<void> {
    const encryptedValue = await encryptData(value, this.encryptionKey)
    let objectStore = this.db
      .transaction(OBJECT_STORE_NAME, 'readwrite')
      .objectStore(OBJECT_STORE_NAME)
    const request = objectStore.put(encryptedValue)

    return new Promise<void>((resolve, reject) => {
      request.onsuccess = () => resolve()
      request.onerror = reject
    })
  }
}
