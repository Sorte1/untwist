import { lineColumn, lineIndex, type LineIndex } from '../diagnostics-model';

const CALL_CHARS = 60;

export interface DecodedRow {
  readonly call: string;
  readonly value: string;
  readonly place: string;
  readonly span: readonly [number, number] | undefined;
}

export interface DecodedTable {
  readonly rows: readonly DecodedRow[];
  readonly distinct: number;
  readonly more: number;
}

type RawRow = readonly [number | null, number | null, string];

const EMPTY: DecodedTable = { rows: [], distinct: 0, more: 0 };

function isRaw(value: unknown): value is RawRow {
  return Array.isArray(value) && value.length === 3 && typeof value[2] === 'string';
}

function spanOf(row: RawRow, length: number): readonly [number, number] | undefined {
  const [start, end] = row;
  return start !== null && end !== null && start <= end && end <= length ? [start, end] : undefined;
}

function callOf(text: string, span: readonly [number, number]): string {
  const slice = text.slice(span[0], span[1]);
  const end = slice.indexOf('\n');
  const line = end === -1 ? slice : slice.slice(0, end);
  return line.length > CALL_CHARS || end !== -1 ? `${line.slice(0, CALL_CHARS)}…` : line;
}

function rowOf(index: LineIndex, raw: RawRow): DecodedRow {
  const span = spanOf(raw, index.text.length);
  if (span === undefined) {
    return { call: '', value: raw[2], place: '', span };
  }
  const { line, column } = lineColumn(index, span[0]);
  return { call: callOf(index.text, span), value: raw[2], place: `${line}:${column}`, span };
}

export function decodedStrings(json: string, text: string): DecodedTable {
  if (json === '') {
    return EMPTY;
  }
  try {
    const value = JSON.parse(json) as { rows?: unknown; more?: unknown };
    const raw = Array.isArray(value.rows) ? value.rows.filter(isRaw) : [];
    const index = lineIndex(text);
    const rows = raw.map((row) => rowOf(index, row));
    const more = typeof value.more === 'number' ? value.more : 0;
    return { rows, distinct: new Set(rows.map((row) => row.value)).size, more };
  } catch {
    return EMPTY;
  }
}

export function matching(rows: readonly DecodedRow[], query: string): readonly DecodedRow[] {
  const needle = query.trim().toLowerCase();
  if (needle === '') {
    return rows;
  }
  return rows.filter((row) => row.value.toLowerCase().includes(needle) || row.call.toLowerCase().includes(needle));
}

export function quoted(value: string): string {
  return JSON.stringify(value);
}
