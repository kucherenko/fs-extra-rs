'use strict'

// Port of node-fs-extra/lib/util/stat.js — the check algorithms live in the
// native core (src/util_stat.rs, src/napi_internal.rs); checkPaths uses
// callback/promise forms via universalify, exactly like the source.

const native = require('../native')
const { fromNativeError, patchSync: patchSyncJ } = require('../native-errors')
const u = require('universalify').fromPromise

function checkPaths (src, dest, funcName, opts, cb) {
  return native.checkPathsAsync(src, dest, funcName).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function checkParentPaths (src, srcStat, dest, funcName, cb) {
  return native.checkParentPathsAsync(src, srcStat, dest, funcName).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function checkParentPathsSync (src, srcStat, dest, funcName) {
  try {
    return native.checkParentPathsSync(src, srcStat, dest, funcName)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  // checkPaths
  checkPaths: u(checkPaths),
  checkPathsSync: patchSyncJ(native.checkPathsSync),
  // checkParent
  checkParentPaths: u(checkParentPaths),
  checkParentPathsSync,
  // Misc
  isSrcSubdir: native.isSrcSubdirJs,
  areIdentical: native.areIdenticalJs
}
