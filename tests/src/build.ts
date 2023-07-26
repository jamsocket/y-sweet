import { execSync, spawn, ChildProcess } from "child_process";
import { dirname, join } from 'path'

export class NativeServer {
    process: ChildProcess
    port: number

    constructor() {
        const yServeBase = join(dirname(__filename), '..', '..')

        execSync(
            'cargo build --release',
            { stdio: 'inherit', cwd: yServeBase }
        )

        this.port = Math.floor(Math.random() * 10000) + 10000

        this.process = spawn(
            `cargo run -- serve --port ${this.port} ./data`,
            { cwd: yServeBase, stdio: 'inherit', shell: true }
        )

        this.process.on('exit', (code) => {
            if (code !== null) {
                console.log('Server exited', code)
            }
        })
    }

    async waitForReady(): Promise<void> {
        for (let i = 0; i < 100; i++) {
            try {
                await fetch(this.serverUrl())
                return
            } catch (e) {
                await new Promise((resolve) => setTimeout(resolve, 1_000))
            }
        }
        throw new Error('Server failed to start')
    }

    serverUrl() {
        return `http://127.0.0.1:${this.port}`
    }

    cleanup() {
        this.process.kill()
    }
}
