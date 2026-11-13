import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, renameSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '../..');
const FILES = ['untwist_wasm.js', 'untwist_wasm.d.ts', 'untwist_wasm_bg.wasm', 'untwist_wasm_bg.wasm.d.ts'];
const TARGET = resolve(HERE, '../src/engine/pkg');

function run(command: string, args: string[], env: NodeJS.ProcessEnv = process.env): void {
  console.log(`wasm-pkg: ${command} ${args.join(' ')}`);
  execFileSync(command, args, { cwd: REPO, stdio: 'inherit', env });
}

function buildId(): string {
  try {
    return execFileSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: REPO, encoding: 'utf8' }).trim();
  } catch {
    return 'dev';
  }
}

function build(): string {
  const cargoTarget = resolve(process.env.CARGO_TARGET_DIR ?? join(REPO, 'target'));
  const out = join(cargoTarget, 'wasm-pkg');
  const env = { ...process.env, UNTWIST_BUILD_ID: process.env.UNTWIST_BUILD_ID ?? buildId() };
  run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'untwist-wasm'], env);
  const wasm = join(cargoTarget, 'wasm32-unknown-unknown/release/untwist_wasm.wasm');
  run('wasm-bindgen', ['--target', 'web', '--out-dir', out, wasm]);
  const module = join(out, 'untwist_wasm_bg.wasm');
  run('wasm-opt', ['-O1', module, '-o', `${module}.opt`]);
  renameSync(`${module}.opt`, module);
  return out;
}

const given = process.argv[2] ?? process.env.UNTWIST_WASM_PKG;
const source = given === undefined ? build() : resolve(given);
const missing = FILES.filter((file) => !existsSync(join(source, file)));
if (missing.length > 0) {
  console.error(`wasm-pkg: ${source} lacks ${missing.join(', ')}`);
  process.exit(1);
}
mkdirSync(TARGET, { recursive: true });
for (const file of FILES) {
  copyFileSync(join(source, file), join(TARGET, file));
}
console.log(`wasm-pkg: copied ${FILES.length} files from ${source}`);
