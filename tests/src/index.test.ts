import { WebsocketProvider } from 'y-websocket'
import { DocumentManager } from '@y-sweet/sdk'
import { WebSocket } from 'ws'
import * as Y from 'yjs'
import { Server, ServerConfiguration } from './server'

const CONFIGURATIONS: ServerConfiguration[] = [
  { useAuth: false, server: 'native' },
  { useAuth: true, server: 'native' },
  { useAuth: false, server: 'worker' },
  { useAuth: true, server: 'worker' },
]

let S3_ACCESS_KEY_ID = process.env.YSERVE_S3_ACCESS_KEY_ID
let S3_SECRET_KEY = process.env.YSERVE_S3_SECRET_KEY
let S3_REGION = process.env.YSERVE_S3_REGION
let S3_BUCKET_PREFIX = process.env.YSERVE_S3_BUCKET_PREFIX
if (S3_ACCESS_KEY_ID && S3_REGION && S3_SECRET_KEY && S3_BUCKET_PREFIX) {
  CONFIGURATIONS.push({
    useAuth: true,
    server: 'worker',
    S3: {
      bucket_prefix: S3_BUCKET_PREFIX,
      aws_access_key_id: S3_ACCESS_KEY_ID,
      aws_region: S3_REGION,
      aws_secret_key: S3_SECRET_KEY,
    },
  })
}
const FIVE_MINUTES_IN_MS = 10 * 60 * 1_000

describe.each(CONFIGURATIONS)(
  'Test $server (auth: $useAuth)',
  (configuration: ServerConfiguration) => {
    let SERVER: Server
    let DOCUMENT_MANANGER: DocumentManager

    beforeAll(async () => {
      SERVER = new Server(configuration)
      DOCUMENT_MANANGER = new DocumentManager({
        url: SERVER.serverUrl(),
        token: SERVER.serverToken,
      })

      await SERVER.waitForReady()
    }, FIVE_MINUTES_IN_MS)

    afterAll(() => {
      SERVER.cleanup()
    })

    test('Create new doc', async () => {
      const result = await DOCUMENT_MANANGER.createDoc()
      expect(typeof result.doc).toBe('string')
    })

    test('Attempt to access non-existing doc', async () => {
      await expect(DOCUMENT_MANANGER.getClientToken('foobar', {})).rejects.toThrow('404')
    })

    test('Create and connect to doc', async () => {
      const docResult = await DOCUMENT_MANANGER.createDoc()
      const key = await DOCUMENT_MANANGER.getClientToken(docResult, {})

      if (configuration.useAuth) {
        expect(key.token).toBeDefined()
      } else {
        expect(key.token).toBeUndefined()
      }

      const doc = new Y.Doc()
      const params = key.token ? { token: key.token } : undefined
      const provider = new WebsocketProvider(key.url, key.doc, doc, {
        params,
        WebSocketPolyfill: require('ws'),
      })

      await new Promise((resolve, reject) => {
        provider.on('synced', resolve)
        provider.on('syncing', reject)
      })
    })

    test('Configure server with URL', async () => {
      let url = new URL(DOCUMENT_MANANGER.baseUrl)
      url.username = encodeURIComponent(DOCUMENT_MANANGER.token ?? '')

      const docManager = new DocumentManager(url.toString())

      const docResult = await docManager.createDoc()
      expect(docResult.doc).toBeDefined()

      const key = await docManager.getClientToken(docResult, {})
      expect(key.url).toBeDefined()
    })

    if (configuration.useAuth) {
      test('Attempting to connect to a document without auth should fail', async () => {
        const docResult = await DOCUMENT_MANANGER.createDoc()
        const key = await DOCUMENT_MANANGER.getClientToken(docResult, {})

        expect(key.token).toBeDefined()
        delete key.token

        let ws = new WebSocket(`${key.url}/${key.doc}`)
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
  },
)
