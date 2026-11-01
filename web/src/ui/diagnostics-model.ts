export type Level = 'error' | 'warning' | 'info';

export interface DiagnosticRow {
  readonly level: Level;
  readonly pass: string;
  readonly message: string;
  readonly inputSpan?: readonly [number, number];
}

const LEVELS: readonly Level[] = ['error', 'warning', 'info'];
const PREFIX = /^(?:warning|error): /;

function isRow(value: unknown): value is DiagnosticRow {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const row = value as Record<string, unknown>;
  const span = row['inputSpan'];
  const spanOk =
    span === undefined || (Array.isArray(span) && span.length === 2 && span.every((at) => Number.isInteger(at) && at >= 0));
  return LEVELS.includes(row['level'] as Level) && typeof row['pass'] === 'string' && typeof row['message'] === 'string' && spanOk;
}

function stderrRows(stderr: string): DiagnosticRow[] {
  return stderr
    .split('\n')
    .filter((line) => line !== '')
    .map((line) => ({ level: 'error', pass: 'flags', message: line }));
}

export function diagnosticsOf(report: string, stderr: string): DiagnosticRow[] {
  try {
    const value = JSON.parse(report) as { diagnostics?: unknown };
    if (Array.isArray(value.diagnostics)) {
      return value.diagnostics.filter(isRow);
    }
  } catch {
    return stderrRows(stderr);
  }
  return stderrRows(stderr);
}

export function warningCount(rows: readonly DiagnosticRow[]): number {
  return rows.filter((row) => row.level !== 'info').length;
}

export function levelLabel(level: Level): string {
  return level === 'warning' ? 'warn' : level;
}

export function shownMessage(row: DiagnosticRow): string {
  return row.message.replace(PREFIX, '');
}

export interface LineIndex {
  readonly text: string;
  readonly starts: readonly number[];
}

export function lineIndex(text: string): LineIndex {
  const starts = [0];
  for (let at = text.indexOf('\n'); at !== -1; at = text.indexOf('\n', at + 1)) {
    starts.push(at + 1);
  }
  return { text, starts };
}

export function lineColumn(index: LineIndex, offset: number): { line: number; column: number } {
  let low = 0;
  let high = index.starts.length - 1;
  while (low < high) {
    const mid = (low + high + 1) >> 1;
    if ((index.starts[mid] ?? 0) <= offset) {
      low = mid;
    } else {
      high = mid - 1;
    }
  }
  const start = index.starts[low] ?? 0;
  const column = [...index.text.slice(start, Math.min(offset, index.text.length))].length + 1;
  return { line: low + 1, column };
}

export function docPosition(text: string, offset: number): number {
  let joined = 0;
  for (let at = text.indexOf('\r\n'); at !== -1 && at < offset - 1; at = text.indexOf('\r\n', at + 2)) {
    joined += 1;
  }
  return offset - joined;
}
