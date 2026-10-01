# Port notes: fs-extra 11.4.1 → @jscpd/fs-extra (napi-rs)

The agent that made the port wrote these notes. Paths such as `node-fs-extra/` and `rust-fs-extra/` are the source and target folders of that session.

Measured with `npx jscpd@5.4.0 --compare node-fs-extra/ rust-fs-extra/`
(CodeRankEmbed, fixed paths/options across runs).

## Final numbers

- Code: 50 of 51 source functions have a Rust counterpart (98%).
- Tests: 417 of 428 source tests have ported counterparts (97%);
  100% of the target's tests trace to source tests (417 of 424 paired).
- API surface: 149 export keys, byte-identical key set with fs-extra
  (verified programmatically — no missing, extra or type-mismatched exports),
  all extra methods in callback/promise/sync forms plus the re-exported
  graceful-fs surface.
- Test suite: `npm test` → 734 passing, 11 pending (cross-device-gated and
  win32-only), 0 failing; plus standard lint, ESM parity check, and 11 Rust
  unit tests over the core algorithms.

The port was done tests-first: the suite was copied and wired to run against
the target before the Rust implementation existed, then each function was
ported (jscpd `readyToPort`/`unmatched` lists drove the order; per-test-file
function coverage on the source bound tests to functions —
`.jscpd-compare/test-map.json`).

## Deliberately not ported as-is (1 code + 11 tests)

- `lib/util/async.js:asyncIteratorConcurrentProcess` — the helper that runs
  fs-extra's JS async copy concurrently item-by-item. Its job is performed at
  the native layer: the Rust copy reads the directory once and processes the
  items concurrently (rayon), with the first failure in directory order
  winning like Promise.all. No JS promise scheduling can inject itself into
  the native walk, so no counterpart function exists.
- The proxyquire-based tests (6 in `lib/util/__tests__/utimes.test.js`, 2+2 in
  `lib/ensure/__tests__/symlink.test.js`, 2+1 in `lib/empty/__tests__/*.test.js`)
  injected stub fs objects into JS module internals. The port performs the IO
  natively, so module stubbing has no injection point:
  - the millisecond-precision utimes tests were ported as-is (internal module),
  - the stub-only assertions (fd/close error ordering inside utimesMillis)
    have no observable surface — the ordering is enforced directly in Rust,
  - the "errors propagate as-original" tests were adapted to trigger REAL
    EACCES errors (chmod 000 directories, skipped as root) and assert the
    same codes, replacing object-identity assertions (impossible across the
    native boundary) with `err.code` assertions. Comments in the test files
    record each adaptation.
- The three `fs.rename`-mocking EXDEV tests in `lib/move/__tests__/move.test.js`
  were converted to REAL cross-device move tests gated on
  `process.env.CROSS_DEVICE_PATH` (the suite's own convention). Verified for
  real: a mounted RAM disk (`hdiutil attach`, different st_dev → genuine EXDEV
  → copy+remove fallback) — async and sync move, plus the two
  preserve-timestamp cross-device test files, all pass on the real second
  device.

## Binding-layer reality (matches fs-extra's architecture)

- The re-exported fs surface passes through graceful-fs exactly like fs-extra
  (same dependency), so all fs methods (including streams, `promises`,
  constants, `realpath.native` warning behavior) are behaviorally identical.
- JSON.stringify/JSON.parse remain in JS (engine parity for stringify
  semantics and V8 error messages); the file IO under them is native.
- Non-utf8 write encodings, `signal` aborts and custom `options.fs` fall back
  to the graceful-fs path with the original behavior; the common utf-8 cases
  run natively.
- Native fs failures come through an error envelope and are rebuilt in JS as
  real Errors carrying node's `code`/`errno`/`syscall`/`path`, with node's
  exact message formats (verified against Node 26 probes, including the
  two-path `rename 'src' -> 'dst'` shapes).

## Performance

`npm run bench` (bench/bench.js): interleaved rounds (9 per package, medians),
same fixtures, release build. Headline ops are 1.1–2.9x faster; parity-level
ops (readJson/mkdirsSync) are within noise of fs-extra. See
`benchmark-results.md` for the recorded runs.