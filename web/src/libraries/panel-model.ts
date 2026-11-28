import type { LibraryTablesUsed } from '../engine/types';
import type { CollectionIndex } from './collection-format';
import type { ContentSource } from './content';

export type TableState = 'built-in' | 'loaded' | 'failed' | 'skipped';

export interface TableRow {
  readonly id: string;
  readonly label: string;
  readonly bytes: number | null;
  readonly state: TableState;
  readonly ms: number | null;
  readonly source: ContentSource | null;
}

function fetchedState(id: string, used: LibraryTablesUsed | undefined): TableState {
  if (used?.fetched.includes(id) === true) {
    return 'loaded';
  }
  return used?.failed.includes(id) === true ? 'failed' : 'skipped';
}

export function builtInIds(tablesLoaded: string | undefined, index: CollectionIndex | null, used: LibraryTablesUsed | undefined): string[] {
  const served = new Set([...(index?.packages.map((pkg) => pkg.id) ?? []), ...(used?.fetched ?? [])]);
  return [...new Set((tablesLoaded ?? '').split(','))].filter((id) => id !== '' && !served.has(id));
}

export function tableRows(index: CollectionIndex | null, used: LibraryTablesUsed | undefined, builtIn: readonly string[] = []): TableRow[] {
  if (index === null) {
    return [];
  }
  const compiled = builtIn.map((id) => ({ id, label: id, bytes: null, state: 'built-in', ms: null, source: null }) satisfies TableRow);
  const fetched = index.packages.map((pkg) => {
    const timing = used?.timings?.find((row) => row.id === pkg.id);
    const state = fetchedState(pkg.id, used);
    const bytes = pkg.shards.reduce((sum, shard) => sum + shard.bytes, 0);
    return { id: pkg.id, label: pkg.label, bytes, state, ms: state === 'skipped' ? null : (timing?.ms ?? null), source: timing?.source ?? null } satisfies TableRow;
  });
  const order: Readonly<Record<TableState, number>> = { loaded: 0, failed: 1, 'built-in': 2, skipped: 3 };
  return [...fetched, ...compiled].sort((a, b) => order[a.state] - order[b.state] || a.label.localeCompare(b.label));
}

const STATE_TEXT: Readonly<Record<TableState, string>> = {
  'built-in': 'built into the engine',
  loaded: 'loaded',
  failed: 'could not be loaded',
  skipped: 'skipped: none of its code is in this run',
};

const SOURCE_TEXT: Readonly<Record<ContentSource, string>> = {
  memory: 'from memory',
  cache: 'from the browser cache',
  network: 'downloaded',
};

export function tableStateText(row: TableRow): string {
  const source = row.state === 'loaded' && row.source !== null ? ` (${SOURCE_TEXT[row.source]})` : '';
  return `${STATE_TEXT[row.state]}${source}`;
}
