import { DocumentManager } from "examples/src/lib/yserv"
import { createYjsProvider } from "examples/src/lib/client"
import * as Y from 'yjs';

// "localhost" breaks on some versions of node because of this
// https://github.com/nodejs/undici/issues/1248#issuecomment-1214773044
const API_SERVER = process.env.Y_SERVE_API ?? 'http://127.0.0.1:8080'

const DOCUMENT_MANANGER = new DocumentManager({
    endpoint: API_SERVER,
    // token
})

test('Create new doc', async () => {
    const result = await DOCUMENT_MANANGER.createDoc()
    expect(typeof result.doc_id).toBe('string')
})

test('Attempt to access non-existing doc', async () => {
    await expect(DOCUMENT_MANANGER.getConnectionKey('foobar', {})).rejects.toThrow("foo")    
})

test('Create and connect to doc', async () => {
    const docResult = await DOCUMENT_MANANGER.createDoc()
    const key = await DOCUMENT_MANANGER.getConnectionKey(docResult, {})

    const doc = new Y.Doc()
    const provider = createYjsProvider(doc, key, {
        WebSocketPolyfill: require('ws'),
    })

    await new Promise((resolve, reject) => {
        provider.on('synced', resolve)
        provider.on('syncing', reject)
    })
})
