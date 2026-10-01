'use strict'

// The native core reports filesystem failures with node's exact
// `CODE: summary, syscall 'path'` message inside a JSON envelope; this module
// unpacks the envelope into real JS Errors carrying code/errno/syscall/path,
// exactly like node fs errors.
function fromNativeError (err) {
  if (err instanceof Error && typeof err.message === 'string' && err.message.startsWith('{"fsExtraNative":')) {
    try {
      const parsed = JSON.parse(err.message)
      const d = parsed.fsExtraNative
      if (d && typeof d.message === 'string') {
        const e = new Error(d.message)
        for (const k of ['code', 'errno', 'syscall', 'path']) {
          if (k in d) e[k] = d[k]
        }
        return e
      }
    } catch {
      // fall through to the original error
    }
  }
  // semantic errors (plain Error in fs-extra) have no code; napi-rs attaches
  // code 'GenericFailure' when throwing Error<Status>, so remove it
  if (err instanceof Error && err.code === 'GenericFailure') {
    const e = new Error(err.message)
    if (err.stack) e.stack = err.stack
    return e
  }
  return err
}

// Wraps a synchronous native binding in the error unpacker.
function patchSync (fn) {
  return function (...args) {
    try {
      return fn.apply(this, args)
    } catch (err) {
      throw fromNativeError(err)
    }
  }
}

// Wraps a promise-returning native binding in the error unpacker.
function patchPromise (fn) {
  return function (...args) {
    try {
      const r = fn.apply(this, args)
      if (r && typeof r.then === 'function') {
        return r.then(v => v, fromNativeError)
      }
      return r
    } catch (err) {
      throw fromNativeError(err)
    }
  }
}

module.exports = { fromNativeError, patchSync, patchPromise }
