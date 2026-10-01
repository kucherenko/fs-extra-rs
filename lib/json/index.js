'use strict'

// Port of node-fs-extra/lib/json/index.js (verbatim): wires the aliases.

const u = require('universalify').fromPromise
const jsonFile = require('./jsonfile')

// jsonFile.outputJson is the async form (callback + promise); the rest are aliases
jsonFile.outputJson = u(require('./output-json'))
jsonFile.outputJsonSync = require('./output-json-sync')
// aliases
jsonFile.outputJSON = jsonFile.outputJson
jsonFile.outputJSONSync = jsonFile.outputJsonSync
jsonFile.writeJSON = jsonFile.writeJson
jsonFile.writeJSONSync = jsonFile.writeJsonSync
jsonFile.readJSON = jsonFile.readJson
jsonFile.readJSONSync = jsonFile.readJsonSync

module.exports = jsonFile
