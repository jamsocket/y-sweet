import { describe, expect, test, beforeAll, afterAll } from 'vitest'
import { DocumentManager, YSweetError } from '@y-sweet/sdk'
import {
  createYjsProvider as createYjsProvider_,
  YSweetProviderParams,
  AuthEndpoint,
} from '@y-sweet/react'
import { WebSocket } from 'ws'
import * as Y from 'yjs'
import { Server, ServerConfiguration } from './server'
import { waitForProviderSync } from './util'

/**
 * Wraps `createYjsProvider` with a polyfill for `WebSocket` and
 * disables the broadcast channel, which gets in the way of tests
 * because it bypasses the network for local changes.
 */
function createYjsProvider(
  doc: Y.Doc,
  docId: string,
  authEndpoint: AuthEndpoint,
  extraOptions: YSweetProviderParams,
) {
  extraOptions = {
    WebSocketPolyfill: require('ws'),
    // Broadcast channel prevents us from mocking separate clients.
    disableBc: true,
    ...extraOptions,
  }
  return createYjsProvider_(doc, docId, authEndpoint, extraOptions)
}

const CONFIGURATIONS: ServerConfiguration[] = [
  { useAuth: false, server: 'native' },
  { useAuth: true, server: 'native' },
  // TODO: figure out why these fail on CI/CD even though they work locally.
  // { useAuth: false, server: 'worker' },
  // { useAuth: true, server: 'worker' },
]

let S3_ACCESS_KEY_ID = process.env.Y_SWEET_S3_ACCESS_KEY_ID
let S3_SECRET_KEY = process.env.Y_SWEET_S3_SECRET_KEY
let S3_REGION = process.env.Y_SWEET_S3_REGION
let S3_BUCKET_PREFIX = process.env.Y_SWEET_S3_BUCKET_PREFIX
let S3_BUCKET_NAME = process.env.Y_SWEET_S3_BUCKET_NAME
//run s3 tests if env vars set
if (S3_ACCESS_KEY_ID && S3_REGION && S3_SECRET_KEY && S3_BUCKET_PREFIX && S3_BUCKET_NAME) {
  CONFIGURATIONS.push({
    useAuth: true,
    server: 'native',
    s3: {
      bucket_name: S3_BUCKET_NAME,
      bucket_prefix: S3_BUCKET_PREFIX,
      aws_access_key_id: S3_ACCESS_KEY_ID,
      aws_region: S3_REGION,
      aws_secret_key: S3_SECRET_KEY,
    },
  })
}

let MINIO_PORT = process.env.Y_SWEET_MINIO_PORT
//run s3 tests using minio if available
if (MINIO_PORT && S3_BUCKET_NAME && S3_BUCKET_PREFIX) {
  CONFIGURATIONS.push({
    useAuth: true,
    server: 'worker',
    s3: {
      bucket_name: S3_BUCKET_NAME,
      bucket_prefix: S3_BUCKET_PREFIX,
      endpoint: `http://localhost:${MINIO_PORT}`,
      aws_access_key_id: 'minioadmin',
      aws_region: 'minio',
      aws_secret_key: 'minioadmin',
    },
  })
}

const TEN_MINUTES_IN_MS = 10 * 60 * 1_000

