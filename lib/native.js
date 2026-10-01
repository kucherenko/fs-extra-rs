'use strict'

// Loads the native addon. A local build (`npm run build`) puts
// fs-extra.<platform>.node in the package root; an install from npm gets the
// same file from the @jscpd/fs-extra-<platform> optional dependency.

const path = require('path')

function isMusl () {
  if (process.platform !== 'linux') return false
  try {
    return !process.report.getReport().header.glibcVersionRuntime
  } catch {
    return false
  }
}

function platformSuffix () {
  const { platform, arch } = process
  if (platform === 'darwin' && (arch === 'arm64' || arch === 'x64')) return `darwin-${arch}`
  if (platform === 'linux' && (arch === 'arm64' || arch === 'x64')) return `linux-${arch}-${isMusl() ? 'musl' : 'gnu'}`
  return null
}

function load () {
  const suffix = platformSuffix()
  if (!suffix) {
    throw new Error(`@jscpd/fs-extra: no native build for ${process.platform}-${process.arch}; supported: macOS and Linux (glibc) on x64 and arm64`)
  }
  const errors = []
  for (const target of [path.join(__dirname, '..', `fs-extra.${suffix}.node`), `@jscpd/fs-extra-${suffix}`]) {
    try {
      return require(target)
    } catch (err) {
      errors.push(`${target}: ${err.message}`)
    }
  }
  throw new Error(`@jscpd/fs-extra: cannot load the native addon for ${suffix}. Run \`npm run build\` in a checkout, or reinstall the package.\n${errors.join('\n')}`)
}

module.exports = load()
