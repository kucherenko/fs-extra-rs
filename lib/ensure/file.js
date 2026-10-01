'use strict'

// Port of node-fs-extra/lib/ensure/file.js — native core (src/ensure.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function createFile (file) {
  return native.createFileAsync(file).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function createFileSync (file) {
  try {
    return native.createFileSync(file)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  createFile: u(createFile),
  createFileSync
}
