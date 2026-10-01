'use strict'

// Port of node-fs-extra/lib/ensure/symlink-type.js — native core
// (src/ensure.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function symlinkType (srcpath, type) {
  return native.symlinkTypeAsync(srcpath, type).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function symlinkTypeSync (srcpath, type) {
  try {
    return native.symlinkTypeSync(srcpath, type)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  symlinkType: u(symlinkType),
  symlinkTypeSync
}
