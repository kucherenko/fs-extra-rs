'use strict'

// Port of node-fs-extra/lib/ensure/symlink-paths.js — native core
// (src/ensure.rs). The async/sync split is preserved: the async form's
// lstat errors carry 'ensureSymlink' in place of 'lstat'; the sync form
// throws plain Error strings for missing sources.

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function symlinkPaths (srcpath, dstpath) {
  return native.symlinkPathsAsync(srcpath, dstpath).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function symlinkPathsSync (srcpath, dstpath) {
  try {
    return native.symlinkPathsSync(srcpath, dstpath)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  symlinkPaths: u(symlinkPaths),
  symlinkPathsSync
}
