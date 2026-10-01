'use strict'

// Port of node-fs-extra/lib/json/jsonfile.js. readJson/readJsonSync/writeJson/
// writeJsonSync keep jsonfile's exact semantics (EOL handling, stripBom,
// `${file}: ${err.message}` on parse errors, throws/reviver/fs options) but
// perform the file IO natively for the common utf-8 cases, falling back to
// the jsonfile package otherwise.

const gracefulFs = require('graceful-fs')
const jsonfile = require('jsonfile')
const { stringify } = require('jsonfile/utils')
const universalify = require('universalify')
const native = require('../native')
const { fromNativeError } = require('../native-errors')

const UTF8_ENCODINGS = new Set([undefined, 'utf8', 'utf-8', 'UTF8', 'UTF-8'])

// reading natively means: default options (no fs/throws/reviver), utf8
function readIsNativeable (options) {
  if (typeof options === 'string') return UTF8_ENCODINGS.has(options)
  if (typeof options !== 'object' || Array.isArray(options) || options === null) return true
  if (options.fs !== undefined) return false
  if ('throws' in options) return false
  if (options.reviver !== undefined && options.reviver !== null) return false
  return options.encoding === undefined || UTF8_ENCODINGS.has(options.encoding)
}

// writing natively means: no custom fs, utf8 encoding, w-family flag
function writeIsNativeable (options) {
  if (typeof options === 'string') return UTF8_ENCODINGS.has(options)
  if (typeof options !== 'object' || Array.isArray(options) || options === null) return true
  if (options.fs !== undefined) return false
  if (options.signal) return false
  if (options.encoding !== undefined && !UTF8_ENCODINGS.has(options.encoding)) return false
  const flag = options.flag === undefined ? 'w' : options.flag
  return ['w', 'wx', 'w+', 'wx+'].includes(flag)
}

function nativeWriteOf (options) {
  if (typeof options === 'object' && options !== null) {
    // only well-formed values reach the native layer
    const mode = typeof options.mode === 'number' && Number.isInteger(options.mode) && options.mode >= 0
      ? options.mode
      : undefined
    const flag = options.flag === undefined ? undefined : options.flag
    return { mode, flag }
  }
  return undefined
}

async function readJson (file, options = {}) {
  if (typeof options === 'string') {
    options = { encoding: options }
  }
  if (!readIsNativeable(options)) {
    return jsonfile.readFile(file, options)
  }
  let data
  try {
    data = await native.readJsonAsync(file)
  } catch (err) {
    // read failures surface untouched, like the source
    throw fromNativeError(err)
  }
  const shouldThrow = 'throws' in options ? options.throws : true
  let obj
  try {
    obj = JSON.parse(stripBomOnString(data), null)
  } catch (parseErr) {
    if (shouldThrow) {
      parseErr.message = `${file}: ${parseErr.message}`
      throw parseErr
    } else {
      return null
    }
  }
  return obj
}

function readJsonSync (file, options = {}) {
  if (typeof options === 'string') {
    options = { encoding: options }
  }
  if (!readIsNativeable(options)) {
    return jsonfile.readFileSync(file, options)
  }
  const shouldThrow = 'throws' in options ? options.throws : true
  let content
  try {
    content = native.readJsonSync(file)
  } catch (err) {
    if (shouldThrow) {
      throw fromNativeError(err)
    }
    return null
  }
  content = stripBomOnString(content)
  try {
    return JSON.parse(content)
  } catch (err) {
    if (shouldThrow) {
      err.message = `${file}: ${err.message}`
      throw err
    } else {
      return null
    }
  }
}

async function writeJson (file, obj, options = {}) {
  const str = stringify(obj, options)
  if (!writeIsNativeable(options)) {
    const fs = (options && options.fs) || gracefulFs
    return universalify.fromCallback(fs.writeFile)(file, str, options)
  }
  return native.writeJsonAsync(file, str, nativeWriteOf(options)).then(
    v => v,
    err => { throw fromNativeError(err) }
  )
}

function writeJsonSync (file, obj, options = {}) {
  const str = stringify(obj, options)
  if (!writeIsNativeable(options)) {
    const fs = (options && options.fs) || gracefulFs
    return fs.writeFileSync(file, str, options)
  }
  try {
    return native.writeJsonSync(file, str, nativeWriteOf(options))
  } catch (err) {
    throw fromNativeError(err)
  }
}

// stripBom for text that has already been decoded to a utf-8 string
// (the native read returns strings; for buffers jsonfile's own stripBom
// runs inside the fallback path)
function stripBomOnString (content) {
  return content.replace(/^\uFEFF/, '')
}

// readJsonSync/writeJsonSync are plain sync functions (no callback form),
// exactly like the source; the async forms are universalified
module.exports = {
  readJson: universalify.fromPromise(readJson),
  readJsonSync,
  writeJson: universalify.fromPromise(writeJson),
  writeJsonSync
}
