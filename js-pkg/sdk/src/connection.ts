import { HttpClient } from './http'
import { ClientToken } from './types'

export class DocConnection {
  private client: HttpClient
  private docId: string

  constructor(clientToken: ClientToken) {
    this.client = new HttpClient(clientToken.baseUrl, clientToken.token ?? null)
    this.docId = clientToken.docId
  }

  /**
   * Returns an entire document, represented as a Yjs update byte string.
   *
   * This can be turned back into a Yjs document as follows:
   *
   * ```typescript
   * import * as Y from 'yjs'
   *
   * let update = await manager.getDocAsUpdate(docId)
   * let doc = new Y.Doc()
   * doc.transact(() => {
   *  Y.applyUpdate(doc, update)
   * })
   * ```
   *
   * @returns
   */
  public async getAsUpdate(): Promise<Uint8Array> {
    const result = await this.client.request(`as-update`, 'GET')
    if (!result.ok) {
      throw new Error(`Failed to get doc ${this.docId}: ${result.status} ${result.statusText}`)
    }

    let buffer = await result.arrayBuffer()
    return new Uint8Array(buffer)
  }

  /**
   * Updates a document with the given Yjs update byte string.
   *
   * This can be generated from a Yjs document as follows:
   *
   * ```typescript
   * import * as Y from 'yjs'
   *
   * let doc = new Y.Doc()
   * // Modify the document...
   * let update = Y.encodeStateAsUpdate(doc)
   * await manager.updateDoc(docId, update)
   * ```
   *
   * @param update
   */
  public async updateDoc(update: Uint8Array): Promise<void> {
    const result = await this.client.request(`update`, 'POST', update)

    if (!result.ok) {
      throw new Error(`Failed to update doc ${this.docId}: ${result.status} ${result.statusText}`)
    }
  }
}
