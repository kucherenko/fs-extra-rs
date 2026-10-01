# @jscpd/fs-extra

The [fs-extra](https://github.com/jprichardson/node-fs-extra) 11.4.1 API as a native Node.js addon written in Rust with [napi-rs](https://napi.rs).

This package is experimental. An AI coding agent wrote it in one session as part of an experiment with the [jscpd](https://github.com/kucherenko/jscpd) code-migration skill. It has been checked with fs-extra's own test suite and a benchmark, both described below, and nothing else yet.

## Usage

The package is not on npm yet. Once it is, it works as a drop-in replacement:

```js
const fse = require('@jscpd/fs-extra')

await fse.copy('src', 'dest')
fse.ensureDirSync('out/logs')
```

```js
import fse from '@jscpd/fs-extra/esm'
```

Every fs-extra method is available in its promise, callback and sync forms, along with the `fs` methods fs-extra re-exports. TypeScript declarations are in `index.d.ts`.

Supported platforms are macOS and Linux with glibc, on x64 and arm64. Windows is not supported, because the Rust code calls Unix file APIs directly.

## How the port was made

The agent was [pi](https://github.com/earendil-works/pi) 0.99.1 with the GLM-5.3-flash model through ollama. It worked for about two hours and followed the code-migration skill from the jscpd repository:

1. Run `jscpd --compare` on the source and the empty target to list fs-extra's functions.
2. Collect test coverage of fs-extra and map each test to the functions it runs.
3. Port the test suite first, then the code, one function at a time, in the order of the report's `readyToPort` list.
4. Run `jscpd --compare` again after each step to check what is still missing.

The final report pairs 50 of fs-extra's 51 functions with a function in this package. The one without a pair, `asyncIteratorConcurrentProcess`, runs fs-extra's async copy item by item; here the Rust copy walks directories on its own threads, so the helper has nothing to do. The JavaScript files in `lib/` keep fs-extra's module layout, and each function in them calls the Rust core and converts its errors to Node's format.

The same agent ported fs-extra a second time without the skill. That port, kept at [kucherenko/fs-extra-rs-plain](https://github.com/kucherenko/fs-extra-rs-plain), passes the same tests, but `jscpd --compare` shows that 32 of its 51 functions are fs-extra's JavaScript copied unchanged.

[PORT-NOTES.md](PORT-NOTES.md) has the agent's own notes on the decisions it made.

## Tests

`npm test` runs fs-extra's test suite against this package: 734 tests pass and 11 are skipped. The skipped tests need a second file system (cross-device moves) or Windows.

Five test files replaced parts of fs-extra's JavaScript with stubs, through proxyquire or a mocked `fs.rename`, and a native implementation has no such parts to replace. The agent rewrote those tests to cause real errors instead, for example `EACCES` through `chmod 000`, and turned the mocked cross-device moves into real ones that run when `CROSS_DEVICE_PATH` points to a second file system.

fs-extra's original, unmodified tests were also run against this package, 63 of their 80 files (the rest test internal modules): 488 of 491 tests pass and 2 are skipped. The one failure is a real difference: this package exports `fs.realpath.native`, and fs-extra does not.

`cargo test` runs 11 Rust unit tests.

## Development

You need Node.js 18 or later and a Rust toolchain.

```sh
npm install
npm run build     # builds fs-extra.<platform>.node in the package root
npm test          # lint, fs-extra's test suite, the ESM test
cargo test
npm run bench     # compares this package with fs-extra from node_modules
```

[benchmark-results.md](benchmark-results.md) has the benchmark runs the agent recorded during the port.

## License

MIT. fs-extra is copyright JP Richardson; this package keeps its API and its test suite. See [LICENSE](LICENSE).
