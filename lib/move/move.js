'use strict'

// Port of node-fs-extra/lib/move/move.js — native core (src/move_mod.rs).

const native = require('../native')
const { fromNativeError } = require('../native-errors')

async function move (src, dest, opts) {
  return native.moveAsync(src, dest, opts).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

module.exports = move
