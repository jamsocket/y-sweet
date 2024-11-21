import { exec } from 'node:child_process'
import { cp, readdir } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'

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

  const name = project || 'my-y-sweet-app'
  const __dirname = dirname(fileURLToPath(import.meta.url))

  const src = resolve(__dirname, './templates', template)
  const dest = resolve(process.cwd(), name)

  // ensure the destination directory is empty
  try {
    const contents = await readdir(dest)
    if (contents.length > 0) {
      console.error('Directory ${name} already exists and is not empty.')
      process.exit(1)
    }
  } catch (err) {
    // directory doesn't exist, which is fine
    if (err.code !== 'ENOENT') throw err
  }

  // copy the template files
  try {
    await cp(src, dest, { recursive: true })
  } catch (err) {
    console.error('\nError copying template files:', err)
    process.exit(1)
  }

  // initialize a git repository
  exec('git init', { cwd: dest })

  // install dependencies
  exec('npm install', { cwd: dest })

  console.log(`Created y-sweet app in ${name}!`)
}
