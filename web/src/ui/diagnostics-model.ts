import { reportField } from '../report-json';

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
  const diagnostics = reportField(report, 'diagnostics');
  return Array.isArray(diagnostics) ? diagnostics.filter(isRow) : stderrRows(stderr);
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
  readonly pairs: readonly number[];
}

const SURROGATE_PAIR = /[\uD800-\uDBFF][\uDC00-\uDFFF]/g;

export function lineIndex(text: string): LineIndex {
  const starts = [0];
  for (let at = text.indexOf('\n'); at !== -1; at = text.indexOf('\n', at + 1)) {
    starts.push(at + 1);
  }
  const pairs = Array.from(text.matchAll(SURROGATE_PAIR), (match) => match.index);
  return { text, starts, pairs };
}

function lastAtMost(sorted: readonly number[], value: number): number {
  let low = 0;
  let high = sorted.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if ((sorted[mid] ?? 0) <= value) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return low;
}

export function lineColumn(index: LineIndex, offset: number): { line: number; column: number } {
  const end = Math.min(offset, index.text.length);
  const line = Math.max(1, lastAtMost(index.starts, offset));
  const start = index.starts[line - 1] ?? 0;
  const joined = end - start >= 2 ? lastAtMost(index.pairs, end - 2) - lastAtMost(index.pairs, start - 1) : 0;
  return { line, column: end - start - joined + 1 };
}

export function docPosition(text: string, offset: number): number {
  let joined = 0;
  for (let at = text.indexOf('\r\n'); at !== -1 && at < offset - 1; at = text.indexOf('\r\n', at + 2)) {
    joined += 1;
  }
  return offset - joined;
}

export function cappedProblems(problems: readonly string[], cap: number): readonly string[] {
  if (problems.length <= cap) {
    return problems;
  }
  return [...problems.slice(0, cap), `And ${(problems.length - cap).toLocaleString('en-US')} more settings notes.`];
}

export type StripTone = 'err' | 'warn' | 'muted' | 'text';

export interface StripLabel {
  readonly text: string;
  readonly tone: StripTone;
}

function counted(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? '' : 's'}`;
}

export function stripLabel(rows: readonly DiagnosticRow[] | null, notes: number): StripLabel {
  const count = (level: Level) => rows?.filter((row) => row.level === level).length ?? 0;
  const errors = count('error');
  const warnings = count('warning');
  const infos = count('info') + notes;
  if (errors > 0) {
    return { text: [counted(errors, 'error'), ...(warnings > 0 ? [counted(warnings, 'warning')] : [])].join(', '), tone: 'err' };
  }
  if (warnings > 0) {
    return { text: counted(warnings, 'warning'), tone: 'warn' };
  }
  if (infos > 0) {
    return { text: counted(infos, 'note'), tone: 'text' };
  }
  return rows === null ? { text: 'Diagnostics', tone: 'text' } : { text: 'No warnings', tone: 'muted' };
}
