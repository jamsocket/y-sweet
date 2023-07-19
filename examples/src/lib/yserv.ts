export type FetchResult = {
    doc_id: string
}

export async function createRoom(): Promise<FetchResult> {
    const result = await fetch('http://127.0.0.1:8080/doc/new', {
        method: 'POST',
        // cache: 'no-store',
    })
    if (!result.ok) {
        throw new Error(`Failed to create room: ${result.status} ${result.statusText}`)
    }
    return result.json()
}

export type AuthDocRequest = {
    authorization?: "none" | "readonly" | "full",
    user_id?: string,
    metadata?: Record<string, any>,
}

export type AuthDocResponse = {
    base_url: string,
    doc_id: string,
}

export async function authDoc(doc_id: string, request: AuthDocRequest): Promise<AuthDocResponse> {
    const result = await fetch(`http://127.0.0.1:8080/doc/${doc_id}/auth`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
        // cache: 'no-store',
    })
    if (!result.ok) {
        throw new Error(`Failed to auth doc ${doc_id}: ${result.status} ${result.statusText}`)
    }
    return result.json()
}