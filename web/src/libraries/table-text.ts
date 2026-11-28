const HEADER = '#untwist-library-table';
const FORMAT = 'v2';

export interface TableSummary {
  readonly id: string;
  readonly label: string;
  readonly versions: readonly string[];
  readonly loose: readonly number[];
  readonly rows: number;
}

type Range = [string, string];

export function versionKey(version: string): number[] {
  return version.split(/[.+-]/).map((part) => (/^\d+$/.test(part) ? Number(part) : 0));
}

export function compareVersions(a: string, b: string): number {
  const left = versionKey(a);
  const right = versionKey(b);
  for (let at = 0; at < Math.min(left.length, right.length); at += 1) {
    const diff = (left[at] ?? 0) - (right[at] ?? 0);
    if (diff !== 0) {
      return diff;
    }
  }
  return left.length - right.length;
}

function normalise(ranges: readonly Range[]): Range[] {
  const sorted = ranges.map((range, at) => ({ range, at })).sort((a, b) => compareVersions(a.range[0], b.range[0]) || a.at - b.at);
  const merged: Range[] = [];
  for (const { range } of sorted) {
    const last = merged.at(-1);
    if (last !== undefined && compareVersions(range[0], last[1]) <= 0) {
      if (compareVersions(range[1], last[1]) > 0) {
        last[1] = range[1];
      }
    } else {
      merged.push([range[0], range[1]]);
    }
  }
  return merged;
}

export function parseRanges(versions: string): Range[] {
  const ranges = versions
    .split(',')
    .map((range) => range.trim())
    .filter((range) => range !== '')
    .map((range): Range => {
      const split = range.includes('~') ? range.indexOf('~') : range.indexOf('–');
      return split === -1 ? [range, range] : [range.slice(0, split).trim(), range.slice(split + 1).trim()];
    })
    .filter(([low, high]) => compareVersions(low, high) <= 0);
  return normalise(ranges);
}

function counted(line: string | undefined, tag: string): number {
  const [found, count] = (line ?? '').split('\t');
  if (found !== tag || count === undefined || !/^\d+$/.test(count)) {
    throw new Error(`library table: expected ${tag}`);
  }
  return Number(count);
}

export function summariseTable(text: string): TableSummary {
  const lines = text.split('\n');
  const header = (lines[0] ?? '').split('\t');
  const id = header[2];
  if (header[0] !== HEADER || header[1] !== FORMAT || id === undefined || id === '') {
    throw new Error('library table: not a v2 table');
  }
  let at = 1;
  const strings = lines.slice(at + 1, at + 1 + counted(lines[at], '#strings'));
  at += 1 + strings.length;
  at += 1 + counted(lines[at], '#lists');
  const rowCount = counted(lines[at], '#rows');
  const rows = lines.slice(at + 1, at + 1 + rowCount).map((line) => line.split('\t'));
  if (rows.length !== rowCount) {
    throw new Error('library table: fewer rows than its count');
  }
  const loose = rows.map((fields) => Number.parseInt(fields[0] ?? '', 16));
  const first = rows.reduce<string[] | null>((best, fields) => (best === null || (fields[1] ?? '') < (best[1] ?? '') ? fields : best), null);
  const nameAt = (index: string | undefined) => strings[Number(index)] ?? '';
  const versions = [...new Set(rows.flatMap((fields) => parseRanges(nameAt(fields[3])).flat()))].sort((a, b) => compareVersions(a, b) || (a < b ? -1 : a > b ? 1 : 0));
  return { id, label: first === null ? id : nameAt(first[2]), versions, loose, rows: rowCount };
}
