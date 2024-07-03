const { spawnSync } = require('node:child_process')
const { installBinary, binaryExists } = require('./get_binary')

async function runBinary(argv) {
  try {
    var binpath = await binaryExists()
  } catch {
    binpath = await installBinary()
  }

  return spawnSync(binpath, argv, { stdio: 'inherit', stderr: 'inherit' })
}

module.exports = { runBinary }
