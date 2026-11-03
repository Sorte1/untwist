import type { EditorState } from '@codemirror/state';

export type Edit = readonly [number, number, string];

export interface Part {
  readonly text: string;
  readonly changed: boolean;
}

export interface PreviewLine {
  readonly line: number;
  readonly parts: readonly Part[];
}

export const PREVIEW_LINES = 5;
const LEAD_CHARS = 40;
const LINE_CHARS = 160;

function lineParts(state: EditorState, number: number, edits: readonly Edit[]): Part[] {
  const line = state.doc.line(number);
  const first = edits[0]?.[0] ?? line.from;
  let at = Math.max(line.from, first - LEAD_CHARS);
  const parts: Part[] = at > line.from ? [{ text: '…', changed: false }] : [];
  for (const [from, to, insert] of edits) {
    parts.push({ text: state.sliceDoc(at, from), changed: false }, { text: insert, changed: true });
    at = to;
  }
  parts.push({ text: state.sliceDoc(at, Math.min(line.to, at + LINE_CHARS)), changed: false });
  const lead = parts[0];
  if (lead !== undefined && !lead.changed) {
    parts[0] = { text: lead.text.trimStart(), changed: false };
  }
  return parts.filter((part) => part.text !== '');
}

export function previewLines(state: EditorState, edits: readonly Edit[]): PreviewLine[] {
  const byLine = new Map<number, Edit[]>();
  for (const edit of [...edits].sort((a, b) => a[0] - b[0])) {
    const number = state.doc.lineAt(Math.min(edit[0], state.doc.length)).number;
    byLine.set(number, [...(byLine.get(number) ?? []), edit]);
  }
  return [...byLine.entries()].map(([line, own]) => ({ line, parts: lineParts(state, line, own) }));
}

export function previewNote(edits: number, lines: number): string {
  const references = `${edits.toLocaleString('en-US')} ${edits === 1 ? 'reference' : 'references'}`;
  return `${references} on ${lines.toLocaleString('en-US')} ${lines === 1 ? 'line' : 'lines'} · Enter to rename`;
}
