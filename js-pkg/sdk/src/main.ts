export type DocCreationResult = {
  doc: string
}

export type ClientToken = {
  url: string
  doc: string
  token?: string
}

export type ServerToken = {
  url?: string
  token?: string
}

export type YSweetErrorPayload =
  | { code: 'ServerRefused'; address: string; port: number }
  | { code: 'ServerError'; status: number; message: string }
  | { code: 'NoAuthProvided' }
  | { code: 'InvalidAuthProvided' }
  | { code: 'Unknown'; message: string }

export class YSweetError extends Error {
  constructor(public cause: YSweetErrorPayload) {
    super(YSweetError.getMessage(cause))
    this.name = 'YSweetError'
  }

  static getMessage(payload: YSweetErrorPayload): string {
    if (payload.code === 'ServerRefused') {
      return `Server at ${payload.address}:${payload.port} refused connection`
    } else if (payload.code === 'ServerError') {
      return `Server responded with ${payload.status} ${payload.message}`
    } else if (payload.code === 'NoAuthProvided') {
      return 'No auth provided'
    } else if (payload.code === 'InvalidAuthProvided') {
      return 'Invalid auth provided'
    } else {
      return payload.message
    }
  }
}

export class DocumentManager {
  baseUrl: string
  token?: string

  constructor(serverToken?: ServerToken | string) {
    if (serverToken === undefined) {
      serverToken = {}
    } else if (typeof serverToken === 'string') {
      const parsedUrl = new URL(serverToken)
      let token
      if (parsedUrl.username) {
        // Decode the token from the URL.
        token = decodeURIComponent(parsedUrl.username)
      }
      parsedUrl.username = ''

      const url = parsedUrl.toString()
      serverToken = {
        url,
        token,
      }
    }

    this.baseUrl = (serverToken.url ?? 'http://127.0.0.1:8080').replace(/\/$/, '')
    this.token = serverToken.token
    console.log(`Using server URL ${this.baseUrl}`)
  }

  async doFetch(url: string, body?: any): Promise<Response> {
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
    try {
      result = await fetch(`${this.baseUrl}/${url}`, {
        method,
        body,
        cache: 'no-store',
        headers,
      })
    } catch (error: any) {
      if (error.cause?.code === 'ECONNREFUSED') {
        let { address, port } = error.cause
        throw new YSweetError({ code: 'ServerRefused', address, port })
      } else if (error.toString().includes('401 Unauthorized')) {
        if (this.token) {
          throw new YSweetError({ code: 'InvalidAuthProvided' })
        } else {
          throw new YSweetError({ code: 'NoAuthProvided' })
        }
      } else {
        throw new YSweetError({ code: 'Unknown', message: error.toString() })
      }
    }

    if (!result.ok) {
      throw new YSweetError({
        code: 'ServerError',
        status: result.status,
        message: result.statusText,
      })
    }

    return result
  }

  public async createDoc(): Promise<DocCreationResult> {
    const result = await this.doFetch('doc/new', { method: 'POST' })
    if (!result.ok) {
      throw new Error(`Failed to create doc: ${result.status} ${result.statusText}`)
    }
    return result.json()
  }

  public async getOrCreateDoc(docId?: string): Promise<ClientToken> {
    if (!docId) {
      let room = await this.createDoc()
      docId = room.doc
    }

    return await this.getClientToken(docId, {})
  }

  public async getClientToken(
    docId: string | DocCreationResult,
    request: AuthDocRequest,
  ): Promise<ClientToken> {
    if (typeof docId !== 'string') {
      docId = docId.doc
    }

    const result = await this.doFetch(`doc/${docId}/auth`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    })
    if (!result.ok) {
      throw new Error(`Failed to auth doc ${docId}: ${result.status} ${result.statusText}`)
    }
    return result.json()
  }
}

export type AuthDocRequest = {
  authorization?: 'none' | 'readonly' | 'full'
  user_id?: string
  metadata?: Record<string, any>
}

export async function getOrCreateDoc(
  docId?: string,
  serverToken?: ServerToken | string,
): Promise<ClientToken> {
  const manager = new DocumentManager(serverToken)
  return await manager.getOrCreateDoc(docId)
}

export async function getClientToken(
  docId: string | DocCreationResult,
  request: AuthDocRequest,
  serverToken?: ServerToken | string,
): Promise<ClientToken> {
  const manager = new DocumentManager(serverToken)
  return await manager.getClientToken(docId, request)
}

export async function createDoc(serverToken?: ServerToken | string): Promise<DocCreationResult> {
  const manager = new DocumentManager(serverToken)
  return await manager.createDoc()
}
