const fs = require('fs/promises')
const path = require('path')

function getSuffix(os_type, os_arch) {
  if (os_type === 'Windows_NT' && os_arch === 'x64') return 'win-x64.exe.gz'
  if (os_type === 'Linux' && os_arch === 'x64') return 'linux-x64.gz'
  //we rely on rosetta to provide emulation on aarch64 for now
  if (os_type === 'Darwin') return 'macos-x64.gz'

  throw new Error(`Unsupported platform: ${os_type} ${os_arch}`)
}

function binaryUrl(version, os_type, os_arch) {
  const suffix = getSuffix(os_type, os_arch)
  const url = `https://github.com/drifting-in-space/y-sweet/releases/download/v${version}/y-sweet-server-${suffix}`
  return url
}

async function downloadFile(url, filename_or_folder = '.') {
  const zlib = require('zlib')
  const { Readable } = require('stream')
  const { finished } = require('stream/promises')
  const res = await fetch(url)
  if (res.status === 404) {
    throw new Error('no such binary')
  }
  let tmpdir = await fs.mkdtemp('bin')
  const destination = path.resolve(tmpdir, filename_or_folder)
  const fileStream = await fs.open(destination, 'w', 0o770).then((fh) => fh.createWriteStream())
  await finished(Readable.fromWeb(res.body).pipe(zlib.createGunzip()).pipe(fileStream))
  return destination
}

async function downloadBinary(version, os_type, os_arch) {
  const url = binaryUrl(version, os_type, os_arch)
  for (let filepath of module.paths) {
    try {
      await fs.access(filepath)
    } catch {
      continue
    }
    let file = await downloadFile(url, path.join(filepath, 'y-serv'))
    return file
  }
}

exports.installBinary = async () => {
  const os = require('os')
  const version = require('../package.json').version
  const type = os.type()
  const arch = os.arch()

  return await downloadBinary(version, type, arch)
}

exports.binaryExists = async () => {
  return require.resolve('y-serv')
}
