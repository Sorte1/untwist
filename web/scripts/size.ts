import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { brotliCompressSync, gzipSync } from 'node:zlib';

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(HERE, '..');
const DIST = resolve(process.env.UNTWIST_WEB_DIST ?? join(WEB, 'dist'));
const WASM = resolve(process.env.UNTWIST_WEB_WASM ?? join(WEB, 'src/engine/pkg/untwist_wasm_bg.wasm'));

const INITIAL_JS_GZIP = 190_000;
const CSS_GZIP = 15_000;
const FONT_BYTES = 120_000;
const SW_GZIP = 4_000;
const WASM_BASELINE_BYTES = 13_550_000;
const WASM_GROWTH = 0.1;

const gzip = (bytes: Buffer): number => gzipSync(bytes, { level: 9 }).length;
const brotli = (bytes: Buffer): number => brotliCompressSync(bytes).length;

interface Chunk {
  readonly file: string;
  readonly isEntry?: boolean;
  readonly imports?: readonly string[];
  readonly css?: readonly string[];
}

type Manifest = Record<string, Chunk>;

function manifest(): Manifest {
  return JSON.parse(readFileSync(join(DIST, '.vite/manifest.json'), 'utf8'));
}

const NOT_LOADED_BY_THE_PAGE: readonly string[] = ['sw.js'];

function initialChunks(chunks: Manifest): Chunk[] {
  const seen = new Set<string>();
  const pending = Object.keys(chunks).filter((key) => chunks[key]?.isEntry === true && !NOT_LOADED_BY_THE_PAGE.includes(chunks[key]?.file ?? ''));
  while (pending.length > 0) {
    const key = pending.pop();
    if (key === undefined || seen.has(key)) {
      continue;
    }
    seen.add(key);
    pending.push(...(chunks[key]?.imports ?? []));
  }
  return [...seen].flatMap((key) => chunks[key] ?? []);
}

function filesOf(chunks: Chunk[], pick: (chunk: Chunk) => readonly string[]): string[] {
  return [...new Set(chunks.flatMap(pick))];
}

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry): string[] => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

function gzipOf(files: string[]): number {
  return files.reduce((sum, file) => sum + gzip(readFileSync(join(DIST, file))), 0);
}

function check(failures: string[], label: string, value: number, cap: number): void {
  console.log(`size ${label} ${value} (cap ${cap})`);
  if (value > cap) {
    failures.push(`${label} is ${value}, over its cap of ${cap}`);
  }
}

function wasmRow(failures: string[]): void {
  const bytes = readFileSync(WASM);
  console.log(`size wasm bytes ${bytes.length} gzip ${gzip(bytes)} brotli ${brotli(bytes)}`);
  check(failures, 'wasm bytes', bytes.length, Math.floor(WASM_BASELINE_BYTES * (1 + WASM_GROWTH)));
}

function main(): number {
  const failures: string[] = [];
  const initial = initialChunks(manifest());
  check(failures, 'initial js gzip', gzipOf(filesOf(initial, (chunk) => [chunk.file])), INITIAL_JS_GZIP);
  check(failures, 'initial css gzip', gzipOf(filesOf(initial, (chunk) => chunk.css ?? [])), CSS_GZIP);
  const fonts = walk(DIST).filter((file) => file.endsWith('.woff2'));
  check(failures, 'font bytes', fonts.reduce((sum, file) => sum + statSync(file).size, 0), FONT_BYTES);
  check(failures, 'service worker gzip', gzipOf(NOT_LOADED_BY_THE_PAGE.filter((file) => existsSync(join(DIST, file)))), SW_GZIP);
  wasmRow(failures);
  for (const failure of failures) {
    console.log(`SIZE-FAIL: ${failure}`);
  }
  return failures.length === 0 ? 0 : 1;
}

process.exitCode = main();
