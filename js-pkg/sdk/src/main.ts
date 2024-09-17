import { YSweetError } from './error'
import type { DocCreationResult, ClientToken, CheckStoreResult } from './types'
export type { DocCreationResult, ClientToken, CheckStoreResult } from './types'
export { type YSweetErrorPayload, YSweetError } from './error'
export { encodeClientToken, decodeClientToken } from './encoding'

function generateRandomString(): string {
  return Math.random().toString(36).substring(2)
}

/** Represents an interface to a y-sweet document management endpoint. */
export class DocumentManager {
  /** The base URL of the remote document manager API. */
  private baseUrl: string

  /** A string that grants the bearer access to the document management API. */
  private token?: string

  /**
   * Create a new {@link DocumentManager}.
   *
   * @param serverToken A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
   */
  constructor(connectionString: string) {
    const parsedUrl = new URL(connectionString)

    let token
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
    const url = `${protocol}//${parsedUrl.host}${parsedUrl.pathname}${parsedUrl.search}`

    this.baseUrl = url.replace(/\/$/, '')
    this.token = token
  }

  private async doFetch(url: string, method: 'GET'): Promise<Response>
  private async doFetch(url: string, method: 'POST', body: Record<string, any>): Promise<Response>

  /** Internal helper for making an authorized fetch request to the API.  */
  private async doFetch(
    url: string,
    method: 'GET' | 'POST',
    body?: Record<string, any>,
  ): Promise<Response> {
    let headers: [string, string][] = []
    if (this.token) {
      // Tokens come base64 encoded.
      headers.push(['Authorization', `Bearer ${this.token}`])
    }

    let bodyJson
    if (method === 'POST') {
      headers.push(['Content-Type', 'application/json'])
      bodyJson = JSON.stringify(body)
    }

    let result: Response

    // NOTE: In some environments (e.g. NextJS), responses are cached by default. Disabling
    // the cache using `cache: 'no-store'` causes fetch() to error in other environments
    // (e.g. Cloudflare Workers). To work around this, we simply add a cache-busting query
    // param.
    const cacheBust = generateRandomString()
    url = `${this.baseUrl}/${url}?z=${cacheBust}`
    try {
      result = await fetch(url, {
        method,
        body: bodyJson,
        headers,
      })
    } catch (error: any) {
      if (error.cause?.code === 'ECONNREFUSED') {
        let { address, port } = error.cause
        throw new YSweetError({ code: 'ServerRefused', address, port, url })
      } else {
        throw new YSweetError({ code: 'Unknown', message: error.toString() })
      }
    }

    if (!result.ok) {
      if (result.status === 401) {
        if (this.token) {
          throw new YSweetError({ code: 'InvalidAuthProvided' })
        } else {
          throw new YSweetError({ code: 'NoAuthProvided' })
        }
      }

      throw new YSweetError({
        code: 'ServerError',
        status: result.status,
        message: result.statusText,
        url,
      })
    }

    return result
  }

  public async checkStore(): Promise<CheckStoreResult> {
    return await (await this.doFetch('check_store', 'GET')).json()
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
    const result = await this.doFetch('doc/new', 'POST', body)
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

    const result = await this.doFetch(`doc/${docId}/auth`, 'POST', {})
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
    const result = await this.doFetch(`doc/${docId}/as-update`, 'GET')
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
    let headers: [string, string][] = [['Content-Type', 'application/octet-stream']]
    if (this.token) {
      headers.push(['Authorization', `Bearer ${this.token}`])
    }

    const result = await fetch(`${this.baseUrl}/doc/${docId}/update`, {
      method: 'POST',
      body: update,
      headers,
    })

    if (!result.ok) {
      throw new Error(`Failed to update doc ${docId}: ${result.status} ${result.statusText}`)
    }
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
