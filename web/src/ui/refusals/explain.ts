import { lineColumn, lineIndex, type LineIndex } from '../diagnostics-model';

export const SHOWN_SITES = 100;
const CODE_CHARS = 80;
const DECODE_PASS = 'inline-decoded-strings';
const UNTRUSTED = "so the string decoder can't be trusted.";

export interface Explanation {
  readonly place: string;
  readonly line: number | null;
  readonly code: string;
  readonly sentence: string;
  readonly span?: readonly [number, number];
}

export interface Explained {
  readonly shown: readonly Explanation[];
  readonly more: number;
}

interface Site {
  readonly what: string;
  readonly layer: number;
  readonly inputSpan?: readonly [number, number];
}

interface NoteRow {
  readonly pass: string;
  readonly kind: string;
  readonly message: string;
  readonly inputSpan?: readonly [number, number];
}

const READS: Readonly<Record<string, string>> = {
  'escaped read': `reads a property by a key Safe mode can't work out, which may hand out a built-in, ${UNTRUSTED}`,
  'reflective read': `reads a property by a key Safe mode can't work out, which may hand out a built-in, ${UNTRUSTED}`,
  'forwarded reflective read': `reads a property by a key Safe mode can't work out, which may hand out a built-in, ${UNTRUSTED}`,
  'destructured read': `destructures an object in a way that may hand out a built-in, ${UNTRUSTED}`,
  'rest pattern': `destructures an object in a way that may hand out a built-in, ${UNTRUSTED}`,
  'escaped runner name': `lets a built-in that runs code escape, ${UNTRUSTED}`,
  'code from text': `may run code built from text, which Safe mode can't see into, ${UNTRUSTED}`,
  'an intrinsic passed to a user function': `hands a built-in to one of the program's functions, which may change it, ${UNTRUSTED}`,
  "a folded chain's assumed key may be written": `may write a key the decoder's lookups assume, ${UNTRUSTED}`,
  'write by an unknown key': `writes a property by a key Safe mode can't work out, which may replace a built-in, ${UNTRUSTED}`,
  GLOBAL_REACH: 'may reach the string array through the global object by a key that could be its name, so the decoder calls were left alone.',
};

const PATCHED = /^BUILTIN_PATCHED(?: write)?: (.*?)(?:, from .*)?$/;
const KEYED = /^(.*)\[(a computed key|a key)\]$/;
const ROLES = / \([a-z ]+\)$/;

function written(target: string, write: boolean): string {
  const keyed = KEYED.exec(target);
  if (keyed !== null) {
    return `writes to ${keyed[1]} by ${keyed[2]}, ${UNTRUSTED}`;
  }
  return `${write ? 'writes to' : 'replaces'} ${target}, ${UNTRUSTED}`;
}

export function sentenceFor(what: string): string {
  const patched = PATCHED.exec(what);
  if (patched !== null) {
    return written(patched[1] ?? '', what.startsWith('BUILTIN_PATCHED write'));
  }
  return READS[what.replace(ROLES, '')] ?? `${what}.`;
}

function isSpan(value: unknown): value is readonly [number, number] {
  return Array.isArray(value) && value.length === 2 && value.every((at) => Number.isInteger(at) && at >= 0);
}

function parsed(report: string): { sites: Site[]; notes: NoteRow[] } {
  try {
    const value = JSON.parse(report) as { refusalSites?: unknown; notes?: unknown };
    const sites = Array.isArray(value.refusalSites) ? value.refusalSites.filter((site): site is Site => typeof site?.what === 'string') : [];
    const notes = Array.isArray(value.notes) ? value.notes.filter((note): note is NoteRow => typeof note?.message === 'string') : [];
    return { sites, notes };
  } catch {
    return { sites: [], notes: [] };
  }
}

function quoted(text: string, span: readonly [number, number]): string {
  const slice = text.slice(span[0], span[1]);
  const end = slice.indexOf('\n');
  const line = end === -1 ? slice : slice.slice(0, end);
  return line.length > CODE_CHARS || end !== -1 ? `${line.slice(0, CODE_CHARS)}…` : line;
}

function explained(index: LineIndex, { sentence, span, layer }: Reason): Explanation {
  if (span === undefined || span[1] > index.text.length) {
    return { place: layer > 0 ? 'Unpacked code' : 'Rewritten code', line: null, code: '', sentence };
  }
  const { line } = lineColumn(index, span[0]);
  return { place: `Line ${line.toLocaleString('en-US')}`, line, code: quoted(index.text, span), sentence, span };
}

interface Reason {
  readonly what: string;
  readonly sentence: string;
  readonly span: readonly [number, number] | undefined;
  readonly layer: number;
}

function reasons(report: string): Reason[] {
  const { sites, notes } = parsed(report);
  if (sites.length > 0) {
    return sites.map((site) => ({
      what: site.what,
      sentence: sentenceFor(site.what),
      span: site.layer === 0 && isSpan(site.inputSpan) ? site.inputSpan : undefined,
      layer: site.layer,
    }));
  }
  return notes
    .filter((note) => note.kind === 'refusal' && note.pass === DECODE_PASS)
    .map((note) => ({ what: '', sentence: note.message, span: isSpan(note.inputSpan) ? note.inputSpan : undefined, layer: 0 }));
}

function uniqueBy<T>(items: readonly T[], key: (item: T) => string): T[] {
  const seen = new Set<string>();
  return items.filter((item) => {
    const name = key(item);
    if (seen.has(name)) {
      return false;
    }
    seen.add(name);
    return true;
  });
}

function rank(reason: Reason): number {
  return reason.what.startsWith('BUILTIN_PATCHED') ? 0 : reason.what === 'GLOBAL_REACH' ? 1 : 2;
}

function placeOf({ sentence, span, layer }: Reason): string {
  return span === undefined ? `layer ${layer}|${sentence}` : span.join(',');
}

export function explainRefusals(report: string, text: string): Explained {
  const index = lineIndex(text);
  const ranked = reasons(report).sort((a, b) => rank(a) - rank(b));
  const rows = uniqueBy(ranked, placeOf)
    .map((reason) => explained(index, reason))
    .sort((a, b) => (a.span?.[0] ?? Infinity) - (b.span?.[0] ?? Infinity));
  return { shown: rows.slice(0, SHOWN_SITES), more: Math.max(0, rows.length - SHOWN_SITES) };
}
