export type DocCreationResult = {
    doc_id: string
}

export type DocumentManagerOptions = {
    endpoint?: string,
    token?: string,
}

export class DocumentManager {
    baseUrl: string
    token?: string

    constructor(options?: DocumentManagerOptions) {
        this.baseUrl = options?.endpoint || 'http://127.0.0.1:8787'
        this.token = options?.token
    }

    async doFetch(url: string, body?: any): Promise<Response> {
        let method = 'GET'
        let headers: [string, string][] = []
        if (this.token) {
            let tokenBuffer = Buffer.from(this.token)
            headers.push(['Authorization', `Bearer ${tokenBuffer.toString('base64')}`])
        }

        if (body !== undefined) {
            headers.push(['Content-Type', 'application/json'])
            body = JSON.stringify(body)
            method = 'POST'
        }
        
        const result = await fetch(`${this.baseUrl}/${url}`, {method, body, cache: 'no-store', headers})
        if (!result.ok) {
            throw new Error(`Failed to fetch ${url}: ${result.status} ${result.statusText}`)
        }

        return result
    }

    public async createDoc(): Promise<DocCreationResult> {
        const result = await this.doFetch('doc/new', {method: 'POST'})
        if (!result.ok) {
            throw new Error(`Failed to create doc: ${result.status} ${result.statusText}`)
        }
        return result.json()
    }

    public async getOrCreateDoc(docId?: string): Promise<ConnectionKey> {
        if (!docId) {
            let room = await this.createDoc()
            docId = room.doc_id
        }
    
        return await this.getConnectionKey(docId, {})
    }

    public async getConnectionKey(docId: string, request: AuthDocRequest): Promise<ConnectionKey> {
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
    authorization?: "none" | "readonly" | "full",
    user_id?: string,
    metadata?: Record<string, any>,
}

export type ConnectionKey = {
    base_url: string,
    doc_id: string,
    token?: string,
}

export async function getOrCreateDoc(docId?: string, options?: DocumentManagerOptions): Promise<ConnectionKey> {
    const manager = new DocumentManager(options)
    return await manager.getOrCreateDoc(docId)
}

export async function getConnectionKey(docId: string, request: AuthDocRequest, options?: DocumentManagerOptions): Promise<ConnectionKey> {
    const manager = new DocumentManager(options)
    return await manager.getConnectionKey(docId, request)
}

export async function createDoc(options?: DocumentManagerOptions): Promise<DocCreationResult> {
    const manager = new DocumentManager(options)
    return await manager.createDoc()
}
