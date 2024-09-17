import { HttpClient } from './http'
import { ClientToken } from './types'

export class YSweetConnection {
  private client: HttpClient

  constructor(clientToken: ClientToken) {
    this.client = new HttpClient(clientToken.url, clientToken.token ?? null)
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
   * @param docId
   * @returns
   */
  public async getDocAsUpdate(docId: string): Promise<Uint8Array> {
    const result = await this.client.request(`doc/${docId}/as-update`, 'GET')
    if (!result.ok) {
      throw new Error(`Failed to get doc ${docId}: ${result.status} ${result.statusText}`)
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
   * @param docId
   * @param update
   */
  public async updateDoc(docId: string, update: Uint8Array): Promise<void> {
    const result = await this.client.request(`doc/${docId}/update`, 'POST', update)

    if (!result.ok) {
      throw new Error(`Failed to update doc ${docId}: ${result.status} ${result.statusText}`)
    }
  }
}
