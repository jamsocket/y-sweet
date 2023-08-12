const fs = require('fs/promises')
const path = require('path')
const VERSION = require('../package.json').version

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

async function downloadFile(url, file_path) {
  const zlib = require('zlib')
  const { Readable } = require('stream')
  const { finished } = require('stream/promises')
  const res = await fetch(url)
  if (res.status === 404) {
    throw new Error(
      `Tried to download ${url} but the file was not found. It may have been removed.`,
    )
  } else if (res.status !== 200) {
    throw new Error(`Error downloading ${url}: server returned ${res.status}`)
  }
  const fileStream = await fs.open(file_path, 'w', 0o770).then((fh) => fh.createWriteStream())
  await finished(Readable.fromWeb(res.body).pipe(zlib.createGunzip()).pipe(fileStream))
  return file_path
}

async function downloadBinary(version, os_type, os_arch) {
  const url = binaryUrl(version, os_type, os_arch)
  for (let modpath of module.paths) {
    try {
      await fs.access(modpath)
    } catch {
      continue
    }
    let dirpath = path.join(modpath, 'y-sweet', 'bin')
    await fs.mkdir(dirpath, { recursive: true })
    let filepath = path.join(dirpath, `y-sweet-${version}`)
    let file = await downloadFile(url, filepath)
    return file
  }
}

exports.installBinary = async () => {
  const os = require('os')
  const type = os.type()
  const arch = os.arch()

  return await downloadBinary(VERSION, type, arch)
}

exports.binaryExists = async () => {
  let binpath = path.join(require.resolve('y-sweet'), 'bin', `y-sweet-${VERSION}`)
  fs.access(binpath).await
  return binpath
}
