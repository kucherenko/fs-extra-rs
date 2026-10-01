#!/usr/bin/env node
'use strict'
// Benchmark: @jscpd/fs-extra vs fs-extra on the fs-extra API surface.
// Usage: node bench/bench.js [--only subset,...] [--iterations n] [--rounds n]
//
// Methodology: per operation, the two packages alternate in `rounds` rounds
// (round A: fs-extra, round A: @jscpd/fs-extra, round B: ..., interleaved so
// system noise hits both packages evenly) and the reported number per package
// is the median round time. Both packages do the same fixtures.

const os = require('os')
const path = require('path')
const fs = require('fs')

const here = __dirname
const pkgRoot = path.join(here, '..')
const rustFs = require(path.join(pkgRoot, 'lib', 'index.js'))
const fsExtra = require('fs-extra')

const argv = require('minimist')(process.argv.slice(2))
const onlyArg = argv.only ? String(argv.only).split(',') : null
const ITER = Number(argv.iterations || 20)
const ROUNDS = Number(argv.rounds || 9)

const now = () => process.hrtime.bigint()

// op(libFixture) -> Promise; the fixture is rebuilt before each round
const OPS = [
  {
    label: 'copySync tree (30 files)',
    iter: 6,
    fixture (lib, root) {
      const src = path.join(root, 'tree-src')
      fs.rmSync(src, { recursive: true, force: true })
      fs.rmSync(path.join(root, 'tree-dest'), { recursive: true, force: true })
      fs.mkdirSync(src, { recursive: true })
      for (let d = 0; d < 3; d++) {
        fs.mkdirSync(path.join(src, `d${d}`), { recursive: true })
        for (let f = 0; f < 10; f++) {
          fs.writeFileSync(path.join(src, `d${d}`, `f${f}.txt`), Buffer.alloc(4096, 97))
        }
      }
      return root
    },
    async run (lib, root) {
      await lib.copySync(path.join(root, 'tree-src'), path.join(root, 'tree-dest'))
    },
    cleanup (root) {
      fs.rmSync(path.join(root, 'tree-dest'), { recursive: true, force: true })
    }
  },
  {
    label: 'copy tree (async, 30 files)',
    iter: 6,
    fixture (lib, root) {
      const src = path.join(root, 'tree-src-async')
      fs.rmSync(src, { recursive: true, force: true })
      fs.rmSync(path.join(root, 'tree-dest-async'), { recursive: true, force: true })
      fs.mkdirSync(src, { recursive: true })
      for (let d = 0; d < 3; d++) {
        fs.mkdirSync(path.join(src, `d${d}`), { recursive: true })
        for (let f = 0; f < 10; f++) {
          fs.writeFileSync(path.join(src, `d${d}`, `f${f}.txt`), Buffer.alloc(4096, 97))
        }
      }
      return root
    },
    async run (lib, root) {
      await lib.copy(path.join(root, 'tree-src-async'), path.join(root, 'tree-dest-async'))
    },
    cleanup (root) {
      fs.rmSync(path.join(root, 'tree-dest-async'), { recursive: true, force: true })
    }
  },
  {
    label: 'mkdirs deep x50',
    iter: 10,
    fixture (lib, root) { return root },
    async run (lib, root, i) {
      await Promise.all(
        Array.from({ length: 50 }, (_, j) => lib.mkdirs(path.join(root, `deep-${i}-${j}`, 'a/b/c/d/e/f')))
      )
    }
  },
  {
    label: 'mkdirsSync deep x50',
    iter: 10,
    fixture (lib, root) { return root },
    async run (lib, root, i) {
      for (let j = 0; j < 50; j++) {
        lib.mkdirsSync(path.join(root, `dps-${i}-${j}`, 'a/b/c/d/e/f'))
      }
    }
  },
  {
    label: 'copy file 64KB',
    iter: 20,
    fixture (lib, root) {
      fs.writeFileSync(path.join(root, 'file.txt'), Buffer.alloc(64 * 1024, 120))
      return root
    },
    async run (lib, root, i) {
      await lib.copy(path.join(root, 'file.txt'), path.join(root, `file-copy-${i}.txt`))
    }
  },
  {
    label: 'remove tree (async)',
    iter: 4,
    fixture (lib, root, i, k) {
      const t = path.join(root, `rm-${i}-${k}`)
      fs.rmSync(t, { recursive: true, force: true })
      fs.mkdirSync(t, { recursive: true })
      for (let d = 0; d < 3; d++) {
        fs.mkdirSync(path.join(t, `d${d}`), { recursive: true })
        for (let f = 0; f < 10; f++) fs.writeFileSync(path.join(t, `d${d}`, `f${f}.txt`), Buffer.alloc(512, 1))
      }
      return t
    },
    async run (lib, root, i, fixturePath) {
      await lib.remove(fixturePath)
    }
  },
  {
    label: 'removeSync tree',
    iter: 4,
    fixture (lib, root, i, k) {
      const t = path.join(root, `rms-${i}-${k}`)
      fs.rmSync(t, { recursive: true, force: true })
      fs.mkdirSync(t, { recursive: true })
      for (let d = 0; d < 3; d++) {
        fs.mkdirSync(path.join(t, `d${d}`), { recursive: true })
        for (let f = 0; f < 10; f++) fs.writeFileSync(path.join(t, `d${d}`, `f${f}.txt`), Buffer.alloc(512, 1))
      }
      return t
    },
    async run (lib, root, i, fixturePath) {
      lib.removeSync(fixturePath)
    }
  },
  {
    label: 'move (rename) x100',
    iter: 6,
    fixture (lib, root) { return root },
    async run (lib, root, i) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => {
          const src = path.join(root, `mv-${i}-${j}.txt`)
          fs.writeFileSync(src, 'sonic\n')
          return lib.move(src, src + '.moved')
        })
      )
    }
  },
  {
    label: 'outputFile x100',
    iter: 5,
    fixture (lib, root) { return root },
    async run (lib, root, i) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.outputFile(path.join(root, `of-${i}-${j}`), 'data', 'utf8'))
      )
    }
  },
  {
    label: 'outputJson x100',
    iter: 5,
    fixture (lib, root) { return root },
    async run (lib, root, i) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.outputJson(path.join(root, `oj-${i}-${j}.json`), { i, j, hello: 'world', nested: { deep: [1, 2, 3] } }))
      )
    }
  },
  {
    label: 'readJson x100',
    iter: 6,
    async fixture (lib, root) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.outputJson(path.join(root, `rj-${j}.json`), { j, hello: 'world', nested: { deep: [1, 2, 3] } }))
      )
      return root
    },
    async run (lib, root) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.readJson(path.join(root, `rj-${j}.json`)))
      )
    }
  },
  {
    label: 'writeJson x100',
    iter: 5,
    fixture (lib, root) { return root },
    async run (lib, root, i) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.writeJson(path.join(root, `wj-${i}-${j}.json`), { i, j, hello: 'world' }))
      )
    }
  },
  {
    label: 'ensureFile x100',
    iter: 5,
    fixture (lib, root) {
      fs.mkdirSync(path.join(root, 'ef'), { recursive: true })
      fs.writeFileSync(path.join(root, 'ef', 'exists.txt'), 'x')
      return root
    },
    async run (lib, root, i) {
      // half the files already exist (the ensure path), half fresh
      const base = path.join(root, 'ef')
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.ensureFile(j % 2 === 0 ? path.join(base, 'exists.txt') : path.join(base, `ef-${i}-${j}.txt`)))
      )
    }
  },
  {
    label: 'ensureSymlink x100',
    iter: 5,
    fixture (lib, root) {
      const dir = path.join(root, 'sym')
      fs.mkdirSync(dir, { recursive: true })
      fs.writeFileSync(path.join(dir, 'target.txt'), 't')
      return dir
    },
    async run (lib, root, i, dir) {
      await Promise.all(
        Array.from({ length: 100 }, (_, j) => lib.ensureSymlink('target.txt', path.join(dir, `lnk-${i}-${j}`), 'file'))
      )
    }
  },
  {
    label: 'emptyDir (100 items)',
    iter: 4,
    fixture (lib, root, i, k) {
      const dir = path.join(root, `empty-${i}-${k}`)
      fs.rmSync(dir, { recursive: true, force: true })
      fs.mkdirSync(dir, { recursive: true })
      for (let j = 0; j < 60; j++) fs.writeFileSync(path.join(dir, `f${j}`), 'x')
      for (let j = 0; j < 10; j++) {
        fs.mkdirSync(path.join(dir, `d${j}`, 's'), { recursive: true })
        fs.writeFileSync(path.join(dir, `d${j}`, 's', 'f'), 'x')
      }
      return dir
    },
    async run (lib, root, i, fixturePath) {
      await lib.emptyDir(fixturePath)
    }
  }
]

