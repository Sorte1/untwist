export const LIBRARIES_DIR = 'libraries';
export const COLLECTION_FILE = 'collection.json';
export const SHARDS_DIR = 'shards';
export const COLLECTION_SCHEMA = 'untwist.library-collection/1';

export interface FileRef {
  readonly file: string;
  readonly sha256: string;
  readonly bytes: number;
}

export interface CollectionShard {
  readonly file: string;
  readonly bytes: number;
  readonly sha256: string;
  readonly versions: string;
  readonly rows: number;
}

export interface CollectionPackage {
  readonly id: string;
  readonly label: string;
  readonly versions: readonly string[];
  readonly shards: readonly CollectionShard[];
  readonly keys: readonly string[];
  readonly curated?: boolean;
}

export interface CollectionIndex {
  readonly schema: string;
  readonly version: string;
  readonly shape_epoch: number;
  readonly untwist: string;
  readonly packages: readonly CollectionPackage[];
  readonly filters?: FileRef;
}

const SHA = /^[0-9a-f]{64}$/;

export function safeFile(file: string): boolean {
  return file !== '' && !file.startsWith('/') && !file.includes('\\') && file.split('/').every((part) => part !== '' && part !== '.' && part !== '..');
}

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);
const isCount = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
const isStrings = (value: unknown): value is string[] => Array.isArray(value) && value.every((item) => typeof item === 'string');
const isRef = (file: unknown, sha256: unknown): boolean => typeof file === 'string' && safeFile(file) && typeof sha256 === 'string' && SHA.test(sha256);

function shardOf(value: unknown): CollectionShard | null {
  if (!isRecord(value)) {
    return null;
  }
  const { file, bytes, sha256, versions = '', rows = 0 } = value;
  if (!isRef(file, sha256) || !isCount(bytes) || typeof versions !== 'string' || !isCount(rows)) {
    return null;
  }
  return { file: file as string, bytes, sha256: sha256 as string, versions, rows };
}

function packageOf(value: unknown): CollectionPackage | null {
  if (!isRecord(value)) {
    return null;
  }
  const { id, label, versions = [], shards, keys = [], curated = false } = value;
  if (typeof id !== 'string' || id === '' || typeof label !== 'string' || !isStrings(versions) || !isStrings(keys) || typeof curated !== 'boolean' || !Array.isArray(shards)) {
    return null;
  }
  const parsed = shards.map(shardOf);
  if (parsed.some((shard) => shard === null)) {
    return null;
  }
  return { id, label, versions, shards: parsed as CollectionShard[], keys, ...(curated ? { curated } : {}) };
}

function filtersOf(value: unknown): FileRef | null | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (!isRecord(value)) {
    return null;
  }
  const { file, sha256, bytes = 0 } = value;
  return isRef(file, sha256) && isCount(bytes) ? { file: file as string, sha256: sha256 as string, bytes } : null;
}

export function parseCollection(text: string): CollectionIndex | null {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isRecord(raw) || raw['schema'] !== COLLECTION_SCHEMA || typeof raw['version'] !== 'string' || !isCount(raw['shape_epoch']) || !Array.isArray(raw['packages'])) {
    return null;
  }
  const untwist = raw['untwist'] ?? '';
  const packages = raw['packages'].map(packageOf);
  const filters = filtersOf(raw['filters']);
  if (typeof untwist !== 'string' || packages.some((entry) => entry === null) || filters === null) {
    return null;
  }
  return {
    schema: COLLECTION_SCHEMA,
    version: raw['version'],
    shape_epoch: raw['shape_epoch'],
    untwist,
    packages: packages as CollectionPackage[],
    ...(filters === undefined ? {} : { filters }),
  };
}

export function shardOwner(index: CollectionIndex, sha256: string): { readonly pkg: CollectionPackage; readonly shard: CollectionShard } | null {
  for (const pkg of index.packages) {
    const shard = pkg.shards.find((entry) => entry.sha256 === sha256);
    if (shard !== undefined) {
      return { pkg, shard };
    }
  }
  return null;
}
