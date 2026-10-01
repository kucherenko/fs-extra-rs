'use strict'

// Port of node-fs-extra/lib/copy/copy.js. The algorithm lives in the native
// core (src/copy.rs); this glue handles the option shorthands and adapts the
// user filter to the calling convention threadsafe functions use.

const native = require('../native')
const { fromNativeError } = require('../native-errors')

async function copy (src, dest, opts) {
  if (typeof opts === 'function') {
    opts = { filter: opts }
  }
  opts = opts || {}

  // Warn about using preserveTimestamps on 32-bit node
  if (opts.preserveTimestamps && process.arch === 'ia32') {
    process.emitWarning(
      'Using the preserveTimestamps option in 32-bit node is not recommended;\n\n' +
      '\tsee https://github.com/jprichardson/node-fs-extra/issues/269',
      'Warning', 'fs-extra-WARN0001'
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
  // The native side expects the filter in its (err, src, dest) callback form
  // whose return value is awaited by the copy; sync and promise filters
  // (including `false`) resolve to the include answer.
  const adaptedFilter = filter
    ? (_err, src1, dest1) => {
        try {
          return Promise.resolve(filter(src1, dest1)).then(r => !!r)
        } catch (filterErr) {
          return Promise.reject(filterErr)
        }
      }
    : undefined

  return native.copyAsync(src, dest, nativeOpts, adaptedFilter).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

module.exports = copy
