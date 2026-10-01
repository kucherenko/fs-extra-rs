'use strict'

// Port of node-fs-extra/lib/empty/index.js: the readdir + remove loop lives
// in the native core (src/empty.rs), missing-directory -> mkdirs included.

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function emptyDir (dir) {
  return native.emptyDirAsync(dir).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function emptyDirSync (dir) {
  try {
    return native.emptyDirSync(dir)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  emptyDirSync,
  emptydirSync: emptyDirSync,
  emptyDir: u(emptyDir),
  emptydir: u(emptyDir)
}
