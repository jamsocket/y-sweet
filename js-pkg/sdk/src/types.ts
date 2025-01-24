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

  /** The base URL for document-level endpoints. */
  baseUrl: string

  /** A unique identifier for the document that the token connects to. */
  docId: string

  /** A string that grants the bearer access to the document. By default, the development server does not require a token. */
  token?: string

  /** The authorization level of the client. */
  authorization?: Authorization
}

export type CheckStoreResult = { ok: true } | { ok: false; error: string }

export type Authorization = 'full' | 'read-only'

export type AuthDocRequest = {
  /** The authorization level to use for the document. Defaults to 'full'. */
  authorization?: Authorization

  /** A user ID to associate with the token. Not currently used. */
  userId?: string

  /** The number of seconds the token should be valid for. */
  validForSeconds?: number
}
