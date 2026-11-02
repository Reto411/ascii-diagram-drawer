#!/usr/bin/env node
/**
 * S-019 AC2 — run the headless crates' test suites on wasm32 (wasm32-wasip1).
 *
 * Why this shape: the core crates are pure Rust with no DOM dependency, so their
 * libtest binaries target `wasm32-wasip1` (WASI preview 1) and run under Node's
 * built-in WASI host (`node:wasi`) — no browser, no wasm-pack, no extra tooling.
 * The web bundle itself (AC1) targets `wasm32-unknown-unknown` via Trunk; the two
 * targets share all of the crates' code, which is the point of QS4: the *same*
 * test suite passes on both the host and wasm.
 *
 * Usage:  node scripts/wasm-core-tests.mjs [crate …]     (default: all five)
 * Exit:   0 when every suite passes, 1 otherwise (libtest exit codes are checked).
 *
 * Notes:
 * - Each binary's crate directory is preopened as the guest filesystem root,
 *   mirroring native `cargo test` (cwd = manifest dir), so relative fixture
 *   paths (`tests/golden/…`) resolve identically on both targets.
 * - libtest runs single-threaded on wasm (no threads in WASI preview 1); the
 *   suites are written target-agnostically, so results match native runs and
 *   goldens compare byte-for-byte against the same committed files.
 */

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { WASI } from 'node:wasi';

const DEFAULT_CRATES = [
  'diagram-model',
  'ad-format',
  'shape-registry',
  'render-pipeline',
  'exporters',
];

const crates = process.argv.length > 2 ? process.argv.slice(2) : DEFAULT_CRATES;

/** Builds the crate's test binaries and returns their paths (like `cargo test --no-run`). */
function buildTestBinaries(crate) {
  const r = spawnSync(
    'cargo',
    ['test', '-p', crate, '--target', 'wasm32-wasip1', '--no-run'],
    { encoding: 'utf8' },
  );
  if (r.status !== 0) {
    process.stderr.write(r.stderr);
    throw new Error(`build failed for ${crate}`);
  }
  // Absolute: the runner chdirs into the crate dir before loading (fixture paths).
  const binaries = [...r.stderr.matchAll(/^\s*Executable .+ \((.+\.wasm)\)$/gm)].map(
    (m) => path.resolve(m[1]),
  );
  if (binaries.length === 0) throw new Error(`no test binaries reported for ${crate}`);
  return binaries;
}

/** Instantiates one wasm test binary under WASI and resolves with its exit code. */
async function runWasm(binary, crateDir) {
  const bytes = readFileSync(binary);
  // The crate dir is preopened as the guest root: relative fixture paths
  // (`tests/golden/…`) then behave exactly as under native `cargo test`, which
  // runs each suite with cwd = the crate's manifest directory.
  const wasi = new WASI({
    version: 'preview1',
    args: [binary, '--test-threads=1'],
    env: {},
    preopens: { '/': path.resolve(crateDir) },
  });
  const { instance } = await WebAssembly.instantiate(bytes, wasi.getImportObject());
  return await wasi.start(instance); // resolves with the process exit code
}

let failures = 0;
for (const crate of crates) {
  const crateDir = `crates/${crate}`;
  console.log(`\n════ ${crate} (wasm32-wasip1) ════`);
  for (const binary of buildTestBinaries(crate)) {
    process.stdout.write(`── ${binary}\n`);
    const code = await runWasm(binary, crateDir);
    console.log(`[runner] exit ${code}`);
    if (code !== 0) failures += 1;
  }
}

console.log(failures === 0 ? '\nAll wasm core suites passed.' : `\n${failures} suite(s) FAILED.`);
process.exit(failures === 0 ? 0 : 1);
