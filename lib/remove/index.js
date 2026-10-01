'use strict'

// Port of node-fs-extra/lib/remove/index.js: `rm recursive force`, native
// core (src/remove.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function remove (path) {
  return native.removeAsync(path).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function removeSync (path) {
  try {
    return native.removeSync(path)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = {
  remove: u(remove),
  removeSync
}