describe.each(CONFIGURATIONS)(
  'Test $server (auth: $useAuth, s3: $s3)',
  (configuration: ServerConfiguration) => {
    let SERVER: Server
    let DOCUMENT_MANANGER: DocumentManager

    beforeAll(async () => {
      SERVER = new Server(configuration)
      DOCUMENT_MANANGER = new DocumentManager(SERVER.connectionString())

      await SERVER.waitForReady()
    }, TEN_MINUTES_IN_MS)

    afterAll(() => {
      SERVER.cleanup()
    })

    test('Check store status', async () => {
      const result = await DOCUMENT_MANANGER.checkStore()
      expect(result).toEqual({ ok: true })
    })

    test('Check store over GET (deprecated)', async () => {
      // Note: this tests deprecated behavior.
      // It will be removed when the behavior is removed.
      // It's ugly to access a private member like this, but
      // it's the best way to avoid changing the SDK API for a
      // test that is temporary anyway.
      let client = (DOCUMENT_MANANGER as any).client

      let result = await client.request('check_store', 'GET')
      expect(result.ok).toBe(true)
    })

    test('Create new doc', async () => {
      const result = await DOCUMENT_MANANGER.createDoc()
      expect(typeof result.docId).toBe('string')
    })

    test('Attempt to access non-existing doc', async () => {
      await expect(DOCUMENT_MANANGER.getClientToken('foobar')).rejects.toThrow('404')

      // When running Cloudflare's workerd locally, sometimes the call following
      // the 404 will fail with a 500.
      // Not sure why, but this is a workaround.
      await DOCUMENT_MANANGER.createDoc().catch(() => {})
    })

    test('Connection token should have baseUrl', async () => {
      let token = await DOCUMENT_MANANGER.getOrCreateDocAndToken('foobar')

      expect(token.baseUrl).toBeDefined()
    })

    test('Create and connect to doc', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc()
      const getClientToken = async () => await DOCUMENT_MANANGER.getClientToken(docResult)

      const doc = new Y.Doc()
      const provider = await createYjsProvider(doc, docResult.docId, getClientToken, {})

      await new Promise((resolve, reject) => {
        provider.on('synced', resolve)
        provider.on('syncing', reject)
      })
    })

    test('Update doc and fetch as update', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc()
      const connection = await DOCUMENT_MANANGER.getDocConnection(docResult.docId)

      const doc = new Y.Doc()
      const text = doc.getText('test')
      text.insert(0, 'Hello, world!')

      await connection.updateDoc(Y.encodeStateAsUpdate(doc))

      const update = await connection.getAsUpdate()
      expect(update).toBeDefined()

      const newDoc = new Y.Doc()
      newDoc.transact(() => {
        Y.applyUpdate(newDoc, update)
      })

      const newText = newDoc.getText('test')
      expect(newText.toString()).toBe('Hello, world!')
    })

    test('Fetch doc as update', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc()
      const getClientToken = async () => await DOCUMENT_MANANGER.getClientToken(docResult)

      const doc = new Y.Doc()
      const provider = await createYjsProvider(doc, docResult.docId, getClientToken, {})

      let map = doc.getMap('test')
      map.set('foo', 'bar')
      map.set('baz', 'qux')

      await waitForProviderSync(provider)

      const update = await DOCUMENT_MANANGER.getDocAsUpdate(docResult.docId)

      let newDoc = new Y.Doc()
      newDoc.transact(() => {
        Y.applyUpdate(newDoc, update)
      })

      let newMap = newDoc.getMap('test')
      expect(newMap.get('foo')).toBe('bar')
      expect(newMap.get('baz')).toBe('qux')
    })

    test('Update doc over HTTP POST', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc()

      const doc = new Y.Doc()

      let map = doc.getMap('abc123')
      map.set('123', '456')

      let update = Y.encodeStateAsUpdate(doc)

      await DOCUMENT_MANANGER.updateDoc(docResult.docId, update)

      const getClientToken = async () => await DOCUMENT_MANANGER.getClientToken(docResult)

      const provider = await createYjsProvider(doc, docResult.docId, getClientToken, {})
      await waitForProviderSync(provider)

      let newMap = doc.getMap('abc123')
      expect(newMap.get('123')).toBe('456')
    })

    test('Create a doc by specifying a name', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc('mydoc123')

      expect(docResult.docId).toBe('mydoc123')
    })

    test('Reject invalid doc name', async () => {
      await expect(DOCUMENT_MANANGER.createDoc('mydoc123!')).rejects.toThrow('400')
    })

    test('Offline changes are synced to doc', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc()
      const getClientToken = async () => await DOCUMENT_MANANGER.getClientToken(docResult)

      const doc = new Y.Doc()

      // Connect to the doc.
      const provider = await createYjsProvider(doc, docResult.docId, getClientToken, {})

      await waitForProviderSync(provider)

      // Disconnect.
      expect(provider.ws).not.toBeNull()
      provider.ws!.close()

      await new Promise<void>((resolve, reject) => {
        setTimeout(() => reject('Expected to disconnect.'), 1_000)
        provider.on('connection-close', () => {
          resolve()
        })
      })

      expect(provider.synced).toBe(false)

      // Modify the doc while offline.
      doc.getMap('test').set('foo', 'bar')

      // Reconnect to the doc.
      provider.connect()
      await new Promise<void>((resolve, reject) => {
        provider.on('status', (event: { status: string }) => {
          if (event.status === 'connected') {
            resolve()
          } else {
            reject(`Expected connected status, got ${event.status}`)
          }
        })
      })

      await waitForProviderSync(provider)
      expect(provider.synced).toBe(true)

      // Create a second doc.
      const doc2 = new Y.Doc()

      // Connect to the doc.
      const provider2 = await createYjsProvider(doc2, docResult.docId, getClientToken, {})

      expect(doc2.getMap('test').get('foo')).toBeUndefined()

      // Wait for the doc to sync.
      await waitForProviderSync(provider2)

      // Ensure that the second doc received the changes.
      expect(doc2.getMap('test').get('foo')).toBe('bar')
    })

    if (configuration.useAuth) {
      test('Connecting with 0 validForSeconds should fail', async () => {
        const docResult = await DOCUMENT_MANANGER.createDoc()
        const conn = await DOCUMENT_MANANGER.getDocConnection(docResult, { validForSeconds: 0 })

        // wait 1 second
        await new Promise((resolve) => setTimeout(resolve, 1_000))

        try {
          await conn.getAsUpdate()
          throw new Error('Expected error')
        } catch (e) {
          if (e instanceof YSweetError) {
            expect(e.cause.code).toBe('InvalidAuthProvided')
          } else {
            throw e
          }
        }
      })

      test('Connecting with 5 validForSeconds should work briefly', async () => {
        const docResult = await DOCUMENT_MANANGER.createDoc()
        const conn = await DOCUMENT_MANANGER.getDocConnection(docResult, { validForSeconds: 5 })

        const update = await conn.getAsUpdate()
        expect(update).toBeDefined()

        // wait 8 seconds
        await new Promise((resolve) => setTimeout(resolve, 8_000))

        try {
          await conn.getAsUpdate()
          throw new Error('Expected error')
        } catch (e) {
          if (e instanceof YSweetError) {
            expect(e.cause.code).toBe('InvalidAuthProvided')
          } else {
            throw e
          }
        }
      }, 10_000)

      test('Attempting to connect to a document without auth should fail', async () => {
        const docResult = await DOCUMENT_MANANGER.createDoc()
        const key = await DOCUMENT_MANANGER.getClientToken(docResult)

        expect(key.token).toBeDefined()
        delete key.token

        let ws = new WebSocket(`${key.url}/${key.docId}`)
        let result = new Promise<void>((resolve, reject) => {
          ws.addEventListener('open', () => {
            resolve()
          })
          ws.addEventListener('error', (e) => {
            reject(e.message)
          })
        })

        await expect(result).rejects.toContain('401')
      })
    }

    test('Create doc with initial content', async () => {
      const doc = new Y.Doc()
      const map = doc.getMap('test')
      map.set('initial', 'content')

      const update = Y.encodeStateAsUpdate(doc)
      const docResult = await DOCUMENT_MANANGER.createDocWithContent(update)

      // Verify the doc was created
      expect(typeof docResult.docId).toBe('string')

      // Connect to the doc and verify the content
      const getClientToken = async () => await DOCUMENT_MANANGER.getClientToken(docResult)
      const newDoc = new Y.Doc()
      const provider = await createYjsProvider(newDoc, docResult.docId, getClientToken, {})

      await waitForProviderSync(provider)

      const newMap = newDoc.getMap('test')
      expect(newMap.get('initial')).toBe('content')
    })
  },
)
