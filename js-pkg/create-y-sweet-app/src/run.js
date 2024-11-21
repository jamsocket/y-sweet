import { exec } from 'node:child_process'
import { cp, readdir } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import readline from 'node:readline'

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

const { values, positionals } = parseArgs({ options: { template: { type: 'string', short: 't' } } })

if (values.template) init(values.template, positionals[0])
else help()

function help() {
  console.log('Usage: npm create y-sweet-app --template <template> [name]')
}

/**
 * @param {string} template
 * @param {string} project
 */
async function init(template, project) {
  const TEMPLATES = new Set(['nextjs', 'remix'])

  if (!TEMPLATES.has(template)) {
    console.log(`No matching template for ${template}.`)
    console.log('Usage: npm create y-sweet-app --template <template> [name]')
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
    bold('Do you want to initialize a git repository? ') + gray('(Y/n) '),
    'y',
  )
  if (gitResponse.toLowerCase() === 'y') git = true

  rl.close()

  // copy the template files
  try {
    await cp(src, dest, { recursive: true })
  } catch (err) {
    console.error('\nError copying template files:', err)
    process.exit(1)
  }

  if (git) {
    // initialize a git repository
    exec('git init', { cwd: dest })
  }

  if (install) {
    // install dependencies
    exec('npm install', { cwd: dest })
  }

  console.log(`Created y-sweet app in ${name}!`)
}

/**
 * Returns a string wrapped in ANSI escape codes for gray text.
 * @param {string} text
 */
function gray(text) {
  return `\x1b[90m${text}\x1b[0m`
}

/**
 * Returns a string wrapped in ANSI escape codes for bold text.
 * @param {string} text
 */
function bold(text) {
  return `\x1b[1m${text}\x1b[0m`
}
