import type { EditorView } from '@codemirror/view';
import { signatureOf, type Builtin } from './builtins';
import { paramsLabel } from './complete';
import type { DefinitionReply } from './extensions';

export const SHOWN_CHARS = 300;

export function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

export function button(label: string, hint: string, action: () => void): HTMLButtonElement {
  const node = el('button', 'cm-lang-btn', label);
  node.type = 'button';
  if (hint !== '') {
    node.append(el('span', 'cm-lang-key', hint));
  }
  node.addEventListener('mousedown', (event) => event.preventDefault());
  node.addEventListener('click', action);
  return node;
}

export function shortened(text: string, limit = SHOWN_CHARS): string {
  return text.length > limit ? `${text.slice(0, limit)}…` : text;
}

function lineOf(view: EditorView, pos: number): number {
  return view.state.doc.lineAt(Math.min(pos, view.state.doc.length)).number;
}

export function whereLabel(view: EditorView, definition: DefinitionReply): string {
  if (definition.module !== undefined) {
    const name = definition.name === '*' || definition.name === undefined ? '' : ` · ${definition.name}`;
    return `From ${definition.module}${name}`;
  }
  return `Declared on line ${lineOf(view, definition.span[0])}`;
}

export function signatureLine(kind: string, name: string, definition: Pick<DefinitionReply, 'params' | 'form'>): HTMLElement {
  const line = el('div', 'cm-lang-sig');
  line.append(el('span', 'cm-lang-kind', kind), document.createTextNode(' '), el('span', 'cm-lang-name', name));
  if (definition.params !== undefined) {
    line.append(document.createTextNode(paramsLabel(definition.params)));
  }
  const form = definition.form;
  if (form !== undefined && form !== kind && !(definition.params !== undefined && (form === 'function' || form === 'class'))) {
    line.append(document.createTextNode(': '), el('span', 'cm-lang-type', form));
  }
  return line;
}

export function valueLine(value: string, decoded: boolean): HTMLElement {
  const line = el('div', 'cm-lang-value');
  line.append(el('span', 'cm-lang-op', '= '), el('span', 'cm-lang-literal', shortened(value)));
  if (decoded) {
    line.append(el('span', 'cm-lang-tag', 'decoded'));
  }
  return line;
}

export function builtinCard(builtin: Builtin, owner: string): HTMLElement {
  const dom = el('div', 'cm-lang-card cm-lang-hover');
  const kind = builtin.kind === 'namespace' ? 'namespace' : builtin.kind === 'class' ? 'class' : builtin.params === null ? 'property' : builtin.kind;
  const line = el('div', 'cm-lang-sig');
  line.append(el('span', 'cm-lang-kind', kind), document.createTextNode(' '), el('span', 'cm-lang-name', signatureOf(builtin, owner)));
  dom.append(line, el('div', 'cm-lang-doc', builtin.doc), el('div', 'cm-lang-where', 'Built in'));
  return dom;
}

export function stringCard(value: string): HTMLElement {
  const dom = el('div', 'cm-lang-card cm-lang-hover');
  const line = el('div', 'cm-lang-sig');
  line.append(el('span', 'cm-lang-kind', 'string'), document.createTextNode(' '), el('span', 'cm-lang-tag', 'decoded'));
  dom.append(line, el('div', 'cm-lang-value', ''));
  dom.lastElementChild?.append(el('span', 'cm-lang-literal', shortened(JSON.stringify(value))));
  dom.append(el('div', 'cm-lang-where', `${value.length.toLocaleString('en-US')} ${value.length === 1 ? 'character' : 'characters'}`));
  return dom;
}
