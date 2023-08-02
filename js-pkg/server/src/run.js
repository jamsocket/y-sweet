#!/usr/bin/env node

const { spawnSync } = require('node:child_process')
const { installBinary, binaryExists } = require('./get_binary')

async function runBinary() {
  try {
    var binpath = binaryExists()
  } catch {
    binpath = await installBinary()
  }

  spawnSync(binpath, process.argv.slice(2), { stdio: 'inherit' })
}

runBinary()
