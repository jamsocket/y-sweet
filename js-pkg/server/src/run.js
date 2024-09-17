#!/usr/bin/env node

const { spawnSync } = require('node:child_process')
const { getBinary } = require('./get-binary')

async function runBinary() {
  let binpath = await getBinary()

  spawnSync(binpath, process.argv.slice(2), { stdio: 'inherit', stderr: 'inherit' })
}

runBinary()
