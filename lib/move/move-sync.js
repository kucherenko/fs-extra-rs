'use strict'

// Port of node-fs-extra/lib/move/move-sync.js — native core (src/move_sync.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')

function moveSync (src, dest, opts) {
  try {
    return native.moveSync(src, dest, opts)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = moveSync
