import { reportField } from '../report-json';

export interface LibraryRow {
  readonly library: string;
  readonly range: string;
  readonly disagree: boolean;
  readonly modules: number;
}

export interface LibraryMatch {
  readonly library: string;
  readonly range: string;
  readonly modules: readonly string[];
}

export interface LibraryComment extends LibraryMatch {
  readonly also: readonly LibraryMatch[];
}

const COMMENT = /^\s*\/\/ (\S.*)$/;
const PART = /^(\S+) ([^:\s][^:]*): (\S.*)$/;
const SAME_CODE = ', same code in ';
const MODULE_SEPARATOR = ' | ';

function rowOf(value: unknown): LibraryRow | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const { library, range, disagree, modules } = value as Record<string, unknown>;
  if (typeof library !== 'string' || library === '' || typeof range !== 'string') {
    return null;
  }
  const count = typeof modules === 'number' && Number.isSafeInteger(modules) && modules >= 0 ? modules : 0;
  return { library, range, disagree: disagree === true, modules: count };
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
    const seen = grouped.get(row.library);
    grouped.set(row.library, seen === undefined ? row : { ...seen, modules: seen.modules + row.modules });
  }
  return [...grouped.values()].sort((a, b) => b.modules - a.modules || a.library.localeCompare(b.library));
}

function matchOf(part: string): LibraryMatch | null {
  const match = PART.exec(part);
  if (match === null) {
    return null;
  }
  const [, library = '', range = '', modules = ''] = match;
  return { library, range: range.trim(), modules: modules.split(MODULE_SEPARATOR).map((module) => module.trim()) };
}

export function commentShape(line: string): LibraryComment | null {
  const body = COMMENT.exec(line)?.[1];
  if (body === undefined) {
    return null;
  }
  const parts = body.split(SAME_CODE).map(matchOf);
  const [first, ...rest] = parts;
  if (first === undefined || first === null || rest.some((part) => part === null)) {
    return null;
  }
  return { ...first, also: rest.filter((part): part is LibraryMatch => part !== null) };
}

export function libraryComment(line: string, known: ReadonlySet<string>): LibraryComment | null {
  const comment = commentShape(line);
  if (comment === null) {
    return null;
  }
  return known.has(comment.library) || comment.also.some((same) => known.has(same.library)) ? comment : null;
}

export function rangeLabel(row: LibraryRow): string {
  return row.disagree ? `${row.range}, matches disagree` : row.range;
}

export function moduleLabel(count: number): string {
  return `${count.toLocaleString('en-US')} ${count === 1 ? 'module' : 'modules'}`;
}
