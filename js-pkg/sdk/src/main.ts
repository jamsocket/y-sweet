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

export class DocumentManager {
  baseUrl: string
  token?: string

  constructor(options?: ServerToken | string) {
    if (typeof options === 'string') {
      options = JSON.parse(options) as ServerToken
    }

    this.baseUrl = options?.url ?? 'http://127.0.0.1:8080'
    // Remove trailing slash.
    this.baseUrl = this.baseUrl.replace(/\/$/, '')
    this.token = options?.token
  }

  async doFetch(url: string, body?: any): Promise<Response> {
    let method = 'GET'
    let headers: [string, string][] = []
    if (this.token) {
      // let tokenBuffer = Buffer.from(this.token)
      // headers.push(['Authorization', `Bearer ${tokenBuffer.toString('base64')}`])
      // Tokens now come base64 encoded.
      headers.push(['Authorization', `Bearer ${this.token}`])
    }

    if (body !== undefined) {
      headers.push(['Content-Type', 'application/json'])
      body = JSON.stringify(body)
      method = 'POST'
    }

    const result = await fetch(`${this.baseUrl}/${url}`, {
      method,
      body,
      cache: 'no-store',
      headers,
    })
    if (!result.ok) {
      throw new Error(`Failed to fetch ${url}: ${result.status} ${result.statusText}`)
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
  options?: ServerToken | string,
): Promise<ClientToken> {
  const manager = new DocumentManager(options)
  return await manager.getOrCreateDoc(docId)
}

export async function getClientToken(
  docId: string | DocCreationResult,
  request: AuthDocRequest,
  options?: ServerToken | string,
): Promise<ClientToken> {
  const manager = new DocumentManager(options)
  return await manager.getClientToken(docId, request)
}

export async function createDoc(
  options?: ServerToken | string,
): Promise<DocCreationResult> {
  const manager = new DocumentManager(options)
  return await manager.createDoc()
}
