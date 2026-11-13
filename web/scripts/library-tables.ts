import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';
import { distinctKeys, type KeyedTable } from '../src/libraries/scan.ts';
import { INDEX_FILE, INDEX_SCHEMA, TABLES_DIR, type TableIndex } from '../src/libraries/index-format.ts';

const HERE = dirname(fileURLToPath(import.meta.url));
const CRATES = resolve(HERE, '../../crates');
const CRATE_PREFIX = 'untwist-libraries-';
const COMPILED_IN = new Set(['untwist-libraries-core']);
const TABLE_SUFFIX = '.table.gz';
const KEYS_SUFFIX = '.keys.json';
const SERVED_SUFFIX = '.table';
const GLOBAL_DEPTH = 2;
const KEYWORDS =
  'arguments async await break case catch class const continue debugger default delete do else enum eval export extends false finally for function if implements import in instanceof interface let new null package private protected public return static super switch this throw true try typeof undefined var void while with yield object string number boolean symbol bigint use strict';

interface TableFile {
  readonly id: string;
  readonly label: string;
  readonly keys: readonly string[];
  readonly table: Buffer;
  readonly keysJson: Buffer;
  readonly compiledIn: boolean;
}

export interface LibraryFiles {
  readonly index: TableIndex;
  readonly files: ReadonlyMap<string, Buffer>;
}

function keysFile(raw: Buffer, path: string): { id: string; label: string; keys: string[] } {
  const parsed = JSON.parse(raw.toString('utf8')) as { id?: unknown; label?: unknown; keys?: unknown };
  if (typeof parsed.id !== 'string' || !Array.isArray(parsed.keys) || !parsed.keys.every((key) => typeof key === 'string')) {
    throw new Error(`library-tables: ${path} is not an <id>.keys.json file`);
  }
  return { id: parsed.id, label: typeof parsed.label === 'string' ? parsed.label : parsed.id, keys: parsed.keys as string[] };
}

function tablesOf(crate: string): TableFile[] {
  const data = join(CRATES, crate, 'data');
  if (!existsSync(data)) {
    return [];
  }
  return readdirSync(data)
    .filter((name) => name.endsWith(TABLE_SUFFIX))
    .sort()
    .map((name) => {
      const id = name.slice(0, -TABLE_SUFFIX.length);
      const keysPath = join(data, `${id}${KEYS_SUFFIX}`);
      if (!existsSync(keysPath)) {
        throw new Error(`library-tables: ${crate}/data/${name} has no ${id}${KEYS_SUFFIX}`);
      }
      const keysJson = readFileSync(keysPath);
      const parsed = keysFile(keysJson, keysPath);
      return { id, label: parsed.label, keys: parsed.keys, table: readFileSync(join(data, name)), keysJson, compiledIn: COMPILED_IN.has(crate) };
    });
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

export function libraryFiles(): LibraryFiles {
  const crates = existsSync(CRATES) ? readdirSync(CRATES).filter((name) => name.startsWith(CRATE_PREFIX)).sort() : [];
  const all = crates.flatMap(tablesOf);
  const distinct = new Map(distinctKeys(all as readonly KeyedTable[], genericNames()).map((table) => [table.id, table.keys]));
  const served = all.filter((table) => !table.compiledIn);
  const files = new Map<string, Buffer>();
  const tables = served.map((table) => {
    files.set(`${table.id}${SERVED_SUFFIX}`, table.table);
    files.set(`${table.id}${KEYS_SUFFIX}`, table.keysJson);
    return {
      id: table.id,
      label: table.label,
      file: `${table.id}${SERVED_SUFFIX}`,
      bytes: table.table.length,
      sha256: createHash('sha256').update(table.table).digest('hex'),
      keys: distinct.get(table.id) ?? [],
    };
  });
  const index: TableIndex = { schema: INDEX_SCHEMA, compiledIn: all.filter((table) => table.compiledIn).map((table) => table.id), tables };
  files.set(INDEX_FILE, Buffer.from(JSON.stringify(index)));
  return { index, files };
}

const TYPES: Readonly<Record<string, string>> = { [SERVED_SUFFIX]: 'application/octet-stream', '.json': 'application/json' };

export function libraryTables(): Plugin {
  let built: LibraryFiles | null = null;
  const current = (): LibraryFiles => (built ??= libraryFiles());
  return {
    name: 'untwist-library-tables',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const path = new URL(request.url ?? '/', 'http://localhost').pathname;
        const prefix = `/${TABLES_DIR}/`;
        const file = path.startsWith(prefix) ? current().files.get(path.slice(prefix.length)) : undefined;
        if (file === undefined) {
          next();
          return;
        }
        const type = Object.entries(TYPES).find(([suffix]) => path.endsWith(suffix))?.[1] ?? 'application/octet-stream';
        response.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-cache' }).end(file);
      });
    },
    generateBundle() {
      for (const [name, source] of current().files) {
        this.emitFile({ type: 'asset', fileName: `${TABLES_DIR}/${name}`, source });
      }
    },
  };
}
