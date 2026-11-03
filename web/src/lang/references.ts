import { Prec, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, keymap, showPanel, type DecorationSet, type Panel, type ViewUpdate } from '@codemirror/view';
import { CLOSE_PATH, strokeIcon } from '../ui/icon';
import type { LangClient } from './client';
import { referencesAt, within, type Span } from './queries';

export const MAX_ROWS = 1000;
const CONTEXT_CHARS = 60;
const ROW_CHARS = 160;

export interface Found {
  readonly name: string;
  readonly refs: readonly Span[];
  readonly selected: number;
}

export interface RowText {
  readonly line: number;
  readonly before: string;
  readonly match: string;
  readonly after: string;
}

const setFound = StateEffect.define<Found | null>();
const referenceMark = Decoration.mark({ class: 'cm-lang-reference' });
const currentMark = Decoration.mark({ class: 'cm-lang-reference cm-lang-reference-current' });

export function rowText(state: EditorState, ref: Span): RowText {
  const line = state.doc.lineAt(ref.from);
  const start = Math.max(line.from, ref.from - CONTEXT_CHARS);
  const end = Math.min(line.to, ref.to, ref.from + ROW_CHARS);
  const before = state.sliceDoc(start, ref.from);
  const rest = Math.min(line.to, start + ROW_CHARS);
  return {
    line: line.number,
    before: start === line.from ? before.trimStart() : `…${before}`,
    match: state.sliceDoc(ref.from, end),
    after: state.sliceDoc(end, Math.max(end, rest)),
  };
}

export function countLabel(count: number, name: string): string {
  return `${count.toLocaleString('en-US')} ${count === 1 ? 'reference' : 'references'} to ${name}`;
}

export function stepped(found: Found, step: number): number {
  const count = found.refs.length;
  return count === 0 ? 0 : (((found.selected + step) % count) + count) % count;
}

function marksOf(state: EditorState, found: Found | null): DecorationSet {
  if (found === null) {
    return Decoration.none;
  }
  const ranges = found.refs
    .map((ref, index) => ({ ...within(state, ref.from, ref.to), index }))
    .filter(({ from, to }) => to > from)
    .map(({ from, to, index }) => (index === found.selected ? currentMark : referenceMark).range(from, to));
  return Decoration.set(ranges, true);
}

const foundField = StateField.define<Found | null>({
  create: () => null,
  update(value, tr) {
    let next = tr.docChanged ? null : value;
    for (const effect of tr.effects) {
      if (effect.is(setFound)) {
        next = effect.value;
      }
    }
    return next;
  },
  provide: (field) => [
    EditorView.decorations.compute([field], (state) => marksOf(state, state.field(field))),
    showPanel.from(field, (found) => (found === null ? null : referencesPanel)),
  ],
});

function jump(view: EditorView, found: Found, index: number): void {
  const ref = found.refs[index];
  if (ref === undefined) {
    return;
  }
  const { from, to } = within(view.state, ref.from, ref.to);
  view.dispatch({ selection: { anchor: from, head: to }, scrollIntoView: true, effects: setFound.of({ ...found, selected: index }) });
  view.focus();
}

function span(className: string, text: string): HTMLElement {
  const node = document.createElement('span');
  node.className = className;
  node.textContent = text;
  return node;
}

function row(view: EditorView, found: Found, index: number): HTMLElement {
  const ref = found.refs[index] ?? { from: 0, to: 0 };
  const text = rowText(view.state, ref);
  const item = document.createElement('li');
  item.className = 'cm-lang-refs-row';
  item.setAttribute('role', 'option');
  item.setAttribute('aria-selected', String(index === found.selected));
  const code = span('cm-lang-refs-text', text.before);
  code.append(span('cm-lang-refs-match', text.match), document.createTextNode(text.after));
  item.append(span('cm-lang-refs-line', String(text.line)), code);
  item.addEventListener('mousedown', (event) => event.preventDefault());
  item.addEventListener('click', () => jump(view, view.state.field(foundField) ?? found, index));
  return item;
}

function render(view: EditorView, found: Found, head: HTMLElement, list: HTMLElement): void {
  head.textContent = countLabel(found.refs.length, found.name);
  const shown = Math.min(found.refs.length, MAX_ROWS);
  list.replaceChildren(...Array.from({ length: shown }, (_, index) => row(view, found, index)));
  if (found.refs.length > shown) {
    const more = span('cm-lang-refs-more', `Showing the first ${MAX_ROWS.toLocaleString('en-US')}`);
    list.append(more);
  }
  list.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
}

