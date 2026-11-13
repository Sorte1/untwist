export interface KeyedTable {
  readonly id: string;
  readonly keys: readonly string[];
}

const WORD = /[A-Za-z0-9_$]+/g;
const MIN_HITS = 3;
const HIT_SHARE = 0.07;

export function hitsNeeded(keys: number): number {
  return Math.min(keys, Math.max(MIN_HITS, Math.ceil(keys * HIT_SHARE)));
}

export function wordsOf(text: string): Set<string> {
  const words = new Set<string>();
  for (const match of text.matchAll(WORD)) {
    words.add(match[0]);
  }
  return words;
}

function isWord(key: string): boolean {
  const runs = key.match(WORD);
  return runs !== null && runs.length === 1 && runs[0] === key;
}

function keyIn(key: string, text: string, words: ReadonlySet<string>): boolean {
  if (isWord(key)) {
    return words.has(key);
  }
  const runs = key.match(WORD) ?? [];
  return runs.every((run) => words.has(run)) && text.includes(key);
}

export function hitCount(table: KeyedTable, text: string, words: ReadonlySet<string>, stopAt = Infinity): number {
  let hits = 0;
  for (const key of table.keys) {
    if (keyIn(key, text, words)) {
      hits += 1;
      if (hits >= stopAt) {
        break;
      }
    }
  }
  return hits;
}

export function tablesIn(text: string, tables: readonly KeyedTable[]): string[] {
  if (tables.length === 0 || text === '') {
    return [];
  }
  const words = wordsOf(text);
  return tables.filter((table) => table.keys.length > 0 && hitCount(table, text, words, hitsNeeded(table.keys.length)) >= hitsNeeded(table.keys.length)).map((table) => table.id);
}

export function distinctKeys(tables: readonly KeyedTable[], generic: ReadonlySet<string>, shared = 3): KeyedTable[] {
  const counts = new Map<string, number>();
  for (const table of tables) {
    for (const key of new Set(table.keys)) {
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  return tables.map((table) => ({ id: table.id, keys: [...new Set(table.keys)].filter((key) => !generic.has(key) && (counts.get(key) ?? 0) < shared) }));
}
