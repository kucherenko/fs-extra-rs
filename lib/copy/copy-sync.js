'use strict'

// Port of node-fs-extra/lib/copy/copy-sync.js. The algorithm lives in the
// native core (src/copy_sync.rs); this glue handles the option shorthands
// and passes the filter through unchanged (it is invoked synchronously with
// (src, dest) from the native side).

const native = require('../native')
const { patchSync, fromNativeError } = require('../native-errors')

function copySync (src, dest, opts) {
  if (typeof opts === 'function') {
    opts = { filter: opts }
  }
  opts = opts || {}

  // Warn about using preserveTimestamps on 32-bit node
  if (opts.preserveTimestamps && process.arch === 'ia32') {
    process.emitWarning(
      'Using the preserveTimestamps option in 32-bit node is not recommended;\n\n' +
      '\tsee https://github.com/jprichardson/node-fs-extra/issues/269',
      'Warning', 'fs-extra-WARN0002'
    )
  }

  const filter = typeof opts.filter === 'function' ? opts.filter : undefined
  const nativeOpts = {
    clobber: opts.clobber === undefined ? undefined : !!opts.clobber,
    overwrite: opts.overwrite === undefined ? undefined : !!opts.overwrite,
    errorOnExist: !!opts.errorOnExist,
    preserveTimestamps: !!opts.preserveTimestamps,
    dereference: !!opts.dereference
  }
  try {
    return native.copySync(src, dest, nativeOpts, filter)
  } catch (err) {
    throw fromNativeError(err)
  }
}

module.exports = patchSync(copySync)
