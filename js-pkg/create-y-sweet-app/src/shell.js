import { spawn } from 'node:child_process'

export async function* execute(command, args, options) {
  const proc = spawn(command, args, options)

  let done = false
  proc.on('exit', () => {
    done = true
  })

  while (!done) {
    const output = { stdout: null, stderr: null }

    await new Promise((resolve) => {
      proc.stdout.once('data', (data) => {
        output.stdout = data.toString()
        resolve()
      })

      proc.stderr.once('data', (data) => {
        output.stderr = data.toString()
        resolve()
      })

      proc.once('exit', resolve)
      proc.once('error', (err) => {
        console.log(err)
        done = true
      })
    })

    if (output.stdout || output.stderr) {
      yield output
    }
  }
}
