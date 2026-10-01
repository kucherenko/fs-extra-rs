'use strict'

const fs = require('fs')
const os = require('os')
const fse = require('../..')
const path = require('path')
const assert = require('assert')
const utimes = require('../utimes')

// NOTE: the original file injected fs stubs with proxyquire (`utimes = proxyquire('../utimes', { '../fs': stub })`)
// to probe the JS-module plumbing of utimesMillis: fd closing on error, close-error reporting,
// and futimes-error precedence. The port performs open/futimes/close natively, so those stub
// injections have no equivalent injection point; the observable millisecond-precision behavior
// is ported as-is below.

/* global beforeEach, describe, it */

// HFS, ext{2,3}, FAT do not
function hasMillisResSync () {
  let tmpfile = path.join('millis-test-sync' + Date.now().toString() + Math.random().toString().slice(2))
  tmpfile = path.join(os.tmpdir(), tmpfile)

  // 550 millis past UNIX epoch
  const d = new Date(1435410243862)
  fs.writeFileSync(tmpfile, 'https://github.com/jprichardson/node-fs-extra/pull/141')
  const fd = fs.openSync(tmpfile, 'r+')
  fs.futimesSync(fd, d, d)
  fs.closeSync(fd)
  return fs.statSync(tmpfile).mtime > 1435410243000
}

describe('utimes', () => {
  let TEST_DIR

  beforeEach(done => {
    TEST_DIR = path.join(os.tmpdir(), 'fs-extra', 'utimes')
    fse.emptyDir(TEST_DIR, done)
  })

  describe('utimesMillis()', () => {
    // see discussion https://github.com/jprichardson/node-fs-extra/pull/141
    it('should set the utimes w/ millisecond precision', done => {
      const tmpFile = path.join(TEST_DIR, 'someFile')
      fs.writeFileSync(tmpFile, 'hello')

      let stats = fs.lstatSync(tmpFile)

      // Apr 21st, 2012
      const awhileAgo = new Date(1334990868773)
      const awhileAgoNoMillis = new Date(1334990868000)

      assert.notDeepStrictEqual(stats.mtime, awhileAgo)
      assert.notDeepStrictEqual(stats.atime, awhileAgo)

      utimes.utimesMillis(tmpFile, awhileAgo, awhileAgo, err => {
        assert.ifError(err)
        stats = fs.statSync(tmpFile)
        if (hasMillisResSync()) {
          assert.deepStrictEqual(stats.mtime, awhileAgo)
          assert.deepStrictEqual(stats.atime, awhileAgo)
        } else {
          assert.deepStrictEqual(stats.mtime, awhileAgoNoMillis)
          assert.deepStrictEqual(stats.atime, awhileAgoNoMillis)
        }
        done()
      })
    })
  })

  describe('utimesMillisSync()', () => {
    // see discussion https://github.com/jprichardson/node-fs-extra/pull/141
    it('should set the utimes w/ millisecond precision', () => {
      const tmpFile = path.join(TEST_DIR, 'someFileSync')
      fs.writeFileSync(tmpFile, 'hello')

      let stats = fs.lstatSync(tmpFile)

      // Apr 21st, 2012
      const awhileAgo = new Date(1334990868773)
      const awhileAgoNoMillis = new Date(1334990868000)

      assert.notDeepStrictEqual(stats.mtime, awhileAgo)
      assert.notDeepStrictEqual(stats.atime, awhileAgo)

      utimes.utimesMillisSync(tmpFile, awhileAgo, awhileAgo)
      stats = fs.statSync(tmpFile)
      if (hasMillisResSync()) {
        assert.deepStrictEqual(stats.mtime, awhileAgo)
        assert.deepStrictEqual(stats.atime, awhileAgo)
      } else {
        assert.deepStrictEqual(stats.mtime, awhileAgoNoMillis)
        assert.deepStrictEqual(stats.atime, awhileAgoNoMillis)
      }
    })
  })
})