import { Application } from 'typedoc'
import path from 'path'
import os from 'os'
import crypto from 'crypto'

async function generateDocumentationForProject(
  basePath: string,
  entryPoints: Array<string>,
): Promise<string> {
  const tempFileName = path.join(os.tmpdir(), crypto.randomBytes(16).toString('hex') + '.json')
  const app = await Application.bootstrap({
    entryPoints,
    json: tempFileName, // output file
    basePath,
    tsconfig: path.join(basePath, 'tsconfig.json'),
    sourceLinkTemplate:
      'https://github.com/drifting-in-space/y-sweet/blob/{gitRevision}/{path}#L{line}',
  })

  const project = await app.convert()
  if (!project) {
    throw new Error('Failed to convert React project')
  }
  await app.generateJson(project, tempFileName)

  return tempFileName
}

async function generateDocumentation() {
  const jsPkgDir = path.join(__dirname, '..', '..')

  const tempFiles = await Promise.all([
    generateDocumentationForProject(path.join(jsPkgDir, 'client'), [
      path.join(jsPkgDir, 'client', 'src', 'main.ts'),
    ]),
    generateDocumentationForProject(path.join(jsPkgDir, 'react'), [
      path.join(jsPkgDir, 'react', 'src', 'main.tsx'),
    ]),
    generateDocumentationForProject(path.join(jsPkgDir, 'sdk'), [
      path.join(jsPkgDir, 'sdk', 'src', 'main.ts'),
    ]),
  ])

  const app = await Application.bootstrap({
    entryPointStrategy: 'merge',
    entryPoints: tempFiles,
    name: 'y-sweet',
    readme: path.join(jsPkgDir, '../', 'README.md'),
  })

  const project = await app.convert()
  if (!project) {
    throw new Error('Failed to convert React project')
  }

  await app.generateDocs(project, path.join(__dirname, '..', 'docs'))
}

generateDocumentation()
  .then(() => {
    console.log('Done')
  })
  .catch((error) => {
    console.error('Failed to generate documentation:', error)
    process.exit(1)
  })
