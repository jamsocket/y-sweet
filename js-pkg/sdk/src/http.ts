import { YSweetError } from "./error"

/** A type that can be used as an HTTP request body.
 * If a `Uint8Array` is provided, the body is sent as a raw binary body
 * with the `Content-Type` header set to `application/octet-stream`.
 * Otherwise, the body is sent as a JSON body with the `Content-Type`
 * header set to `application/json`.
 */
export type BodyType = Record<string, any> | Uint8Array

export class HttpClient {
    private token: string | null = null
    private baseUrl: string

    constructor(baseUrl: string, token: string | null) {
        this.baseUrl = baseUrl
        this.token = token
    }

    public async request(url: string, method: 'GET'): Promise<Response>
    public async request(url: string, method: 'POST', body: BodyType): Promise<Response>

    public async request(path: string, method: 'GET' | 'POST', body?: BodyType): Promise<Response> {
        const headers = new Headers()
        if (this.token) {
            headers.set('Authorization', `Bearer ${this.token}`)
        }

        let rawBody: string | Uint8Array
        if (body instanceof Uint8Array) {
            headers.set('Content-Type', 'application/octet-stream')
            rawBody = body
        } else if (body) {
            headers.set('Content-Type', 'application/json')
            rawBody = JSON.stringify(body)
        }

        // NOTE: In some environments (e.g. NextJS), responses are cached by default. Disabling
        // the cache using `cache: 'no-store'` causes fetch() to error in other environments
        // (e.g. Cloudflare Workers). To work around this, we simply add a cache-busting query
        // param.
        const cacheBust = generateRandomString()
        let url = `${this.baseUrl}/${path}?z=${cacheBust}`
        let result: Response
        try {
            result = await fetch(url, {
                method,
                body: rawBody!,
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
}

function generateRandomString(): string {
    return Math.random().toString(36).substring(2)
}
