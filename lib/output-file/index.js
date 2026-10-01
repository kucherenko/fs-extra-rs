'use strict'

// Port of node-fs-extra/lib/output-file/index.js. The common cases (utf-8
// encodings, w-family flags) run in the native core (src/output_file.rs);
// unusual write options (other encodings, signals, a custom fs) fall back to
// graceful-fs with the exact behavior of the source.

const path = require('path')
const gracefulFs = require('graceful-fs')
const universalifiedFs = require('../fs') // universalified, like the source's dependency
const mkdirs = require('../mkdirs')
const native = require('../native')
const { fromNativeError } = require('../native-errors')
const u = require('universalify').fromPromise

const UTF8_ENCODINGS = new Set([undefined, 'utf8', 'utf-8', 'UTF8', 'UTF-8'])

// fs.writeFile options handled natively: no signal, default or utf8
// encoding, and a w-family flag.
function writeIsNativeable (options) {
  if (options === undefined) return true
  if (typeof options === 'string') return UTF8_ENCODINGS.has(options)
  if (typeof options !== 'object') return true // invalid options: let the fs error match
  if (options.signal) return false
  if (!UTF8_ENCODINGS.has(options.encoding)) return false
  const flag = options.flag === undefined ? 'w' : options.flag
  return ['w', 'wx', 'w+', 'wx+'].includes(flag)
}

// The native binding writes with utf-8 and takes mode/flag; mode-less calls
// pass undefined so the binding applies its defaults (0o666 / 'w').
function nativeWriteOf (options) {
  if (options === undefined || options === 'utf8' || options === 'utf-8') {
    return undefined
  }
  if (typeof options === 'object') {
    return { mode: options.mode, flag: options.flag }
  }
  return undefined
}

function toWriteData (data) {
  // Uint8Array (non-Buffer) view -> Buffer over the same memory, zero-copy
  if (typeof data === 'string') return data
  if (Buffer.isBuffer(data)) return data
  if (ArrayBuffer.isView(data)) {
    return Buffer.from(data.buffer, data.byteOffset, data.byteLength)
  }
  return data
}

async function outputFile (file, data, encoding = 'utf-8') {
  if (!writeIsNativeable(encoding)) {
    // exact source fallback: pathExists + mkdirs + the graceful write
    const dir = path.dirname(file)
    if (!(await pathExists(dir))) {
      await mkdirs.mkdirs(dir)
    }
    return gracefulFs.writeFile(file, data, encoding)
  }
  // the native call ensures the parent dir exists, then writes
  return native.outputFileAsync(file, toWriteData(data), nativeWriteOf(encoding)).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function outputFileSync (file, ...args) {
  const options = args.length > 1 ? args[1] : undefined
  if (!writeIsNativeable(options)) {
    // exact source fallback
    const dir = path.dirname(file)
    if (!gracefulFs.existsSync(dir)) {
      mkdirs.mkdirsSync(dir)
    }
    const fs = (options && options.fs) || gracefulFs
    return fs.writeFileSync(file, ...args)
  }

  // the native call ensures the parent dir exists, then writes
  try {
    return native.outputFileSync(file, toWriteData(args[0]), nativeWriteOf(options))
  } catch (err) {
    throw fromNativeError(err)
  }
}

// pathExists via the universalified fs, like the source's ../fs dependency
function pathExists (path) {
  return universalifiedFs.access(path).then(() => true).catch(() => false)
}

module.exports = {
  outputFile: u(outputFile),
  outputFileSync
}
