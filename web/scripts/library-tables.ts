import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';
import type { Plugin } from 'vite';
import { bloomFile, bloomParams, bloomWithKeys, type Bloom, type BloomParams } from '../src/libraries/bloom.ts';
import {
  COLLECTION_FILE,
  COLLECTION_SCHEMA,
  LIBRARIES_DIR,
  SHARDS_DIR,
  parseCollection,
  safeFile,
  type CollectionIndex,
  type CollectionPackage,
} from '../src/libraries/collection-format.ts';
import { distinctKeys } from '../src/libraries/keys.ts';
import { summariseTable, type TableSummary } from '../src/libraries/table-text.ts';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '../..');
const CURATED_DIRS = ['crates/untwist-libraries-core/data', 'crates/untwist-libraries-corejs/data'];
const COMPILED_IN = new Set(['crates/untwist-libraries-core/data']);
const EPOCH_SOURCE = 'crates/untwist-rename/src/library/mod.rs';
const BLOOM_SOURCE = 'crates/untwist-rename/src/library/collection.rs';
const TABLE_SUFFIX = '.table.gz';
const KEYS_SUFFIX = '.keys.json';
const COLLECTION_ENV = 'UNTWIST_LIBRARY_COLLECTION';
const VERSION_CHARS = 12;
const PHRASE_CHARS = 16;
const STRONG_MARKS = ['core-js', 'zloirock'];
const GLOBAL_DEPTH = 2;
const KEYWORDS =
  'arguments async await break case catch class const continue debugger default delete do else enum eval export extends false finally for function if implements import in instanceof interface let new null package private protected public return static super switch this throw true try typeof undefined var void while with yield object string number boolean symbol bigint use strict';

interface Curated {
  readonly summary: TableSummary;
  readonly bytes: Buffer;
  readonly keys: readonly string[];
  readonly compiledIn: boolean;
}

export interface LibraryFiles {
  readonly index: CollectionIndex;
  readonly files: ReadonlyMap<string, Buffer>;
}

const sha256 = (bytes: Uint8Array): string => createHash('sha256').update(bytes).digest('hex');

export function shapeEpoch(source: string = readFileSync(join(REPO, EPOCH_SOURCE), 'utf8')): number {
  const found = /pub const SHAPE_EPOCH: u32 = (\d+);/.exec(source)?.[1];
  if (found === undefined) {
    throw new Error(`library-tables: ${EPOCH_SOURCE} has no SHAPE_EPOCH`);
  }
  return Number(found);
}

function untwistVersion(): string {
  const found = /\[workspace\.package\][^[]*?\nversion = "([^"]+)"/.exec(readFileSync(join(REPO, 'Cargo.toml'), 'utf8'))?.[1];
  if (found === undefined) {
    throw new Error('library-tables: the workspace Cargo.toml has no version');
  }
  return found;
}

function keysOf(path: string): string[] {
  if (!existsSync(path)) {
    return [];
  }
  const parsed = JSON.parse(readFileSync(path, 'utf8')) as { keys?: unknown };
  if (!Array.isArray(parsed.keys) || !parsed.keys.every((key) => typeof key === 'string')) {
    throw new Error(`library-tables: ${path} is not an <id>.keys.json file`);
  }
  return parsed.keys as string[];
}

function curatedTables(): Curated[] {
  return CURATED_DIRS.flatMap((dir) => {
    const data = join(REPO, dir);
    if (!existsSync(data)) {
      return [];
    }
    return readdirSync(data)
      .filter((name) => name.endsWith(TABLE_SUFFIX))
      .map((name) => join(data, name))
      .map((path) => {
        const bytes = readFileSync(path);
        const summary = summariseTable(gunzipSync(bytes).toString('utf8'));
        return { summary, bytes, keys: keysOf(join(data, `${summary.id}${KEYS_SUFFIX}`)), compiledIn: COMPILED_IN.has(dir), path };
      });
  })
    .sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0))
    .map(({ summary, bytes, keys, compiledIn }) => ({ summary, bytes, keys, compiledIn }));
}

function genericNames(): Set<string> {
  const names = new Set(KEYWORDS.split(' '));
  const seen = new Set<unknown>();
  const visit = (value: unknown, depth: number): void => {
    if ((typeof value !== 'object' && typeof value !== 'function') || value === null || seen.has(value) || depth > GLOBAL_DEPTH) {
      return;
    }
    seen.add(value);
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== 'string') {
        continue;
      }
      names.add(key);
      const descriptor = Reflect.getOwnPropertyDescriptor(value, key);
      const inner: unknown = descriptor !== undefined && 'value' in descriptor ? descriptor.value : undefined;
      visit(inner, depth + 1);
      if (typeof inner === 'function') {
        visit((inner as { prototype?: unknown }).prototype, depth + 1);
      }
    }
  };
  visit(globalThis, 0);
  return names;
}

export function strongKey(key: string): boolean {
  if (key !== key.trim() || key === '') {
    return false;
  }
  const phrase = key.length >= PHRASE_CHARS && /[^A-Za-z]/.test(key);
  return phrase || key.startsWith('__') || STRONG_MARKS.some((mark) => key.includes(mark));
}

