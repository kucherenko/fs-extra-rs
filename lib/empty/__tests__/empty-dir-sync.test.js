'use strict'

const fs = require('fs')
const os = require('os')
const fse = require('../..')
const path = require('path')
const assert = require('assert')

/* global afterEach, beforeEach, describe, it */

// NOTE: the original file injected a stubbed `readdirSync` with proxyquire and asserted the
// exact error object was thrown. The port performs readdirSync and the removal loop
// natively, so the same propagation guarantee is exercised through a real EACCES error
// on an unreadable directory instead (skipped when running as root, where permission
// bits are not enforced). The EIO variant has no portable real-world trigger and was
// folded into the EACCES case.

describe('+ emptyDir()', () => {
  let TEST_DIR

  beforeEach(() => {
    TEST_DIR = path.join(os.tmpdir(), 'test-fs-extra', 'empty-dir')
    if (fs.existsSync(TEST_DIR)) {
      fse.removeSync(TEST_DIR)
    }
    fse.ensureDirSync(TEST_DIR)
  })

  afterEach(done => fse.remove(TEST_DIR, done))

  const runIfNotRoot = process.getuid && process.getuid() !== 0 ? it : it.skip

  describe('propagating readdir errors', () => {
    runIfNotRoot('should throw the original EACCES readdir error', () => {
      const unreadable = path.join(TEST_DIR, 'unreadable')
      fse.ensureDirSync(unreadable)
      fs.chmodSync(unreadable, 0o000)
      try {
        assert.throws(() => fse.emptyDirSync(unreadable), err => err.code === 'EACCES')
      } finally {
        fs.chmodSync(unreadable, 0o755)
      }
    })
  })

  describe('> when directory exists and contains items', () => {
    it('should delete all of the items', () => {
      // verify nothing
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
      fse.ensureFileSync(path.join(TEST_DIR, 'some-file'))
      fse.ensureFileSync(path.join(TEST_DIR, 'some-file-2'))
      fse.ensureDirSync(path.join(TEST_DIR, 'some-dir'))
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 3)

      fse.emptyDirSync(TEST_DIR)
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
    })
  })

  describe('> when directory exists and contains no items', () => {
    it('should do nothing', () => {
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
      fse.emptyDirSync(TEST_DIR)
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
    })
  })

  describe('> when directory does not exist', () => {
    it('should create it', () => {
      fse.removeSync(TEST_DIR)
      assert(!fs.existsSync(TEST_DIR))
      fse.emptyDirSync(TEST_DIR)
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
    })
  })
})