'use strict'

// Port of node-fs-extra/lib/ensure/symlink.js — native core (src/ensure.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function createSymlink (srcpath, dstpath, type) {
  return native.createSymlinkAsync(srcpath, dstpath, type).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function createSymlinkSync (srcpath, dstpath, type) {
  try {
    return native.createSymlinkSync(srcpath, dstpath, type)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  createSymlink: u(createSymlink),
  createSymlinkSync
}
