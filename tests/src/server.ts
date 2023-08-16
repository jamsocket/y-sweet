import { ChildProcess, execSync, spawn } from 'child_process'
import { rmSync } from 'fs'
import { tmpdir } from 'os'
import { dirname, join } from 'path'

export type ServerType = 'native' | 'worker'

type S3Config = {
  aws_access_key_id: string
  aws_secret_key: string
  aws_region: string
  bucket_name: string
  bucket_prefix: string
}

export type ServerConfiguration = {
  useAuth: boolean
  server: ServerType
  S3?: S3Config
}

export class Server {
  process: ChildProcess
  port: number
  dataDir: string
  reject?: (reason?: any) => void
  serverToken?: string
  finished: boolean = false

  static generateAuth(yServeBase: string) {
    const result = execSync('cargo run -- gen-auth --json', { cwd: yServeBase })
    return JSON.parse(result.toString())
  }

  constructor(configuration: ServerConfiguration) {
    const yServeBase = join(dirname(__filename), '..', '..', 'crates')

    execSync('cargo build --release', { stdio: 'inherit', cwd: yServeBase })

    this.port = Math.floor(Math.random() * 10000) + 10000
    this.dataDir = join(tmpdir(), `y-sweet-test-${this.port}`)

    let auth
    if (configuration.useAuth) {
      auth = Server.generateAuth(yServeBase)
    }

    if (configuration.server === 'native') {
      let command = `cargo run -- serve --port ${this.port} ${this.dataDir}`
      if (configuration.useAuth) {
        let auth = Server.generateAuth(yServeBase)
        command += ` --auth ${auth.private_key}`
        this.serverToken = auth.server_token
      }

      this.process = spawn(command, { cwd: yServeBase, stdio: 'inherit', shell: true })
    } else if (configuration.server === 'worker') {
      const workerBase = join(yServeBase, 'y-sweet-worker')
      let command = `npx wrangler dev --persist-to ${this.dataDir} --port ${this.port}`

      if (configuration.useAuth) {
        command += ` --env auth-test`
        // derived from the private key in the auth-test environment, hard-coded in wrangler.toml.
        // the value of the private key is "quThwCWto1e3ybRQKA1pz98fANzm+/j5+zXygEIEIBQ="
        this.serverToken = 'AAAgKZEAjp3ZqT6jUQCKO48OC9zYvFCWInQSj6sXbvaUeU8='
      } else {
        command += ` --env test`
      }

      if (configuration.S3) {
        command +=
          ` --var AWS_ACCESS_KEY_ID:${configuration.S3.aws_access_key_id}` +
          ` AWS_SECRET_ACCESS_KEY:${configuration.S3.aws_secret_key}` +
          ` AWS_REGION:${configuration.S3.aws_region}` +
          ` S3_BUCKET_PREFIX:${configuration.S3.bucket_prefix}`
      }

      this.process = spawn(command, { cwd: workerBase, stdio: 'inherit', shell: true })
    } else {
      throw new Error(`Unknown server type ${configuration.server}`)
    }

    this.process.on('exit', (code) => {
      if (!this.finished) {
        console.log('Server exited', code)
        if (this.reject) {
          this.reject(new Error(`Server exited with code ${code}`))
        }
      }
    })
  }

  async waitForReady(): Promise<void> {
    for (let i = 0; i < 300; i++) {
      try {
        await fetch(this.serverUrl())
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

  serverUrl() {
    return `http://127.0.0.1:${this.port}`
  }

  cleanup() {
    this.finished = true
    this.process.kill()
    rmSync(this.dataDir, { recursive: true, force: true })
  }
}
