import { DocumentManager } from "examples/src/lib/yserv"
import { createYjsProvider } from "examples/src/lib/client"
import * as Y from 'yjs';
import { LocalServer } from "./build";

type TestConfiguration = {
    useAuth: boolean,
    server: 'native' | 'local',
}

const CONFIGURATIONS: TestConfiguration[] = [
    { useAuth: false, server: 'native' },
    { useAuth: true, server: 'native' },
]

describe.each(CONFIGURATIONS)('Test $server (auth: $useAuth)', ({useAuth, server}: TestConfiguration) => {
    let SERVER: LocalServer
    let DOCUMENT_MANANGER: DocumentManager

    beforeAll(async () => {
        SERVER = new LocalServer(useAuth)
        DOCUMENT_MANANGER = new DocumentManager({
            endpoint: SERVER.serverUrl(),
            token: SERVER.serverToken,
        })

        await SERVER.waitForReady()
    }, 30_000)

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

        if (useAuth) {
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
