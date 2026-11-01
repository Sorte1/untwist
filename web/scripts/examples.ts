import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '../..');
const OUT = resolve(HERE, '../src/examples');
const FILES = 'files';

export const MAX_EXAMPLES = 8;
export const MAX_EXAMPLE_BYTES = 64 * 1024;

export interface Pick {
  readonly name: string;
  readonly source: string;
  readonly licence: string | null;
}

export interface Entry extends Pick {
  readonly bytes: number;
}

const REFERENCE_LICENCE = 'fixtures/LICENSE-reference';

export const PICKS: readonly Pick[] = [
  { name: 'obfuscator.io — string array', source: 'fixtures/reference/deobfuscate/obfuscator.io.js', licence: REFERENCE_LICENCE },
  { name: 'obfuscator.io — control flow', source: 'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js', licence: REFERENCE_LICENCE },
  { name: 'webpack 5 bundle', source: 'fixtures/reference/unpack/webpack-5.js', licence: REFERENCE_LICENCE },
  { name: 'webpack 4 bundle', source: 'fixtures/reference/unpack/webpack-4.js', licence: REFERENCE_LICENCE },
  { name: 'Vite / Rollup bundle', source: 'fixtures/unpack-bundlers/rollup/vite-iife.js', licence: 'fixtures/unpack-bundlers/licenses/LICENSE-vite' },
  { name: 'Browserify bundle', source: 'fixtures/reference/unpack/browserify.js', licence: REFERENCE_LICENCE },
  { name: 'React createElement calls (JSX)', source: 'fixtures/jsx/classic.js', licence: null },
  { name: 'esbuild async helpers (untranspile)', source: 'fixtures/untranspile/esbuild/async.0.28.1-es2015.js', licence: 'fixtures/unpack-bundlers/licenses/LICENSE-esbuild' },
];

export function entries(picks: readonly Pick[] = PICKS, repo: string = REPO): Entry[] {
  if (picks.length > MAX_EXAMPLES) {
    throw new Error(`${picks.length} examples, more than ${MAX_EXAMPLES}`);
  }
  return picks.map((pick) => {
    const path = join(repo, pick.source);
    if (pick.licence !== null && !existsSync(join(repo, pick.licence))) {
      throw new Error(`${pick.source}: licence ${pick.licence} is missing`);
    }
    const bytes = statSync(path).size;
    if (bytes > MAX_EXAMPLE_BYTES) {
      throw new Error(`${pick.source} is ${bytes} bytes, over ${MAX_EXAMPLE_BYTES}`);
    }
    return { ...pick, bytes };
  });
}

export function copyName(path: string): string {
  return path.endsWith('.js') ? basename(path) : `${basename(path)}.txt`;
}

export function copies(list: readonly Entry[]): ReadonlyMap<string, string> {
  const paths = list.flatMap((entry) => (entry.licence === null ? [entry.source] : [entry.source, entry.licence]));
  const named = new Map(paths.map((path) => [copyName(path), path]));
  if (named.size !== new Set(paths).size) {
    throw new Error('two example files share a name');
  }
  return named;
}

function loader(entry: Entry): string {
  const code = `import('./${FILES}/${copyName(entry.source)}?raw')`;
  const parts = entry.licence === null ? code : `${code}, import('./${FILES}/${copyName(entry.licence)}?raw')`;
  return `  '${entry.source}': () => Promise.all([${parts}]).then(([module]) => module.default),`;
}

export function renderModule(list: readonly Entry[]): string {
  return [
    "import manifest from './manifest.json';",
    '',
    'export interface Example {',
    '  readonly name: string;',
    '  readonly source: string;',
    '  readonly licence: string | null;',
    '  readonly bytes: number;',
    '}',
    '',
    'export const EXAMPLES: readonly Example[] = manifest;',
    '',
    'const LOADERS: Readonly<Record<string, () => Promise<string>>> = {',
    ...list.map(loader),
    '};',
    '',
    'export function loadExample(source: string): Promise<string> {',
    '  const load = LOADERS[source];',
    '  return load === undefined ? Promise.reject(new Error(`no example ${source}`)) : load();',
    '}',
    '',
  ].join('\n');
}

export function renderManifest(list: readonly Entry[]): string {
  return `${JSON.stringify(list, null, 2)}\n`;
}

export const MODULE_PATH = join(OUT, 'examples.ts');
export const MANIFEST_PATH = join(OUT, 'manifest.json');
export const FILES_DIR = join(OUT, FILES);

export function repoText(path: string): string {
  return readFileSync(join(REPO, path), 'utf8');
}

function main(): void {
  const list = entries();
  mkdirSync(FILES_DIR, { recursive: true });
  for (const [name, path] of copies(list)) {
    writeFileSync(join(FILES_DIR, name), repoText(path));
  }
  writeFileSync(MANIFEST_PATH, renderManifest(list));
  writeFileSync(MODULE_PATH, renderModule(list));
  console.log(`examples: ${list.length} written, ${list.reduce((sum, entry) => sum + entry.bytes, 0)} bytes`);
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}

export function current(path: string): string {
  return existsSync(path) ? readFileSync(path, 'utf8') : '';
}
