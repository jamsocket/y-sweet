import { expect, test } from 'vitest'
import * as Y from 'yjs'
import { spawn } from 'child_process'
import { CRATE_BASE, Server } from './server'
import { DocumentManager } from '@y-sweet/sdk'
import { createYjsProvider } from '@y-sweet/react'

async function convertDoc(doc: Y.Doc, docId: string, store: string) {
  const update = Y.encodeStateAsUpdate(doc)

  const child = spawn(`cargo run -- convert-from-update ${store} ${docId}`, {
    cwd: CRATE_BASE,
    shell: true,
  })

  child.stdin.write(update)
  child.stdin.end()

  await new Promise<void>((resolve, reject) => {
    child.on('close', (code) => {
      if (code === 0) {
        resolve()
      } else {
        reject(`Process exited with code ${code}`)
      }
    })
  })
}

async function connectToDoc(server: Server, docId: string): Promise<Y.Doc> {
  const docManager = new DocumentManager(server.connectionString())
  const getClientToken = async () => await docManager.getClientToken(docId)
  const doc = new Y.Doc()
  const provider = await createYjsProvider(doc, docId, getClientToken, {
    WebSocketPolyfill: require('ws'),
  })

  await new Promise<void>((resolve, reject) => {
    provider.on('synced', resolve)

    setTimeout(() => {
      reject('Timed out waiting for sync')
    }, 5_000)
  })

  return doc
}

test('can convert an empty doc from an update', async () => {
  const server = new Server({ useAuth: false })
  await server.waitForReady()
  const doc = new Y.Doc()
  const docId = Math.random().toString(36).substring(7)
  await convertDoc(doc, docId, server.dataDir)

  const newDoc = await connectToDoc(server, docId)
  expect(newDoc.toJSON()).toEqual({})

  const testMap = newDoc.getMap('test')
  expect(testMap.get('hello')).toBeUndefined()
  expect(testMap.get('foo')).toBeUndefined()
})

test('can convert a doc with content from an update', async () => {
  const server = new Server({ useAuth: false })
  await server.waitForReady()
  const doc = new Y.Doc()

  doc.getMap('test').set('hello', 'world')

  const docId = Math.random().toString(36).substring(7)
  await convertDoc(doc, docId, server.dataDir)

  const newDoc = await connectToDoc(server, docId)
  const testMap = newDoc.getMap('test')
  expect(testMap.get('hello')).toBe('world')
})
