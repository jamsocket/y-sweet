import { afterAll, beforeAll, test } from 'vitest'
import { ChildProcess, spawn } from 'child_process'
import { dirname, join } from 'path'
import { ClientToken, DocConnection } from '@y-sweet/sdk'

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

  connection(): DocConnection {
    let token: ClientToken = {
      baseUrl: `http://127.0.0.1:${this.port}`,
      docId: 'mydoc',
      url: '==unused==',
    }
    return new DocConnection(token)
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

// // Slows tests by a lot, so commented by default.
// test('get doc as update after delay', async () => {
//   let connection = SERVER.connection()

//   await new Promise((resolve) => setTimeout(resolve, 24_000))

//   await connection.getAsUpdate()
// }, 30_000)
