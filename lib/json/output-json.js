'use strict'

// Port of node-fs-extra/lib/json/output-json.js: stringify in the JS layer
// (exact engine parity with JSON.stringify), then a single native call that
// ensures the parent directory and writes the file.

const { stringify } = require('jsonfile/utils')
const native = require('../native')
const { fromNativeError } = require('../native-errors')
const { outputFile } = require('../output-file')

async function outputJson (file, data, options = {}) {
  const str = stringify(data, options)

  if (isPlainOptions(options)) {
    return native.outputJsonAsync(file, str, nativeWriteOf(options)).then(
      v => v,
      err => { throw fromNativeError(err) }
    )
  }
  // unusual write options: the exact source path (outputFile handles the
  // graceful-fs fallback)
  return outputFile(file, str, options)
}

function isPlainOptions (options) {
  if (typeof options !== 'object' || Array.isArray(options) || options === null) return false
  if (options.fs !== undefined) return false
  if (options.signal) return false
  const modeOk = options.mode === undefined || (typeof options.mode === 'number' && Number.isInteger(options.mode) && options.mode >= 0)
  const flagOk = options.flag === undefined || ['w', 'wx', 'w+', 'wx+'].includes(options.flag)
  const encOk = options.encoding === undefined
  return modeOk && flagOk && encOk
}

function nativeWriteOf (options) {
  return {
    // mode is only passed through when it is a well-formed integer; the
    // native core applies its 0o666 default for undefined/invalid values
    mode: typeof options.mode === 'number' && options.mode >= 0 ? options.mode : undefined,
    flag: options.flag === undefined ? undefined : options.flag
  }
}

module.exports = outputJson
