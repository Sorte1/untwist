export const TABLES_DIR = 'libraries';
export const INDEX_FILE = 'index.json';
export const INDEX_SCHEMA = 'untwist.library-index/1';

export interface IndexedTable {
  readonly id: string;
  readonly label: string;
  readonly file: string;
  readonly bytes: number;
  readonly sha256: string;
  readonly keys: readonly string[];
}

export interface TableIndex {
  readonly schema: string;
  readonly compiledIn: readonly string[];
  readonly tables: readonly IndexedTable[];
}

const ID = /^[a-z0-9][a-z0-9-]*$/;
const SHA = /^[0-9a-f]{64}$/;

function tableOf(value: unknown): IndexedTable | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const row = value as Record<string, unknown>;
  const { id, label, file, bytes, sha256, keys } = row;
  const valid =
    typeof id === 'string' &&
    ID.test(id) &&
    typeof label === 'string' &&
    file === `${id}.table` &&
    typeof bytes === 'number' &&
    Number.isSafeInteger(bytes) &&
    bytes > 0 &&
    typeof sha256 === 'string' &&
    SHA.test(sha256) &&
    Array.isArray(keys) &&
    keys.every((key) => typeof key === 'string');
  return valid ? { id, label, file, bytes, sha256, keys: keys as string[] } : null;
}

export function parseIndex(text: string): TableIndex | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof parsed !== 'object' || parsed === null) {
    return null;
  }
  const raw = parsed as Record<string, unknown>;
  if (raw['schema'] !== INDEX_SCHEMA || !Array.isArray(raw['tables'])) {
    return null;
  }
  const compiledIn = Array.isArray(raw['compiledIn']) ? raw['compiledIn'].filter((id): id is string => typeof id === 'string') : [];
  const tables = raw['tables'].map(tableOf).filter((table): table is IndexedTable => table !== null && !compiledIn.includes(table.id));
  return { schema: INDEX_SCHEMA, compiledIn, tables };
}
