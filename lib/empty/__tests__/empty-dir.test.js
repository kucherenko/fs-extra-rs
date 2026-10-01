'use strict'

const fs = require('fs')
const os = require('os')
const fse = require('../..')
const path = require('path')
const assert = require('assert')

/* global afterEach, beforeEach, describe, it */

// NOTE: the original file injected a stubbed `readdir` with proxyquire and asserted the
// exact error object was propagated. The port performs readdir and the removal loop
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
    runIfNotRoot('should reject with the original EACCES readdir error', async () => {
      const unreadable = path.join(TEST_DIR, 'unreadable')
      fse.ensureDirSync(unreadable)
      fs.chmodSync(unreadable, 0o000)
      try {
        await assert.rejects(fse.emptyDir(unreadable), err => err.code === 'EACCES')
      } finally {
        fs.chmodSync(unreadable, 0o755)
      }
    })

    runIfNotRoot('should pass readdir errors to the callback', done => {
      const unreadable = path.join(TEST_DIR, 'unreadable-cb')
      fse.ensureDirSync(unreadable)
      fs.chmodSync(unreadable, 0o000)
      fse.emptyDir(unreadable, err => {
        fs.chmodSync(unreadable, 0o755)
        done(err && err.code === 'EACCES' ? undefined : new Error('original EACCES readdir error was not forwarded'))
      })
    })
  })

  describe('> when directory exists and contains items', () => {
    it('should delete all of the items', done => {
      // verify nothing
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
      fse.ensureFileSync(path.join(TEST_DIR, 'some-file'))
      fse.ensureFileSync(path.join(TEST_DIR, 'some-file-2'))
      fse.ensureDirSync(path.join(TEST_DIR, 'some-dir'))
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 3)

      fse.emptyDir(TEST_DIR, err => {
        assert.ifError(err)
        assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
        done()
      })
    })
  })

  describe('> when directory exists and contains no items', () => {
    it('should do nothing', done => {
      assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
      fse.emptyDir(TEST_DIR, err => {
        assert.ifError(err)
        assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
        done()
      })
    })
  })

  describe('> when directory does not exist', () => {
    it('should create it', done => {
      fse.removeSync(TEST_DIR)
      assert(!fs.existsSync(TEST_DIR))
      fse.emptyDir(TEST_DIR, err => {
        assert.ifError(err)
        assert.strictEqual(fs.readdirSync(TEST_DIR).length, 0)
        done()
      })
    })
  })
})