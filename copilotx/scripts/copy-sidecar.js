const fs = require('fs')
const path = require('path')

const targetTriple = process.env.TARGET_TRIPLE || ''
const targetDir = targetTriple
  ? path.join('sidecar', 'target', targetTriple, 'release')
  : path.join('sidecar', 'target', 'release')

const resourcesDir = path.join(__dirname, '..', 'resources')
if (!fs.existsSync(resourcesDir)) {
  fs.mkdirSync(resourcesDir, { recursive: true })
}

const exeExt = process.env.TARGET_OS
  ? process.env.TARGET_OS === 'windows'
  : targetTriple.includes('windows')
    ? '.exe'
    : process.platform === 'win32'
      ? '.exe'
      : ''

const srcPath = path.join(__dirname, '..', targetDir, 'system-helper' + exeExt)
const sidecarDest = path.join(resourcesDir, 'system-helper' + exeExt)

if (fs.existsSync(srcPath)) {
  fs.copyFileSync(srcPath, sidecarDest)
  console.log(`Copied ${srcPath} -> ${sidecarDest}`)
} else {
  console.error(`Sidecar binary not found at ${srcPath}. Build it first.`)
  process.exit(1)
}
