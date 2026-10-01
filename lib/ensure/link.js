'use strict'

// Port of node-fs-extra/lib/ensure/link.js — native core (src/ensure.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function createLink (srcpath, dstpath) {
  return native.createLinkAsync(srcpath, dstpath).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function createLinkSync (srcpath, dstpath) {
  try {
    return native.createLinkSync(srcpath, dstpath)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  createLink: u(createLink),
  createLinkSync
}