function referencesPanel(view: EditorView): Panel {
  const dom = document.createElement('div');
  dom.className = 'cm-lang-refs';
  dom.setAttribute('role', 'region');
  dom.setAttribute('aria-label', 'References');
  const bar = document.createElement('div');
  bar.className = 'cm-lang-refs-bar';
  const head = span('cm-lang-refs-head', '');
  const close = document.createElement('button');
  close.type = 'button';
  close.className = 'cm-lang-refs-close';
  close.setAttribute('aria-label', 'Close the references');
  close.append(strokeIcon(CLOSE_PATH, 14));
  close.addEventListener('mousedown', (event) => event.preventDefault());
  close.addEventListener('click', () => closeReferences(view));
  bar.append(head, close);
  const list = document.createElement('ul');
  list.className = 'cm-lang-refs-list';
  list.setAttribute('role', 'listbox');
  list.setAttribute('aria-label', 'References');
  dom.append(bar, list);
  let shown = view.state.field(foundField);
  if (shown !== null) {
    render(view, shown, head, list);
  }
  return {
    dom,
    update(update: ViewUpdate) {
      const next = update.state.field(foundField);
      if (next !== null && next !== shown) {
        shown = next;
        render(update.view, next, head, list);
      }
    },
  };
}

export function closeReferences(view: EditorView): boolean {
  if (view.state.field(foundField, false) == null) {
    return false;
  }
  view.dispatch({ effects: setFound.of(null) });
  view.focus();
  return true;
}

function step(view: EditorView, by: number): boolean {
  const found = view.state.field(foundField, false);
  if (found === null || found === undefined || found.refs.length === 0) {
    return false;
  }
  jump(view, found, stepped(found, by));
  return true;
}

export async function showReferences(view: EditorView, client: LangClient, pos: number): Promise<number> {
  const refs = await referencesAt(client, pos);
  const word = view.state.wordAt(pos);
  const name = word === null ? '' : view.state.sliceDoc(word.from, word.to);
  const selected = Math.max(0, refs.findIndex((ref) => ref.from <= pos && pos <= ref.to));
  view.dispatch({ effects: setFound.of(refs.length === 0 ? null : { name, refs, selected }) });
  return refs.length;
}

const panelLook = EditorView.theme({
  '.cm-lang-reference-current': { backgroundColor: 'color-mix(in srgb, var(--accent) 26%, transparent)', boxShadow: 'inset 0 0 0 1px var(--accent)' },
  '.cm-lang-refs': { borderTop: '1px solid var(--hair)', fontFamily: 'var(--sans)', fontSize: '12px', color: 'var(--text)' },
  '.cm-lang-refs-bar': { display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '4px 8px 4px 14px', color: 'var(--muted)' },
  '.cm-lang-refs-close': {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '24px',
    height: '24px',
    padding: '0',
    border: 'none',
    borderRadius: '5px',
    backgroundColor: 'transparent',
    color: 'var(--muted)',
    cursor: 'pointer',
  },
  '.cm-lang-refs-close:hover': { backgroundColor: 'var(--sel)', color: 'var(--text)' },
  '.cm-lang-refs-list': { listStyle: 'none', margin: '0', padding: '0 6px 6px', maxHeight: '168px', overflowY: 'auto' },
  '.cm-lang-refs-row': {
    display: 'flex',
    gap: '12px',
    alignItems: 'baseline',
    height: '24px',
    lineHeight: '24px',
    padding: '0 8px',
    borderRadius: '5px',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
  },
  '.cm-lang-refs-row:hover': { backgroundColor: 'var(--sel)' },
  '.cm-lang-refs-row[aria-selected="true"]': { backgroundColor: 'var(--selection)', boxShadow: 'inset 2px 0 0 var(--accent)' },
  '.cm-lang-refs-line': { minWidth: '32px', textAlign: 'right', fontFamily: 'var(--mono)', fontSize: '11px', color: 'var(--faint)' },
  '.cm-lang-refs-text': { overflow: 'hidden', textOverflow: 'ellipsis', fontFamily: 'var(--mono)', color: 'var(--plain)' },
  '.cm-lang-refs-match': { color: 'var(--text)', fontWeight: '600', backgroundColor: 'color-mix(in srgb, var(--accent) 18%, transparent)', borderRadius: '2px' },
  '.cm-lang-refs-more': { display: 'block', padding: '4px 8px', color: 'var(--muted)' },
});

export function referencesPanelExtension(): Extension {
  return [
    foundField,
    panelLook,
    keymap.of([{ key: 'F4', run: (view) => step(view, 1), shift: (view) => step(view, -1), preventDefault: true }]),
    Prec.high(keymap.of([{ key: 'Escape', run: closeReferences }])),
  ];
}
