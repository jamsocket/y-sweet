import { ChildProcess, execSync, spawn } from 'child_process'
import { rmSync } from 'fs'
import { tmpdir } from 'os'
import { dirname, join } from 'path'

export type ServerType = 'native' | 'worker'

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
  server: ServerType
  s3?: S3Config
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
      this.serverToken = auth.server_token
    }

    if (configuration.server === 'native') {
      let command = `cargo run -- serve --port ${this.port} ${this.dataDir} --prod`
      if (configuration.useAuth) {
        command += ` --auth ${auth.private_key}`
      }

      this.process = spawn(command, { cwd: yServeBase, stdio: 'inherit', shell: true })
    } else if (configuration.server === 'worker') {
      const workerBase = join(yServeBase, 'y-sweet-worker')
      const vars: Record<string, string> = {}

      if (configuration.useAuth) {
        vars['AUTH_KEY'] = auth.private_key
      }

      if (configuration.s3) {
        vars['S3_BUCKET_NAME'] = configuration.s3.bucket_name
        vars['S3_BUCKET_PREFIX'] = configuration.s3.bucket_prefix
        vars['S3_ACCESS_KEY_ID'] = configuration.s3.aws_access_key_id
        vars['S3_SECRET_ACCESS_KEY'] = configuration.s3.aws_secret_key
        vars['S3_REGION'] = configuration.s3.aws_region
        if (configuration.s3.endpoint) vars['S3_ENDPOINT'] = configuration.s3.endpoint
        vars['BUCKET_KIND'] = 'S3'
      }

      let command = `npx wrangler dev --persist-to ${this.dataDir} --port ${this.port} --env test`

      if (Object.entries(vars).length > 0) {
        command += ' --var'
        for (const [key, value] of Object.entries(vars)) {
          command += ` ${key}:${value}`
        }
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
        await fetch(`http://127.0.0.1:${this.port}`)
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
