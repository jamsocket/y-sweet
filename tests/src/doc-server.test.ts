import { afterAll, beforeAll, expect, test } from 'vitest'
import { ChildProcess, spawn } from 'child_process'
import { dirname, join } from 'path'
import { ClientToken, DocConnection } from '@y-sweet/sdk'
import * as Y from 'yjs'

const CRATE_BASE = join(dirname(__filename), '..', '..', 'crates')

const TEN_MINUTES_IN_MS = 10 * 60 * 1_000

let SERVER: DocServer

class DocServer {
  private port: number
  private process: ChildProcess
  private reject?: (reason?: any) => void

  constructor() {
    this.port = Math.floor(Math.random() * 10000) + 10000

    this.process = spawn('cargo run -- serve-doc', {
      cwd: CRATE_BASE,
      shell: true,
      stdio: 'inherit',
      env: {
        SESSION_BACKEND_KEY: 'mydoc',
        PORT: this.port.toString(),
        ...process.env,
      },
    })

    this.process.on('error', (error) => {
      console.error('Error starting doc server', error)
      this.reject?.(error)
      process.exit(1)
    })

    this.process.on('close', (code) => {
      this.reject?.(new Error(`Doc server exited with code ${code}`))
    })
  }

  async waitForReady(): Promise<void> {
    const attempts = 300
    for (let i = 0; i < attempts; i++) {
      try {
        await fetch(`http://127.0.0.1:${this.port}`)
        console.log('Server started.')
        return
      } catch (e) {
        await new Promise((resolve, reject) => {
          this.reject = reject
          setTimeout(resolve, 1_000)
        })
      }
    }
    throw new Error('Server failed to start')
  }

  clientToken(): ClientToken {
    return {
      baseUrl: `http://127.0.0.1:${this.port}`,
      docId: 'mydoc',
      url: '==unused==',
    }
  }

  connection(): DocConnection {
    return new DocConnection(this.clientToken())
  }

  cleanup() {
    this.process.kill()
  }
}

beforeAll(async () => {
  SERVER = new DocServer()

  await SERVER.waitForReady()
}, TEN_MINUTES_IN_MS)

afterAll(() => {
  SERVER.cleanup()
})

test('get doc as update', async () => {
  let connection = SERVER.connection()

  await connection.getAsUpdate()
})

test('Attempting to update a document with read-only authorization should fail', async () => {
  const clientToken = SERVER.clientToken()
  const updateUrl = `${clientToken.baseUrl}/update`

  // A request with full authorization should succeed
  const doc1 = new Y.Doc()
  doc1.getMap('test').set('foo', 'bar')
  let result = await fetch(updateUrl, {
    method: 'POST',
    body: Y.encodeStateAsUpdate(doc1),
    headers: {
      'Content-Type': 'application/octet-stream',
      'X-Verified-User-Data': JSON.stringify({ authorization: 'full' }),
    },
  })
  expect(result.ok).toBe(true)

  // A request with read-only authorization should fail
  const doc2 = new Y.Doc()
  doc2.getMap('test').set('foo', 'qux')
  result = await fetch(updateUrl, {
    method: 'POST',
    body: Y.encodeStateAsUpdate(doc2),
    headers: {
      'Content-Type': 'application/octet-stream',
      'X-Verified-User-Data': JSON.stringify({ authorization: 'read-only' }),
    },
  })
  expect(result.ok).toBe(false)

  // A request with invalid user-data should fail
  result = await fetch(updateUrl, {
    method: 'POST',
    body: Y.encodeStateAsUpdate(doc2),
    headers: {
      'Content-Type': 'application/octet-stream',
      'X-Verified-User-Data': JSON.stringify({ foo: 'bar' }),
    },
  })
  expect(result.ok).toBe(false)

  // A request without the verified user data header should fail
  result = await fetch(updateUrl, {
    method: 'POST',
    body: Y.encodeStateAsUpdate(doc2),
    headers: {
      'Content-Type': 'application/octet-stream',
    },
  })
  expect(result.ok).toBe(false)

  const connection = SERVER.connection()
  const doc1Update = await connection.getAsUpdate()
  expect(doc1Update).toEqual(Y.encodeStateAsUpdate(doc1))
  expect(doc1Update).not.toEqual(Y.encodeStateAsUpdate(doc2))
})
