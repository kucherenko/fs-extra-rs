'use strict'

// Port of node-fs-extra/lib/json/output-json-sync.js: stringify in the JS
// layer, then a single native call that ensures the parent directory and
// writes the file.

const { stringify } = require('jsonfile/utils')
const native = require('../native')
const { fromNativeError } = require('../native-errors')
const { outputFileSync } = require('../output-file')

function outputJsonSync (file, data, options) {
  const str = stringify(data, options === undefined ? {} : options)

  if (isPlainOptions(options)) {
    try {
      return native.outputJsonSync(file, str, nativeWriteOf(options))
    } catch (err) {
      throw fromNativeError(err)
    }
  }
  // unusual write options: the exact source path (outputFileSync handles the
  // graceful-fs fallback)
  outputFileSync(file, str, options)
}

function isPlainOptions (options) {
  if (options === undefined) return true
  if (typeof options !== 'object' || Array.isArray(options) || options === null) return false
  if (options.fs !== undefined) return false
  if (options.signal) return false
  const modeOk = options.mode === undefined || (typeof options.mode === 'number' && Number.isInteger(options.mode) && options.mode >= 0)
  const flagOk = options.flag === undefined || ['w', 'wx', 'w+', 'wx+'].includes(options.flag)
  const encOk = options.encoding === undefined
  return modeOk && flagOk && encOk
}

function nativeWriteOf (options) {
  if (options === undefined) return undefined
  return {
    mode: typeof options.mode === 'number' && options.mode >= 0 ? options.mode : undefined,
    flag: options.flag === undefined ? undefined : options.flag
  }
}

module.exports = outputJsonSync
