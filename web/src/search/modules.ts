export const HIT_LIMIT = 1000;

const BEFORE_CHARS = 20;
const AFTER_CHARS = 60;
const ELLIPSIS = '…';

export interface Hit {
  readonly module: number;
  readonly from: number;
  readonly to: number;
  readonly line: number;
  readonly column: number;
}

export interface Found {
  readonly hits: readonly Hit[];
  readonly modules: number;
  readonly capped: boolean;
}

export interface Snippet {
  readonly before: string;
  readonly match: string;
  readonly after: string;
}

function escaped(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function countLines(text: string, from: number, to: number): number {
  let count = 0;
  for (let at = text.indexOf('\n', from); at !== -1 && at < to; at = text.indexOf('\n', at + 1)) {
    count += 1;
  }
  return count;
}

function hitsIn(text: string, module: number, pattern: RegExp, room: number): Hit[] {
  const hits: Hit[] = [];
  pattern.lastIndex = 0;
  let line = 1;
  let lineStart = 0;
  let scanned = 0;
  for (let match = pattern.exec(text); match !== null && hits.length < room; match = pattern.exec(text)) {
    const from = match.index;
    const newlines = countLines(text, scanned, from);
    if (newlines > 0) {
      line += newlines;
      lineStart = text.lastIndexOf('\n', from - 1) + 1;
    }
    scanned = from;
    hits.push({ module, from, to: from + match[0].length, line, column: from - lineStart + 1 });
  }
  return hits;
}

export function searchModules(texts: readonly string[], order: readonly number[], query: string, limit = HIT_LIMIT): Found {
  if (query.trim() === '') {
    return { hits: [], modules: 0, capped: false };
  }
  const pattern = new RegExp(escaped(query), 'gi');
  const hits: Hit[] = [];
  let modules = 0;
  for (const module of order) {
    const found = hitsIn(texts[module] ?? '', module, pattern, limit + 1 - hits.length);
    modules += found.length > 0 ? 1 : 0;
    hits.push(...found);
    if (hits.length > limit) {
      return { hits: hits.slice(0, limit), modules, capped: true };
    }
  }
  return { hits, modules, capped: false };
}

export function snippetOf(text: string, from: number, to: number): Snippet {
  const start = text.lastIndexOf('\n', from - 1) + 1;
  const newline = text.indexOf('\n', to);
  const end = newline === -1 ? text.length : newline;
  const lead = text.slice(start, from).trimStart();
  const tail = text.slice(to, end);
  return {
    before: lead.length > BEFORE_CHARS ? `${ELLIPSIS}${lead.slice(-BEFORE_CHARS)}` : lead,
    match: text.slice(from, to),
    after: tail.length > AFTER_CHARS ? `${tail.slice(0, AFTER_CHARS)}${ELLIPSIS}` : tail,
  };
}
