import { cp, readdir } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import readline from 'node:readline'

import { spinner, bold, gray } from './cli.js'
import { execute } from './shell.js'

const TEMPLATES = new Set(['nextjs', 'remix'])

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
})

/**
 * @param {string} prompt
 * @returns {Promise<string>}
 */
function question(prompt, defaultValue = '') {
  return new Promise((resolve) => rl.question(prompt, (result) => resolve(result || defaultValue)))
}

try {
  const { values, positionals } = parseArgs({
    options: { template: { type: 'string', short: 't' } },
  })

  if (values.template) init(values.template, positionals[0])
  else help()
} catch {
  help()
}

function help() {
  console.log(bold('Usage: npm create y-sweet-app --template <template> [name]'))
  console.log('Available templates:', [...TEMPLATES].join(', '))
  process.exit(0)
}

/**
 * @param {string} template
 * @param {string} project
 */
async function init(template, project) {
  if (!TEMPLATES.has(template)) {
    console.log(`No matching template for ${template}.`)
    console.log(bold('Usage: npm create y-sweet-app --template <template> [name]'))
    console.log('Available templates:', [...TEMPLATES].join(', '))
    process.exit(1)
  }

  let name = project
  if (!name)
    name = await question(
      bold('What do you want to call your app? ') + gray('(my-y-sweet-app) '),
      'my-y-sweet-app',
    )

  const __dirname = dirname(fileURLToPath(import.meta.url))

  const src = resolve(__dirname, './templates', template)
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
