/**
 * Schema of object returned after a successful document creation.
 */
export type DocCreationResult = {
  /** A unique identifier for the created document. */
  docId: string
}

/**
 * An object containing information needed for the client connect to a document.
 *
 * This value is expected to be passed from your server to your client. Your server
 * should obtain this value by calling {@link DocumentManager.getClientToken},
 * and then pass it to the client.
 */
export type ClientToken = {
  /** The bare URL of the WebSocket endpoint to connect to. The `doc` string will be appended to this. */
  url: string

  /** A unique identifier for the document that the token connects to. */
  docId: string

  /** A string that grants the bearer access to the document. By default, the development server does not require a token. */
  token?: string
}

function generateRandomString(): string {
  return Math.random().toString(36).substring(2)
}

/** Metadata associated with a {@link YSweetError}. */
export type YSweetErrorPayload =
  | { code: 'ServerRefused'; address: string; port: number; url: string }
  | { code: 'ServerError'; status: number; message: string; url: string }
  | { code: 'NoAuthProvided' }
  | { code: 'InvalidAuthProvided' }
  | { code: 'Unknown'; message: string }

export type CheckStoreResult = { ok: true } | { ok: false; error: string }

/** An error returned by the y-sweet SDK. */
export class YSweetError extends Error {
  /**
   * Create a new {@link YSweetError}.
   *
   * @param cause An object representing metadata associated with the error.
   * @see {@link YSweetErrorPayload}
   */
  constructor(public cause: YSweetErrorPayload) {
    super(YSweetError.getMessage(cause))
    this.name = 'YSweetError'
  }

  /** Convert the message to an error string that can be displayed to the user.
   *
   * The error string can also be used with {@link YSweetError.fromMessage} to
   * reconstruct the payload object, which is useful in the context of Next.js,
   * which will only pass an error string from the server to the client.
   *
   * @param payload The payload object to convert to a string.
   * @returns A string representation of the error.
   */
  static getMessage(payload: YSweetErrorPayload): string {
    let message
    if (payload.code === 'ServerRefused') {
      message = `Server at ${payload.address}:${payload.port} refused connection. URL: ${payload.url}`
    } else if (payload.code === 'ServerError') {
      message = `Server responded with ${payload.status} ${payload.message}. URL: ${payload.url}`
    } else if (payload.code === 'NoAuthProvided') {
      message = 'No auth provided'
    } else if (payload.code === 'InvalidAuthProvided') {
      message = 'Invalid auth provided'
    } else {
      message = payload.message
    }
    return `${payload.code}: ${message}`
  }

  /**
   * In development, next.js passes error objects to the client but strips out everything but the
   * `message` field. This method allows us to reconstruct the original error object.
   *
   * @param messageString The error message string to reconstruct a payload from.
   * @returns A {@link YSweetError} object.
   * @see {@link https://nextjs.org/docs/app/api-reference/file-conventions/error#errormessage| Next.js docs}
   */
  static fromMessage(messageString: string): YSweetError {
    let match = messageString.match(/^(.*?): (.*)$/)
    if (!match) {
      return new YSweetError({ code: 'Unknown', message: messageString })
    }

    let [, code, message] = match

    if (code === 'ServerRefused') {
      match = message.match(/^Server at (.*?):(\d+) refused connection. URL: (.*)$/)
      if (!match) {
        return new YSweetError({ code: 'Unknown', message: messageString })
      }

      let [, address, port, url] = match
      return new YSweetError({ code, address, port: parseInt(port), url })
    }

    if (code === 'ServerError') {
      match = message.match(/^Server responded with (\d+) (.*). URL: (.*)$/)
      if (!match) {
        return new YSweetError({ code: 'Unknown', message: messageString })
      }

      let [, status, statusText, url] = match
      return new YSweetError({ code, status: parseInt(status), message: statusText, url })
    }

    if (code === 'NoAuthProvided') {
      return new YSweetError({ code })
    }

    if (code === 'InvalidAuthProvided') {
      return new YSweetError({ code })
    }

    return new YSweetError({ code: 'Unknown', message })
  }
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

function stringToBase64(input: string) {
  if (typeof window !== 'undefined' && window.btoa) {
    // Browser
    return window.btoa(input)
  } else if (typeof Buffer !== 'undefined') {
    // Node.js
    return Buffer.from(input).toString('base64')
  } else {
    throw new Error('Unable to encode to Base64')
  }
}

function base64ToString(input: string) {
  if (typeof window !== 'undefined' && window.atob) {
    // Browser
    return window.atob(input)
  } else if (typeof Buffer !== 'undefined') {
    // Node.js
    return Buffer.from(input, 'base64').toString()
  } else {
    throw new Error('Unable to decode from Base64')
  }
}

export function encodeClientToken(token: ClientToken): string {
  const jsonString = JSON.stringify(token)
  let base64 = stringToBase64(jsonString)
  base64 = base64.replace('+', '-').replace('/', '_').replace(/=+$/, '')
  return base64
}

export function decodeClientToken(token: string): ClientToken {
  let base64 = token.replace('-', '+').replace('_', '/')
  while (base64.length % 4) {
    base64 += '='
  }
  const jsonString = base64ToString(base64)
  return JSON.parse(jsonString)
}
