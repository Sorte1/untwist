import type { LibraryTablesUsed } from '../engine/types';
import type { TableIndex } from './index-format';
import type { TableSource } from './tables';

export type TableState = 'built-in' | 'loaded' | 'failed' | 'skipped';

export interface TableRow {
  readonly id: string;
  readonly label: string;
  readonly bytes: number | null;
  readonly state: TableState;
  readonly ms: number | null;
  readonly source: TableSource | null;
}

function fetchedState(id: string, used: LibraryTablesUsed | undefined): TableState {
  if (used?.fetched.includes(id) === true) {
    return 'loaded';
  }
  return used?.failed.includes(id) === true ? 'failed' : 'skipped';
}

export function tableRows(index: TableIndex | null, used: LibraryTablesUsed | undefined): TableRow[] {
  if (index === null) {
    return [];
  }
  const builtIn = index.compiledIn.map((id) => ({ id, label: id, bytes: null, state: 'built-in', ms: null, source: null }) satisfies TableRow);
  const fetched = index.tables.map((table) => {
    const timing = used?.timings?.find((row) => row.id === table.id);
    const state = fetchedState(table.id, used);
    return { id: table.id, label: table.label, bytes: table.bytes, state, ms: state === 'skipped' ? null : (timing?.ms ?? null), source: timing?.source ?? null } satisfies TableRow;
  });
  const order: Readonly<Record<TableState, number>> = { loaded: 0, failed: 1, 'built-in': 2, skipped: 3 };
  return [...fetched, ...builtIn].sort((a, b) => order[a.state] - order[b.state] || a.label.localeCompare(b.label));
}

const STATE_TEXT: Readonly<Record<TableState, string>> = {
  'built-in': 'built into the engine',
  loaded: 'loaded',
  failed: 'could not be loaded',
  skipped: 'skipped: too few of its names in the code',
};

const SOURCE_TEXT: Readonly<Record<TableSource, string>> = {
  memory: 'from memory',
  cache: 'from the browser cache',
  network: 'downloaded',
};

export function tableStateText(row: TableRow): string {
  const source = row.state === 'loaded' && row.source !== null ? ` (${SOURCE_TEXT[row.source]})` : '';
  return `${STATE_TEXT[row.state]}${source}`;
}
