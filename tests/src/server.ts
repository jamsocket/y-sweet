import { ChildProcess, execSync, spawn } from 'child_process'
import { rmSync, mkdirSync } from 'fs'
import { dirname, join } from 'path'

export const CRATE_BASE = join(dirname(__filename), '..', '..', 'crates')

type S3Config = {
  aws_access_key_id: string
  aws_secret_key: string
  aws_region: string
  endpoint?: string
  bucket_name: string
  bucket_prefix: string
}

export type ServerConfiguration = {
  useAuth: boolean
  s3?: S3Config
}

function configToString(configuration: ServerConfiguration) {
  let result = 'test'
  if (configuration.useAuth) {
    result += '-auth'
  }
  if (configuration.s3) {
    result += '-s3'
  }
  return result
}

export class Server {
  process: ChildProcess
  port: number
  dataDir: string
  reject?: (reason?: any) => void
  serverToken?: string
  finished: boolean = false
  outFileBase: string

  static generateAuth(yServeBase: string) {
    const result = execSync('cargo run -- gen-auth --json', { cwd: yServeBase })
    return JSON.parse(result.toString())
  }

  constructor(configuration: ServerConfiguration) {
    this.port = Math.floor(Math.random() * 10000) + 10000
    const outFilePath = join(dirname(__filename), '..', 'out')

    mkdirSync(outFilePath, { recursive: true })

    this.outFileBase = join(outFilePath, configToString(configuration) + '-' + this.port)
    mkdirSync(this.outFileBase, { recursive: true })
    this.dataDir = join(this.outFileBase, 'data')
    mkdirSync(this.dataDir, { recursive: true })

    let auth
    if (configuration.useAuth) {
      console.log('Generating auth.')
      auth = Server.generateAuth(CRATE_BASE)
      this.serverToken = auth.server_token
      console.log('Done generating auth.')
    }

    execSync('cargo build > ' + join(this.outFileBase, 'build.txt'), {
      stdio: 'ignore',
      cwd: CRATE_BASE,
    })

    let command = `target/debug/y-sweet serve --port ${this.port} ${this.dataDir} --prod`
    if (configuration.useAuth) {
      command += ` --auth ${auth.private_key}`
    }

    command +=
      ' > ' + join(this.outFileBase, 'server.txt') + ' 2> ' + join(this.outFileBase, 'stderr.txt')

    console.log('Spawning server.', command)
    this.process = spawn(command, {
      cwd: CRATE_BASE,
      shell: true,
      stdio: 'ignore',
      env: { RUST_BACKTRACE: '1', ...process.env },
    })
    console.log('Done spawning server.')

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

  connectionString(): string {
    return `ys://${this.serverToken}@127.0.0.1:${this.port}`
  }

  cleanup() {
    this.finished = true
    this.process.kill()
    rmSync(this.dataDir, { recursive: true, force: true })
  }
}
