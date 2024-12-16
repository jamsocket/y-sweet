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
const MAX_UPDATES_IN_STORE = 5

/** Pair of key and value, used both for the encrypted entry and decrypted entry. */
interface BytesWithKey {
  key: number
  value: Uint8Array
}

function openIndexedDB(name: string): Promise<IDBDatabase> {
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(name, 4)
    request.onupgradeneeded = (event: IDBVersionChangeEvent) => {
      const db = (event.target! as any).result as IDBDatabase
      db.createObjectStore(OBJECT_STORE_NAME, { keyPath: 'key' })
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = reject
  })
}

export async function createIndexedDBProvider(doc: Doc, docId: string): Promise<IndexedDBProvider> {
  const db = await openIndexedDB(DB_PREFIX + docId)
  const encryptionKey = await getOrCreateKey()

  let provider = new IndexedDBProvider(doc, docId, db, encryptionKey)
  return provider
}

export class IndexedDBProvider {
  objectCount: number = 0
  lastUpdateKey: number = -1
  broadcastChannel: BroadcastChannel

  constructor(
    private doc: Doc,
    docId: string,
    private db: IDBDatabase,
    private encryptionKey: CryptoKey,
  ) {
    this.handleUpdate = this.handleUpdate.bind(this)
    doc.on('update', this.handleUpdate)

    this.broadcastChannel = new BroadcastChannel(`y-sweet-${docId}`)

    this.broadcastChannel.onmessage = (event) => {
      if (event.data > this.lastUpdateKey) {
        this.loadFromDb()
      }
    }

    this.loadFromDb()
  }

  private updateKey() {
    this.lastUpdateKey += 1
    return this.lastUpdateKey
  }

  async loadFromDb() {
    let range = IDBKeyRange.lowerBound(this.lastUpdateKey, true)
    const updates = await this.getAllValues(range)

    this.objectCount = updates.length

    this.doc.transact(() => {
      for (const update of updates) {
        Y.applyUpdate(this.doc, update.value)

        this.lastUpdateKey = update.key
      }
    }, this)
  }

  destroy() {
    this.doc.off('update', this.handleUpdate)
    this.broadcastChannel.close()
  }

  async handleUpdate(update: Uint8Array, origin: any) {
    if (origin === this) {
      return
    }

    await this.loadFromDb()
    await this.insertValue(update)

    this.objectCount += 1
    if (this.objectCount > MAX_UPDATES_IN_STORE) {
      this.saveWholeState()
    }

    this.broadcastChannel.postMessage(this.lastUpdateKey)
  }

  async getAllValues(range?: IDBKeyRange): Promise<Array<BytesWithKey>> {
    let transaction = this.db.transaction(OBJECT_STORE_NAME)
    let objectStore = transaction.objectStore(OBJECT_STORE_NAME)
    const request = objectStore.getAll(range)

    let result = await new Promise<Array<BytesWithKey>>((resolve, reject) => {
      request.onsuccess = async () => {
        try {
          resolve(request.result)
        } catch (error) {
          reject(error)
        }
      }
      request.onerror = reject
    })

    return await Promise.all(
      result.map(async (data) => {
        let value = await decryptData(data.value, this.encryptionKey)

        return {
          key: data.key,
          value,
        }
      }),
    )
  }

  async saveWholeState() {
    const update = Y.encodeStateAsUpdate(this.doc)
    const encryptedUpdate = await encryptData(update, this.encryptionKey)
    let transaction = this.db.transaction(OBJECT_STORE_NAME, 'readwrite')
    let objectStore = transaction.objectStore(OBJECT_STORE_NAME)

    let updateKey = this.updateKey()

    let range = IDBKeyRange.upperBound(updateKey, false)
    objectStore.delete(range)

    objectStore.add({
      key: updateKey,
      value: encryptedUpdate,
    })
  }

  async insertValue(value: Uint8Array): Promise<void> {
    const encryptedValue = await encryptData(value, this.encryptionKey)
    let objectStore = this.db
      .transaction(OBJECT_STORE_NAME, 'readwrite')
      .objectStore(OBJECT_STORE_NAME)

    let updateKey = this.updateKey()

    const request = objectStore.put({
      key: updateKey,
      value: encryptedValue,
    })

    await new Promise<void>((resolve, reject) => {
      request.onsuccess = () => resolve()
      request.onerror = reject
    })
  }
}
