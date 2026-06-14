const { execSync } = require('child_process')
const path = require('path')
const fs = require('fs')

const distDir = path.join(__dirname, '..', 'dist')
const unpackedDir = path.join(distDir, 'win-unpacked')

if (!fs.existsSync(unpackedDir)) {
  console.error('win-unpacked directory not found. Build may have failed.')
  process.exit(1)
}

const pkg = require(path.join(__dirname, '..', 'package.json'))
const zipName = `${pkg.productName || pkg.name}-${pkg.version}-win.zip`
const zipPath = path.join(distDir, zipName)

const pythonScript = path.join(distDir, '_zip.py')
fs.writeFileSync(pythonScript, `
import zipfile
import os

os.chdir('${distDir}')
with zipfile.ZipFile('${zipName}', 'w', zipfile.ZIP_DEFLATED) as zf:
    for root, dirs, files in os.walk('win-unpacked'):
        for f in files:
            fp = os.path.join(root, f)
            zf.write(fp, fp)
print(f'Created ${zipName}')
print(f'Size: {os.path.getsize("${zipName}") / 1024 / 1024:.1f}MB')
`)

try {
  execSync(`python3 "${pythonScript}"`, { stdio: 'inherit' })
  console.log(`\nOutput: ${zipPath}`)
} finally {
  fs.unlinkSync(pythonScript)
}
