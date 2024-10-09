import { DocConnection } from './connection'
export { DocConnection } from './connection'
import { HttpClient } from './http'
import type { DocCreationResult, ClientToken, CheckStoreResult } from './types'
export type { DocCreationResult, ClientToken, CheckStoreResult } from './types'
export { type YSweetErrorPayload, YSweetError } from './error'
export { encodeClientToken, decodeClientToken } from './encoding'

/** Represents an interface to a y-sweet document management endpoint. */
export class DocumentManager {
  /** Wraps a fetch request with authorization and error handling. */
  private client: HttpClient

  /**
   * Create a new {@link DocumentManager}.
   *
   * @param serverToken A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
   */
  constructor(connectionString: string) {
    const parsedUrl = new URL(connectionString)

    let token = null
    if (parsedUrl.username) {
      // Decode the token from the URL.
      token = decodeURIComponent(parsedUrl.username)
    }

    let protocol = parsedUrl.protocol
    if (protocol === 'ys:') {
      protocol = 'http:'
    } else if (protocol === 'yss:') {
      protocol = 'https:'
    }

    // NB: we manually construct the string here because node's URL implementation does
    //     not handle changing the protocol of a URL well.
    //     see: https://nodejs.org/api/url.html#urlprotocol
    const url = `${protocol}//${parsedUrl.host}${parsedUrl.pathname}`
    let baseUrl = url.replace(/\/$/, '') // Remove trailing slash

    this.client = new HttpClient(baseUrl, token)
  }

  public async checkStore(): Promise<CheckStoreResult> {
    return await (await this.client.request('check_store', 'POST', {})).json()
  }

  /**
   * Creates a new document on the y-sweet server given an optional docId. If a document with given
   * ID already exists, this is a no-op.
   *
   * @param docId The ID of the document to be created. If not provided, a random ID will be generated.
   * @returns A {@link DocCreationResult} object containing the ID of the created document.
   */
  public async createDoc(docId?: string): Promise<DocCreationResult> {
    const body = docId ? { docId } : {}
    const result = await this.client.request('doc/new', 'POST', body)
    if (!result.ok) {
      throw new Error(`Failed to create doc: ${result.status} ${result.statusText}`)
    }
    const responseBody = (await result.json()) as DocCreationResult
    return responseBody
  }

  /**
   * Get a client token for the given document.
   *
   * If you are using authorization, this is expected to be called from your server
   * after a user has authenticated. The returned token should then be passed to the
   * client.
   *
   * @param docId The ID of the document to get a token for.
   * @returns A {@link ClientToken} object containing the URL and token needed to connect to the document.
   */
  public async getClientToken(docId: string | DocCreationResult): Promise<ClientToken> {
    if (typeof docId !== 'string') {
      docId = docId.docId
    }

    const result = await this.client.request(`doc/${docId}/auth`, 'POST', {})
    if (!result.ok) {
      throw new Error(`Failed to auth doc ${docId}: ${result.status} ${result.statusText}`)
    }
    const responseBody = (await result.json()) as ClientToken
    return responseBody
  }

  /**
   * A convenience wrapper around {@link DocumentManager.createDoc} and {@link DocumentManager.getClientToken} for
   * getting a client token for a document. If a docId is provided, ensures that a document exists with that ID or
   * that one is created. If no docId is provided, a new document is created with a random ID.
   *
   * @param docId The ID of the document to get or create. If not provided, a new document with a random ID will be created.
   * @returns A {@link ClientToken} object containing the URL and token needed to connect to the document.
   */
  public async getOrCreateDocAndToken(docId?: string): Promise<ClientToken> {
    const result = await this.createDoc(docId)
    return await this.getClientToken(result)
  }

  /**
   * Returns an entire document, represented as a Yjs update byte string.
   *
   * @param docId The ID of the document to get.
   * @returns The document as a Yjs update byte string
   */
  public async getDocAsUpdate(docId: string): Promise<Uint8Array> {
    const connection = await this.getDocConnection(docId)
    return await connection.getAsUpdate()
  }

  /**
   * Updates a document with the given Yjs update byte string.
   *
   * @param docId The ID of the document to update.
   * @param update The Yjs update byte string to apply to the document.
   */
  public async updateDoc(docId: string, update: Uint8Array): Promise<void> {
    const connection = await this.getDocConnection(docId)
    return await connection.updateDoc(update)
  }

  public async getDocConnection(docId: string): Promise<DocConnection> {
    const clientToken = await this.getClientToken(docId)
    return new DocConnection(clientToken)
  }
}

/**
 * A convenience wrapper around {@link DocumentManager.getOrCreateDocAndToken} for getting or creating a document
 * with the given ID and returning a client token for accessing it.
 *
 * @param connectionString A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
 * @param docId The ID of the document to get or create. If not provided, a new document with a random ID will be created.
 * @returns A {@link ClientToken} object containing the URL and token needed to connect to the document.
 */
export async function getOrCreateDocAndToken(
  connectionString: string,
  docId?: string,
): Promise<ClientToken> {
  const manager = new DocumentManager(connectionString)
  return await manager.getOrCreateDocAndToken(docId)
}

/**
 * A convenience wrapper around {@link DocumentManager.getClientToken} for getting a client token for a document.
 *
 * @param connectionString A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
 * @param docId The ID of the document to get a token for.
 * @returns A {@link ClientToken} object containing the URL and token needed to connect to the document.
 */
export async function getClientToken(
  connectionString: string,
  docId: string | DocCreationResult,
): Promise<ClientToken> {
  const manager = new DocumentManager(connectionString)
  return await manager.getClientToken(docId)
}

/**
 * A convenience wrapper around {@link DocumentManager.createDoc} for creating a new document. If a document with the
 * given ID already exists, this is a no-op.
 *
 * @param connectionString A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
 * @param docId The ID of the document to create. If not provided, a random ID will be generated.
 * @returns A {@link DocCreationResult} object containing the ID of the created document.
 */
export async function createDoc(
  connectionString: string,
  docId?: string,
): Promise<DocCreationResult> {
  const manager = new DocumentManager(connectionString)
  return await manager.createDoc(docId)
}
