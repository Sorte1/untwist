import { reportField } from '../report-json';

export interface LibraryRow {
  readonly library: string;
  readonly range: string;
  readonly modules: number;
}

export interface LibraryComment {
  readonly library: string;
  readonly range: string;
  readonly path: string;
}

const COMMENT = /^\s*\/\/ (\S+) ([^:\n]+): (\S[^\n]*)$/;

function text(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null;
}

function moduleCount(value: unknown): number {
  if (Array.isArray(value)) {
    return value.length;
  }
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : 1;
}

function rowOf(value: unknown): LibraryRow | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const row = value as Record<string, unknown>;
  const library = text(row['library']) ?? text(row['name']);
  if (library === null) {
    return null;
  }
  return { library, range: text(row['range']) ?? text(row['version']) ?? '', modules: moduleCount(row['modules']) };
}

export function librariesOf(report: string): LibraryRow[] {
  const field = reportField(report, 'libraries');
  if (!Array.isArray(field)) {
    return [];
  }
  const grouped = new Map<string, LibraryRow>();
  for (const row of field.map(rowOf)) {
    if (row === null) {
      continue;
    }
    const key = `${row.library}\u0000${row.range}`;
    const seen = grouped.get(key);
    grouped.set(key, seen === undefined ? row : { ...seen, modules: seen.modules + row.modules });
  }
  return [...grouped.values()].sort((a, b) => b.modules - a.modules || a.library.localeCompare(b.library) || a.range.localeCompare(b.range));
}

export function commentShape(line: string): LibraryComment | null {
  const match = COMMENT.exec(line);
  if (match === null) {
    return null;
  }
  const [, library = '', range = '', path = ''] = match;
  return { library, range: range.trim(), path: path.trim() };
}

export function libraryComment(line: string, known: ReadonlySet<string>): LibraryComment | null {
  const comment = commentShape(line);
  return comment !== null && known.has(comment.library) ? comment : null;
}

export function moduleLabel(count: number): string {
  return `${count.toLocaleString('en-US')} ${count === 1 ? 'module' : 'modules'}`;
}
