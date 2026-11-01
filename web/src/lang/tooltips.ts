import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { EditorView, hoverTooltip, showTooltip, type Tooltip } from '@codemirror/view';
import { definitionAt, goToDefinition, parsed, referencesAt, showReferences, type DefinitionReply, type LangHooks, type RenameReply } from './extensions';

const openRename = StateEffect.define<number | null>();

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function button(label: string, hint: string, action: () => void): HTMLButtonElement {
  const node = el('button', 'cm-lang-btn', label);
  node.type = 'button';
  if (hint !== '') {
    node.append(el('span', 'cm-lang-key', hint));
  }
  node.addEventListener('mousedown', (event) => event.preventDefault());
  node.addEventListener('click', action);
  return node;
}

async function applyRename(view: EditorView, hooks: LangHooks, pos: number, name: string): Promise<string | null> {
  const reply = parsed<RenameReply>(await hooks.client.query('rename', pos, name));
  if (reply === null) {
    return 'The text changed; try again.';
  }
  if ('refused' in reply) {
    return `Not renamed: ${reply.refused}.`;
  }
  const changes = reply.edits.map(([from, to, insert]) => ({ from, to, insert }));
  view.dispatch({ changes, userEvent: 'rename', effects: openRename.of(null) });
  view.focus();
  return null;
}

function renameDom(view: EditorView, hooks: LangHooks, pos: number): HTMLElement {
  const word = view.state.wordAt(pos);
  const dom = el('div', 'cm-lang-card cm-lang-rename');
  dom.setAttribute('role', 'dialog');
  dom.setAttribute('aria-label', 'Rename symbol');
  const input = el('input', 'cm-lang-input');
  input.value = word === null ? '' : view.state.sliceDoc(word.from, word.to);
  input.setAttribute('aria-label', 'New name');
  const note = el('div', 'cm-lang-note', 'Enter to rename · Esc to cancel');
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      view.dispatch({ effects: openRename.of(null) });
      view.focus();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      void applyRename(view, hooks, pos, input.value.trim()).then((message) => {
        if (message !== null) {
          note.textContent = message;
          note.classList.add('cm-lang-refused');
        }
      });
    }
  });
  dom.append(el('div', 'cm-lang-title', 'Rename symbol'), input, note);
  return dom;
}

export function renaming(hooks: LangHooks): { readonly extension: Extension; readonly open: (view: EditorView) => boolean } {
  const field = StateField.define<Tooltip | null>({
    create: () => null,
    update(value, tr) {
      let next = tr.docChanged ? null : value;
      for (const effect of tr.effects) {
        if (effect.is(openRename)) {
          const pos = effect.value;
          next = pos === null ? null : { pos, above: true, create: (view) => ({ dom: renameDom(view, hooks, pos), mount: () => focusInput(view) }) };
        }
      }
      return next;
    },
    provide: (f) => showTooltip.from(f),
  });
  const open = (view: EditorView) => {
    view.dispatch({ effects: openRename.of(view.state.selection.main.head) });
    return true;
  };
  return { extension: field, open };
}

function focusInput(view: EditorView): void {
  const input = view.dom.querySelector<HTMLInputElement>('.cm-lang-rename input');
  input?.focus();
  input?.select();
}

function lineOf(view: EditorView, pos: number): number {
  return view.state.doc.lineAt(Math.min(pos, view.state.doc.length)).number;
}

function whereLabel(view: EditorView, definition: DefinitionReply): string {
  if (definition.module !== undefined) {
    const name = definition.name === '*' || definition.name === undefined ? '' : ` · ${definition.name}`;
    return `From ${definition.module}${name}`;
  }
  return `Declared on line ${lineOf(view, definition.span[0])}`;
}

function cardDom(view: EditorView, hooks: LangHooks, pos: number, definition: DefinitionReply, count: number, open: (view: EditorView) => boolean): HTMLElement {
  const word = view.state.wordAt(pos);
  const dom = el('div', 'cm-lang-card cm-lang-hover');
  const signature = el('div', 'cm-lang-sig');
  signature.append(el('span', 'cm-lang-kind', definition.kind), document.createTextNode(' '), el('span', 'cm-lang-name', word === null ? '' : view.state.sliceDoc(word.from, word.to)));
  const actions = el('div', 'cm-lang-actions');
  actions.append(
    button('Go to definition', 'F12', () => void goToDefinition(view, hooks, pos)),
    button(`${count} ${count === 1 ? 'reference' : 'references'}`, '', () => void showReferences(view, hooks.client, pos)),
    button('Rename symbol', '', () => {
      view.dispatch({ selection: { anchor: pos } });
      open(view);
    }),
  );
  dom.append(signature, el('div', 'cm-lang-where', whereLabel(view, definition)), actions);
  return dom;
}

export function hovering(hooks: LangHooks, open: (view: EditorView) => boolean): Extension {
  return hoverTooltip(async (view, pos) => {
    const word = view.state.wordAt(pos);
    if (word === null) {
      return null;
    }
    const definition = await definitionAt(hooks.client, pos);
    if (definition === null) {
      return null;
    }
    const count = (await referencesAt(hooks.client, pos)).length;
    return { pos: word.from, end: word.to, above: false, create: () => ({ dom: cardDom(view, hooks, pos, definition, count, open) }) };
  }, { hoverTime: 350 });
}
