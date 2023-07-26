import { execSync, spawn, ChildProcess } from "child_process";
import { dirname, join } from 'path'
import { tmpdir } from 'os'
import { rmSync } from 'fs'

export class LocalServer {
    process: ChildProcess
    port: number
    dataDir: string
    reject?: (reason?: any) => void
    serverToken?: string

    static generateAuth(yServeBase: string) {
        const result = execSync(
            'cargo run -- gen-auth --json',
            { cwd: yServeBase }
        )
        return JSON.parse(result.toString())
    }

    constructor(useAuth: boolean = true) {
        const yServeBase = join(dirname(__filename), '..', '..')

        execSync(
            'cargo build --release',
            { stdio: 'inherit', cwd: yServeBase }
        )

        this.port = Math.floor(Math.random() * 10000) + 10000
        this.dataDir = join(tmpdir(), `y-serve-test-${this.port}`)

        let command = `cargo run -- serve --port ${this.port} ${this.dataDir}`
        if (useAuth) {
            let auth = LocalServer.generateAuth(yServeBase)
            command += ` --auth ${auth.private_key}`
            this.serverToken = auth.server_token
        }

        this.process = spawn(
            command,
            { cwd: yServeBase, stdio: 'inherit', shell: true }
        )

        this.process.on('exit', (code) => {
            if (code !== null) {
                console.log('Server exited', code)
                if (this.reject) {
                    this.reject(new Error(`Server exited with code ${code}`))
                }
            }
        })
    }

    async waitForReady(): Promise<void> {
        for (let i = 0; i < 100; i++) {
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
        this.process.kill()
        rmSync(this.dataDir, { recursive: true, force: true })
    }
}