function rangeOf(versions: readonly string[]): string {
  const low = versions[0];
  const high = versions.at(-1);
  if (low === undefined || high === undefined) {
    return '';
  }
  return low === high ? low : `${low}~${high}`;
}

function curatedPackage(table: Curated, keys: readonly string[], params: BloomParams, files: Map<string, Buffer>): { readonly pkg: CollectionPackage; readonly bloom: Bloom } {
  const sha = sha256(table.bytes);
  const file = `${SHARDS_DIR}/${sha}.utl`;
  files.set(file, table.bytes);
  const { summary } = table;
  const pkg: CollectionPackage = {
    id: summary.id,
    label: summary.label,
    versions: summary.versions,
    shards: [{ file, bytes: table.bytes.length, sha256: sha, versions: rangeOf(summary.versions), rows: summary.rows }],
    keys,
    curated: true,
  };
  return { pkg, bloom: bloomWithKeys(summary.loose, params) };
}

export function builtCollection(): LibraryFiles {
  const tables = curatedTables();
  const distinct = new Map(distinctKeys(tables.map((table) => ({ id: table.summary.id, keys: table.keys })), genericNames()).map((table) => [table.id, table.keys]));
  const params = bloomParams(readFileSync(join(REPO, BLOOM_SOURCE), 'utf8'));
  const files = new Map<string, Buffer>();
  const built = tables.filter((table) => !table.compiledIn).map((table) => curatedPackage(table, (distinct.get(table.summary.id) ?? []).filter(strongKey), params, files));
  const packages = built.map(({ pkg }) => pkg);
  const bloom = Buffer.from(bloomFile(built.map(({ pkg, bloom }) => [pkg.id, bloom] as const)));
  const bloomSha = sha256(bloom);
  const bloomPath = `${SHARDS_DIR}/${bloomSha}.bloom`;
  files.set(bloomPath, bloom);
  const version = `${untwistVersion()}+tables.${sha256(Buffer.from(JSON.stringify(packages))).slice(0, VERSION_CHARS)}`;
  const index: CollectionIndex = {
    schema: COLLECTION_SCHEMA,
    version,
    shape_epoch: shapeEpoch(),
    untwist: `>=${untwistVersion()}`,
    packages,
    filters: { file: bloomPath, sha256: bloomSha, bytes: bloom.length },
  };
  files.set(COLLECTION_FILE, Buffer.from(`${JSON.stringify(index, null, 1)}\n`));
  return { index, files };
}

function checkedFile(root: string, file: string, sha: string, bytes: number): Buffer {
  const path = join(root, file);
  if (!safeFile(file) || !existsSync(path) || !statSync(path).isFile()) {
    throw new Error(`library-tables: ${COLLECTION_ENV}: ${file} is missing or leaves the collection`);
  }
  const content = readFileSync(path);
  if (sha256(content) !== sha || (bytes > 0 && content.length !== bytes)) {
    throw new Error(`library-tables: ${COLLECTION_ENV}: ${file} does not match its SHA-256 or size`);
  }
  return content;
}

export function pinnedCollection(root: string, epoch: number = shapeEpoch()): LibraryFiles {
  const text = readFileSync(join(root, COLLECTION_FILE), 'utf8');
  const index = parseCollection(text);
  if (index === null) {
    throw new Error(`library-tables: ${COLLECTION_ENV}: ${COLLECTION_FILE} is not an ${COLLECTION_SCHEMA} index`);
  }
  if (index.shape_epoch !== epoch) {
    throw new Error(`library-tables: ${COLLECTION_ENV}: the collection is for shape epoch ${index.shape_epoch}, this untwist computes ${epoch}`);
  }
  const files = new Map<string, Buffer>([[COLLECTION_FILE, Buffer.from(text)]]);
  const refs = [...(index.filters === undefined ? [] : [index.filters]), ...index.packages.flatMap((pkg) => pkg.shards)];
  for (const ref of refs) {
    files.set(ref.file, checkedFile(root, ref.file, ref.sha256, ref.bytes));
  }
  return { index, files };
}

export function libraryFiles(): LibraryFiles {
  const pinned = process.env[COLLECTION_ENV];
  return pinned === undefined || pinned === '' ? builtCollection() : pinnedCollection(resolve(pinned));
}

export function libraryTables(): Plugin {
  let built: LibraryFiles | null = null;
  const current = (): LibraryFiles => (built ??= libraryFiles());
  return {
    name: 'untwist-library-tables',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const path = new URL(request.url ?? '/', 'http://localhost').pathname;
        const prefix = `/${LIBRARIES_DIR}/`;
        const name = path.startsWith(prefix) ? path.slice(prefix.length) : '';
        const file = current().files.get(name);
        if (file === undefined) {
          next();
          return;
        }
        const json = name === COLLECTION_FILE;
        response
          .writeHead(200, { 'Content-Type': json ? 'application/json' : 'application/octet-stream', 'Cache-Control': json ? 'no-cache' : 'public, max-age=31536000, immutable' })
          .end(file);
      });
    },
    generateBundle() {
      for (const [name, source] of current().files) {
        this.emitFile({ type: 'asset', fileName: `${LIBRARIES_DIR}/${name}`, source });
      }
    },
  };
}