const tmpBase = path.join(os.tmpdir(), 'rustfs-bench2')

function median (arr) {
  const s = [...arr].sort((a, b) => a - b)
  return s[Math.floor(s.length / 2)]
}

;(async () => {
  fs.rmSync(tmpBase, { recursive: true, force: true })
  fs.mkdirSync(tmpBase, { recursive: true })
  console.log(`node ${process.version} — ${os.cpus().length} cpus, ${os.platform()} ${os.release()}; rust binding: release build`)
  console.log(`ops: ${ITER} calls/round, ${ROUNDS} interleaved rounds per package, median reported; fixtures identical\n`)

  for (const op of OPS) {
    if (onlyArg && !onlyArg.includes(op.label)) continue
    const perPkg = { 'fs-extra': [], '@jscpd/fs-extra': [] }
    for (let r = 0; r < ROUNDS; r++) {
      for (const [name, lib] of [['fs-extra', fsExtra], ['@jscpd/fs-extra', rustFs]]) {
        const pkgRoot = path.join(tmpBase, r.toString(), name.replace(/[^a-z]/g, ''))
        fs.rmSync(pkgRoot, { recursive: true, force: true })
        fs.mkdirSync(pkgRoot, { recursive: true })
        const fixturePath = await op.fixture(lib, pkgRoot)
        const start = now()
        for (let i = 0; i < op.iter; i++) {
          await op.run(lib, pkgRoot, i, fixturePath)
        }
        perPkg[name].push(Number(now() - start) / 1e6)
        if (op.cleanup) op.cleanup(pkgRoot)
      }
    }
    const a = median(perPkg['fs-extra'])
    const b = median(perPkg['@jscpd/fs-extra'])
    const ratio = a / b
    console.log(
      `${op.label.padEnd(28)} fs-extra ${a.toFixed(2).padStart(8)} ms   rust ${b.toFixed(2).padStart(8)} ms   ${ratio.toFixed(2)}x ${ratio >= 1 ? 'faster ✓' : 'slower ✗'}`
    )
  }

  fs.rmSync(tmpBase, { recursive: true, force: true })
  console.log('\ndone: fs-extra from node_modules, @jscpd/fs-extra from ./lib')
})().catch(e => { console.error('BENCH FAIL', e); process.exit(1) })
