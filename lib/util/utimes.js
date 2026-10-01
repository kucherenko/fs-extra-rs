'use strict'

// Port of node-fs-extra/lib/util/utimes.js — utimes with millisecond
// precision on a native fd, both universalified like the source. Times are
// normalized to epoch ms here with node's toUnixTimestamp rules before the
// native call.

const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

async function utimesMillis (path, atime, mtime) {
  return native.utimesMillisAsync(path, toMs(atime), toMs(mtime)).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function utimesMillisSync (path, atime, mtime) {
  try {
    return native.utimesMillisSyncNative(path, toMs(atime), toMs(mtime))
  } catch (err) {
    throw fromNativeError(err)
  }
}

// node fs toUnixTimestamp: Date -> ms; number is epoch SECONDS; a string
// parseable as a number is seconds too, otherwise a date string
function toMs (time) {
  if (typeof time === 'object' && time) {
    if (typeof time.getTime === 'function') return time.getTime()
    return NaN
  }
  if (typeof time === 'number') return time * 1000
  if (typeof time === 'string') {
    const n = Number(time)
    if (Number.isFinite(n)) return n * 1000
    return Date.parse(time)
  }
  // mirrors the node error: invalid argument, utime
  return NaN
}

module.exports = {
  utimesMillis: u(utimesMillis),
  utimesMillisSync
}
