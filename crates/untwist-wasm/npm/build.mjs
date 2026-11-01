import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, renameSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '../../..');
const OUTPUTS = ['untwist_wasm.js', 'untwist_wasm.d.ts', 'untwist_wasm_bg.wasm', 'untwist_wasm_bg.wasm.d.ts'];
const LICENSES = ['LICENSE-MIT', 'LICENSE-APACHE'];

function run(command, args) {
  console.log(`${command} ${args.join(' ')}`);
  execFileSync(command, args, { cwd: REPO, stdio: 'inherit' });
}

function build() {
  const target = resolve(process.env.CARGO_TARGET_DIR ?? join(REPO, 'target'));
  run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'untwist-wasm']);
  run('wasm-bindgen', ['--target', 'web', '--out-dir', HERE, join(target, 'wasm32-unknown-unknown/release/untwist_wasm.wasm')]);
  const module = join(HERE, 'untwist_wasm_bg.wasm');
  run('wasm-opt', ['-O2', module, '-o', `${module}.opt`]);
  renameSync(`${module}.opt`, module);
}

function copyLicenses() {
  for (const file of LICENSES) {
    copyFileSync(join(REPO, file), join(HERE, file));
  }
}

function check() {
  const missing = [...OUTPUTS, ...LICENSES].filter((file) => !existsSync(join(HERE, file)));
  if (missing.length > 0) {
    console.error(`missing ${missing.join(', ')}; run npm run build first`);
    process.exit(1);
  }
}

if (process.argv.includes('--check')) {
  copyLicenses();
  check();
} else {
  build();
  copyLicenses();
  check();
}
