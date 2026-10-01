'use strict'

// Port of node-fs-extra/lib/mkdirs/make-dir.js: recursive mkdir with node's
// fs.mkdir({ recursive: true }) semantics, in the native core (src/mkdirs.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function makeDir (dir, options) {
  return native.mkdirsAsync(dir, options).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function makeDirSync (dir, options) {
  try {
    return native.mkdirsSync(dir, options)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  makeDir: u(makeDir),
  makeDirSync
}
