import { createYjsProvider } from "examples/src/lib/client";
import { DocumentManager } from "examples/src/lib/yserv";
import * as Y from 'yjs';
import { Server, ServerConfiguration } from "./server";

const CONFIGURATIONS: ServerConfiguration[] = [
    { useAuth: false, server: 'native' },
    { useAuth: true, server: 'native' },
    { useAuth: false, server: 'worker' },
]

const FIVE_MINUTES_IN_MS = 5 * 60 * 1_000

describe.each(CONFIGURATIONS)('Test $server (auth: $useAuth)', (configuration: ServerConfiguration) => {
    let SERVER: Server
    let DOCUMENT_MANANGER: DocumentManager

    beforeAll(async () => {
        SERVER = new Server(configuration)
        DOCUMENT_MANANGER = new DocumentManager({
            endpoint: SERVER.serverUrl(),
            token: SERVER.serverToken,
        })

        await SERVER.waitForReady()
    }, FIVE_MINUTES_IN_MS)

    afterAll(() => {
        SERVER.cleanup()
    })

    test('Create new doc', async () => {
        const result = await DOCUMENT_MANANGER.createDoc()
        expect(typeof result.doc_id).toBe('string')
    })

    test('Attempt to access non-existing doc', async () => {
        await expect(
            DOCUMENT_MANANGER.getConnectionKey('foobar', {})
        ).rejects.toThrow("404")
    })

    test('Create and connect to doc', async () => {
        const docResult = await DOCUMENT_MANANGER.createDoc()
        const key = await DOCUMENT_MANANGER.getConnectionKey(docResult, {})

        if (configuration.useAuth) {
            expect(key.token).toBeDefined()
        } else {
            expect(key.token).toBeUndefined()
        }

        const doc = new Y.Doc()
        const provider = createYjsProvider(doc, key, {
            WebSocketPolyfill: require('ws'),
        })

        await new Promise((resolve, reject) => {
            provider.on('synced', resolve)
            provider.on('syncing', reject)
        })
    })
})
