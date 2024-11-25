#!/usr/bin/env node

import { cp, readdir } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'

import { select, question, spinner, bold, gray } from './cli.js'
import { execute } from './shell.js'

const FRAMEWORKS = new Set(['nextjs', 'remix'])

try {
  const { values, positionals } = parseArgs({
    options: { framework: { type: 'string', short: 'f' }, help: { type: 'boolean', short: 'h' } },
    allowPositionals: true,
  })

  if (values.help) help()
  else init({ framework: values.framework, name: positionals[0] })
} catch (e) {
  console.log(e)
  help()
}

function help() {
  console.log(bold('Usage: create-y-sweet-app [name] [options]'))
  console.log('\nOptions:')
  console.log('  -f, --framework <framework>\t\tUse a specific framework')
  console.log('\n    Available frameworks:', [...FRAMEWORKS].join(', '), '\n')
  console.log('  -h, --help                 \t\tShow help')
  process.exit(0)
}

/**
 * @param {object} [options]
 * @param {string} [options.framework]
 * @param {string} [options.name]
 */
async function init(options = {}) {
  let { framework, name } = options

  if (!name) {
    name = await question(
      bold('What do you want to call your app? ') + gray('(my-y-sweet-app) '),
      'my-y-sweet-app',
    )
  }

  if (!framework) framework = await select('What framework do you want to use?', [...FRAMEWORKS])
  else if (!FRAMEWORKS.has(framework)) {
    console.log(`No matching framework "${framework}".`)
    console.log('Available frameworks:', [...FRAMEWORKS].join(', '))
    process.exit(1)
  }

  const __dirname = dirname(fileURLToPath(import.meta.url))

  const src = resolve(__dirname, './frameworks', framework)
  const dest = resolve(process.cwd(), name)

  // ensure the destination directory is empty
  try {
    const contents = await readdir(dest)
    if (contents.length > 0) {
      console.error(`Directory ${name} already exists and is not empty.`)
      process.exit(1)
    }
  } catch (err) {
    // directory doesn't exist, which is fine
    if (err.code !== 'ENOENT') throw err
  }

  let install = false
  const installResponse = await question(
    bold('Do you want to install dependencies with npm? ') + gray('(Y/n) '),
    'y',
  )
  if (installResponse.toLowerCase() === 'y') install = true

  let git = false
  const gitResponse = await question(
    bold('Do you want to initialize a Git repository? ') + gray('(Y/n) '),
    'y',
  )
  if (gitResponse.toLowerCase() === 'y') git = true

  // copy the template files
  const { stop } = spinner('Copying files...')
  try {
    await cp(src, dest, { recursive: true })
    stop()
    console.log('✅ Copied files!')
  } catch (err) {
    stop()
    console.error('\nError copying template files:', err)
    process.exit(1)
  }

  // install dependencies
  if (install) {
    const { stop } = spinner('Installing dependencies...')
    for await (const { stdout, stderr } of execute('npm', ['install'], { cwd: dest })) {
      stop()
      if (stdout) console.log(stdout)
      if (stderr) console.error(stderr)
    }

    console.log('✅ Installed dependencies!')
  }

  // initialize a git repository
  if (git) {
    const { stop } = spinner('Initializing Git repository...')
    for await (const { stdout, stderr } of execute('git', ['init', '-q'], { cwd: dest })) {
      if (stdout) console.log(stdout)
      if (stderr) console.error(stderr)
    }

    stop()
    console.log('✅ Initialized git repository!')
  }

  console.log('🚀 Created y-sweet app!')
  console.log(`Run ${bold(`cd ${name}`)} to get started.`)
  process.exit(0)
}
