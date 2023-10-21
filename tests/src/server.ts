import { ChildProcess, execSync, spawn } from 'child_process'
import { rmSync, mkdirSync } from 'fs'
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

function configToString(configuration: ServerConfiguration) {
  let result = configuration.server
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
    const yServeBase = join(dirname(__filename), '..', '..', 'crates')

    this.port = Math.floor(Math.random() * 10000) + 10000
    this.dataDir = join(tmpdir(), `y-sweet-test-${this.port}`)
    const outFilePath = join(dirname(__filename), '..', 'out')

    mkdirSync(outFilePath, { recursive: true })

    this.outFileBase = join(outFilePath, configToString(configuration))
    mkdirSync(this.outFileBase, { recursive: true })

    let auth
    if (configuration.useAuth) {
      console.log('Generating auth.')
      auth = Server.generateAuth(yServeBase)
      this.serverToken = auth.server_token
      console.log('Done generating auth.')
    }

    if (configuration.server === 'native') {
      execSync('cargo build > ' + join(this.outFileBase, 'build.txt'), {
        stdio: 'ignore',
        cwd: yServeBase,
      })

      let command = `target/debug/y-sweet serve --port ${this.port} ${this.dataDir} --prod`
      if (configuration.useAuth) {
        command += ` --auth ${auth.private_key}`
      }

      command +=
        ' > ' + join(this.outFileBase, 'server.txt') + ' 2> ' + join(this.outFileBase, 'stderr.txt')

      console.log('Spawning server.')
      this.process = spawn(command, { cwd: yServeBase, shell: true, stdio: 'ignore' })
      console.log('Done spawning server.')
    } else if (configuration.server === 'worker') {
      const workerBase = join(yServeBase, 'y-sweet-worker')
      execSync('./build.sh --dev > ' + join(this.outFileBase, 'build.txt'), {
        stdio: 'ignore',
        cwd: workerBase,
      })

      const vars: Record<string, string> = {}

      if (configuration.useAuth) {
        vars['AUTH_KEY'] = auth.private_key
      }

      if (configuration.s3) {
        vars['S3_BUCKET_NAME'] = configuration.s3.bucket_name
        vars['S3_BUCKET_PREFIX'] = configuration.s3.bucket_prefix
        vars['AWS_ACCESS_KEY_ID'] = configuration.s3.aws_access_key_id
        vars['AWS_SECRET_ACCESS_KEY'] = configuration.s3.aws_secret_key
        vars['AWS_REGION'] = configuration.s3.aws_region
        if (configuration.s3.endpoint) vars['AWS_ENDPOINT_URL_S3'] = configuration.s3.endpoint
        vars['BUCKET_KIND'] = 'S3'
      }

      let command = `npx --yes wrangler dev --persist-to ${this.dataDir} --port ${this.port} --env test`

      if (Object.entries(vars).length > 0) {
        command += ' --var'
        for (const [key, value] of Object.entries(vars)) {
          command += ` ${key}:${value}`
        }
      }

      command +=
        ' > ' + join(this.outFileBase, 'server.txt') + ' 2> ' + join(this.outFileBase, 'stderr.txt')

      // For some reason, forwarding the output to a file breaks the build itself.
      this.process = spawn(command, { cwd: workerBase, shell: true, stdio: 'ignore' })
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
