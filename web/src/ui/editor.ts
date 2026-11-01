import { javascriptLanguage } from '@codemirror/lang-javascript';
import { HighlightStyle, LanguageSupport, syntaxHighlighting } from '@codemirror/language';
import { EditorState, Prec, type Extension } from '@codemirror/state';
import { drawSelection, EditorView, highlightSpecialChars, keymap, lineNumbers } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
import { tags } from '@lezer/highlight';

const highlight = HighlightStyle.define([
  { tag: [tags.keyword, tags.controlKeyword, tags.moduleKeyword, tags.operatorKeyword, tags.definitionKeyword, tags.modifier, tags.self, tags.bool, tags.null], color: 'var(--kw)' },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: 'var(--str)' },
  { tag: [tags.number], color: 'var(--num)' },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], color: 'var(--fn)' },
  { tag: [tags.comment, tags.lineComment, tags.blockComment], color: 'var(--com)' },
]);

const look = EditorView.theme({
  '&': { flex: '1 1 auto', minWidth: '0', height: '100%', backgroundColor: 'var(--code)', color: 'var(--plain)', fontSize: '12.5px' },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '20px', padding: '10px 0' },
  '.cm-content': { padding: '0', caretColor: 'var(--text)' },
  '.cm-line': { padding: '0 16px 0 0' },
  '.cm-gutters': { backgroundColor: 'var(--code)', color: 'var(--faint)', border: 'none' },
  '.cm-lineNumbers .cm-gutterElement': { minWidth: '44px', padding: '0 14px 0 0', boxSizing: 'border-box' },
  '.cm-activeLine': { backgroundColor: 'transparent' },
  '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
    backgroundColor: 'var(--selection) !important',
    boxShadow: 'inset 0 0 0 1px var(--info)',
  },
  '.cm-cursor': { borderLeftColor: 'var(--text)' },
  '.cm-selectionMatch': { backgroundColor: 'var(--selection)', outline: '1px solid var(--btn-border)' },
  '.cm-panels': { backgroundColor: 'var(--head)', color: 'var(--text)', borderColor: 'var(--border)' },
});

export const outputLook = Prec.highest(
  EditorView.theme({
    '.cm-line': { padding: '0 16px 0 12px' },
    '.cm-lineNumbers .cm-gutterElement': { padding: '0 12px 0 0' },
  }),
);

export function access(editable: boolean): Extension {
  return [EditorState.readOnly.of(!editable), EditorView.editable.of(editable)];
}

export function baseExtensions(editable: boolean, gate: Extension = access(editable)): Extension[] {
  return [
    lineNumbers(),
    highlightSpecialChars(),
    drawSelection(),
    highlightSelectionMatches(),
    new LanguageSupport(javascriptLanguage),
    syntaxHighlighting(highlight),
    look,
    gate,
    editable ? [] : EditorView.contentAttributes.of({ tabindex: '0' }),
    editable ? history() : [],
    keymap.of([...defaultKeymap, ...(editable ? historyKeymap : []), ...searchKeymap]),
  ];
}

export function makeEditor(parent: HTMLElement, label: string, extensions: Extension[]): EditorView {
  const host = document.createElement('div');
  host.className = 'editor-host';
  parent.append(host);
  const root = host.attachShadow({ mode: 'open' });
  const view = new EditorView({ parent: root, root, state: EditorState.create({ doc: '', extensions }) });
  view.contentDOM.setAttribute('aria-label', label);
  return view;
}

export function replaceDoc(view: EditorView, text: string): void {
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: { anchor: 0 }, scrollIntoView: true });
}
