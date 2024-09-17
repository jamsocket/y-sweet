const fs = require('fs')
const path = require('path')
const VERSION = require('../package.json').version

function getSuffix(osType, osArch) {
  if (osType === 'win32' && osArch === 'x64') return 'win-x64.exe.gz'
  if (osType === 'linux' && osArch === 'x64') return 'linux-x64.gz'
  if (osType === 'linux' && osArch === 'arm64') return 'linux-arm64.gz'
  if (osType === 'darwin' && osArch === 'x64') return 'macos-x64.gz'
  if (osType === 'darwin' && osArch === 'arm64') return 'macos-arm64.gz'

  throw new Error(`Unsupported platform: ${osType} ${osArch}`)
}

function binaryUrl(version, osType, osArch) {
  const suffix = getSuffix(osType, osArch)

  const url = `https://github.com/drifting-in-space/y-sweet/releases/download/v${version}/y-sweet-${suffix}`
  return url
}

async function downloadFile(url, filePath) {
  const zlib = require('zlib')
  const { pipeline } = require('stream/promises')
  const res = await fetch(url)

  if (res.status === 404) {
    throw new Error(
      `Tried to download ${url} but the file was not found. It may have been removed.`,
    )
  } else if (res.status !== 200) {
    throw new Error(`Error downloading ${url}: server returned ${res.status}`)
  }

  await pipeline(res.body, zlib.createGunzip(), fs.createWriteStream(filePath, { mode: 0o770 }))

  return filePath
}

exports.getBinary = async () => {
  let binpath = path.normalize(path.join(__dirname, '..', 'bin'))
  if (fs.existsSync(binpath)) return binpath

  let url = binaryUrl(VERSION, process.platform, process.arch)
  await downloadFile(url, binpath)

  return binpath
}
