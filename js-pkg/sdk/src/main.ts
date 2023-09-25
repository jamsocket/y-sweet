export type DocCreationRequest = {
  /** The ID of the document to create. If not provided, a random ID will be generated. */
  doc?: string
}

/**
 * Schema of object returned after a successful document creation.
 */
export type DocCreationResult = {
  /** A unique identifier for the created document. */
  doc: string
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
  doc: string

  /** A string that grants the bearer access to the document. By default, the development server does not require a token. */
  token?: string
}

/** Metadata associated with a {@link YSweetError}. */
export type YSweetErrorPayload =
  | { code: 'ServerRefused'; address: string; port: number; url: string }
  | { code: 'ServerError'; status: number; message: string; url: string }
  | { code: 'NoAuthProvided' }
  | { code: 'InvalidAuthProvided' }
  | { code: 'Unknown'; message: string }

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
  constructor(connectionString?: string) {
    const parsedUrl = new URL(connectionString || 'http://127.0.0.1:8080')

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

  /** Internal helper for making an authorized fetch request to the API.  */
  private async doFetch(url: string, body?: any): Promise<Response> {
    let method = 'GET'
    let headers: [string, string][] = []
    if (this.token) {
      // Tokens come base64 encoded.
      headers.push(['Authorization', `Bearer ${this.token}`])
    }

    if (body !== undefined) {
      headers.push(['Content-Type', 'application/json'])
      body = JSON.stringify(body)
      method = 'POST'
    }

    let result: Response
    url = `${this.baseUrl}/${url}`
    try {
      result = await fetch(url, {
        method,
        body,
        cache: 'no-store',
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

  /**
   * Create a new, empty document on the y-sweet server.
   *
   * @returns A {@link DocCreationResult} object containing the ID of the created document.
   */
  public async createDoc(request?: DocCreationRequest): Promise<DocCreationResult> {
    const result = await this.doFetch('doc/new', request || {})
    if (!result.ok) {
      throw new Error(`Failed to create doc: ${result.status} ${result.statusText}`)
    }
    return result.json()
  }

  /**
   * Get a client token for the given document.
   *
   * If you are using authorization, this is expected to be called from your server
   * after a user has authenticated. The returned token should then be passed to the
   * client.
   *
   * @param docId The ID of the document to get a token for.
   * @param request Metadata associated with the request.
   * @returns
   */
  public async getClientToken(
    docId: string | DocCreationResult,
    request: AuthDocRequest,
  ): Promise<ClientToken> {
    if (typeof docId !== 'string') {
      docId = docId.doc
    }

    const result = await this.doFetch(`doc/${docId}/auth`, request)
    if (!result.ok) {
      throw new Error(`Failed to auth doc ${docId}: ${result.status} ${result.statusText}`)
    }
    return result.json()
  }
}

/** Request to authorize a document. Currently ignored by y-sweet server. */
export type AuthDocRequest = {
  // authorization?: 'none' | 'readonly' | 'full'
  // user_id?: string
  // metadata?: Record<string, any>
}

/**
 * A convenience wrapper around {@link DocumentManager.createDoc} and {@link DocumentManager.getClientToken} for
 * getting a client token for a document, given a value which may be a
 * document ID or `undefined`.
 *
 * @param docId The ID of the document to get a token for. If `undefined`, a new doc is created.
 * @param connectionString A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
 * @returns A {@link ClientToken} object containing the URL and token needed to connect to the document.
 */
export async function getOrCreateDoc(
  docId?: string,
  connectionString?: string,
): Promise<ClientToken> {
  const manager = new DocumentManager(connectionString)

  if (!docId) {
    const result = await manager.createDoc()
    docId = result.doc
  }

  return await manager.getClientToken(docId, {})
}

/**
 * A convenience wrapper around {@link DocumentManager.getClientToken} for getting a client token for a document.
 *
 * @param docId The ID of the document to get a token for.
 * @param request Metadata associated with the request (currently ignored by y-sweet server).
 * @param connectionString A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
 * @returns A {@link ClientToken} object containing the URL and token needed to connect to the document.
 */
export async function getClientToken(
  docId: string | DocCreationResult,
  request: AuthDocRequest,
  connectionString?: string,
): Promise<ClientToken> {
  const manager = new DocumentManager(connectionString)
  return await manager.getClientToken(docId, request)
}

/**
 * A convenience wrapper around {@link DocumentManager.createDoc} for creating a new document.
 *
 * @param connectionString A connection string (starting with `ys://` or `yss://`) referring to a y-sweet server.
 * @returns A {@link DocCreationResult} object containing the ID of the created document.
 */
export async function createDoc(connectionString?: string): Promise<DocCreationResult> {
  const manager = new DocumentManager(connectionString)
  return await manager.createDoc()
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
