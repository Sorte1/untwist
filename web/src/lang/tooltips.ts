import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { closeHoverTooltips, EditorView, hoverTooltip, showTooltip, type Tooltip } from '@codemirror/view';
import { globals, memberBuiltin } from './builtins';
import { builtinCard, button, el, signatureLine, stringCard, valueLine, whereLabel } from './cards';
import { describeAt, ownerOf } from './complete';
import { memberNameAt, stringAt } from './context';
import { definitionAt, goToDefinition, parsed, referencesAt, showReferences, type DefinitionReply, type LangHooks, type RenameReply } from './extensions';
import { decodeJsString, hasEscapes } from './js-string';
import { PREVIEW_LINES, previewLines, previewNote, type PreviewLine } from './rename-preview';

const PREVIEW_MS = 120;
const DEFAULT_NOTE = 'Enter to rename · Esc to cancel';

const openRename = StateEffect.define<number | null>();

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

function previewRows(preview: HTMLElement, lines: readonly PreviewLine[]): void {
  const rows = lines.slice(0, PREVIEW_LINES).map((line) => {
    const row = el('div', 'cm-lang-rename-line');
    const text = el('span', 'cm-lang-rename-text');
    text.append(...line.parts.map((part) => (part.changed ? el('mark', 'cm-lang-rename-new', part.text) : document.createTextNode(part.text))));
    row.append(el('span', 'cm-lang-rename-num', String(line.line)), text);
    return row;
  });
  const more = lines.length - rows.length;
  preview.replaceChildren(...rows, ...(more > 0 ? [el('div', 'cm-lang-rename-more', `and ${more.toLocaleString('en-US')} more ${more === 1 ? 'line' : 'lines'}`)] : []));
}

function renamePreview(view: EditorView, hooks: LangHooks, pos: number, input: HTMLInputElement, note: HTMLElement, preview: HTMLElement): () => void {
  const original = input.value;
  let ticket = 0;
  const say = (text: string, refused: boolean) => {
    note.textContent = text;
    note.classList.toggle('cm-lang-refused', refused);
  };
  return () => {
    ticket += 1;
    const asked = ticket;
    const name = input.value.trim();
    if (name === '' || name === original) {
      say(DEFAULT_NOTE, false);
      preview.replaceChildren();
      return;
    }
    void hooks.client.query('rename', pos, name).then((value) => {
      if (asked !== ticket) {
        return;
      }
      const reply = parsed<RenameReply>(value);
      if (reply === null || 'refused' in reply) {
        say(reply === null ? 'The text changed; try again.' : `Not renamed: ${reply.refused}.`, true);
        preview.replaceChildren();
        return;
      }
      const lines = previewLines(view.state, reply.edits);
      say(previewNote(reply.edits.length, lines.length), false);
      previewRows(preview, lines);
    });
  };
}

function renameDom(view: EditorView, hooks: LangHooks, pos: number): HTMLElement {
  const word = view.state.wordAt(pos);
  const dom = el('div', 'cm-lang-card cm-lang-rename');
  dom.setAttribute('role', 'dialog');
  dom.setAttribute('aria-label', 'Rename symbol');
  const input = el('input', 'cm-lang-input');
  input.value = word === null ? '' : view.state.sliceDoc(word.from, word.to);
  input.setAttribute('aria-label', 'New name');
  const note = el('div', 'cm-lang-note', DEFAULT_NOTE);
  note.setAttribute('aria-live', 'polite');
  const preview = el('div', 'cm-lang-rename-preview');
  const refresh = renamePreview(view, hooks, pos, input, note, preview);
  let timer: ReturnType<typeof setTimeout> | null = null;
  input.addEventListener('input', () => {
    if (timer !== null) {
      clearTimeout(timer);
    }
    timer = setTimeout(refresh, PREVIEW_MS);
  });
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
          preview.replaceChildren();
        }
      });
    }
  });
  dom.append(el('div', 'cm-lang-title', 'Rename symbol'), input, note, preview);
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
    view.dispatch({ effects: [openRename.of(view.state.selection.main.head), closeHoverTooltips] });
    return true;
  };
  return { extension: field, open };
}

function focusInput(view: EditorView): void {
  const input = view.dom.querySelector<HTMLInputElement>('.cm-lang-rename input');
  input?.focus();
  input?.select();
}

function cardDom(view: EditorView, hooks: LangHooks, pos: number, definition: DefinitionReply, count: number, open: (view: EditorView) => boolean): HTMLElement {
  const word = view.state.wordAt(pos);
  const dom = el('div', 'cm-lang-card cm-lang-hover');
  const name = word === null ? '' : view.state.sliceDoc(word.from, word.to);
  dom.append(signatureLine(definition.kind, name, definition));
  if (definition.value !== undefined) {
    dom.append(valueLine(definition.value, definition.decoded === true));
  }
  const actions = el('div', 'cm-lang-actions');
  actions.append(
    button('Go to definition', 'F12', () => void goToDefinition(view, hooks, pos)),
    button(`${count} ${count === 1 ? 'reference' : 'references'}`, '', () => void showReferences(view, hooks.client, pos)),
    button('Rename symbol', '', () => {
      view.dispatch({ selection: { anchor: pos } });
      open(view);
    }),
  );
  dom.append(el('div', 'cm-lang-where', whereLabel(view, definition)), actions);
  return dom;
}

async function builtinAt(view: EditorView, hooks: LangHooks, pos: number, word: { from: number; to: number }): Promise<HTMLElement | null> {
  const member = memberNameAt(view.state, pos);
  if (member !== null) {
    const described = member.subject.name === null ? null : await describeAt(hooks.client, member.subject.from, member.subject.name);
    const owner = ownerOf(member.subject, described);
    const builtin = memberBuiltin(owner, member.property);
    return builtin === null ? null : builtinCard(builtin, owner.global ? owner.name : '');
  }
  const builtin = globals().get(view.state.sliceDoc(word.from, word.to));
  return builtin === undefined ? null : builtinCard(builtin, '');
}

function decodedAt(view: EditorView, pos: number): { from: number; to: number; dom: HTMLElement } | null {
  const string = stringAt(view.state, pos);
  if (string === null || !hasEscapes(string.literal)) {
    return null;
  }
  const value = decodeJsString(string.literal);
  return value === null ? null : { from: string.from, to: string.to, dom: stringCard(value) };
}

export function hovering(hooks: LangHooks, open: (view: EditorView) => boolean): Extension {
  return hoverTooltip(async (view, pos) => {
    const decoded = decodedAt(view, pos);
    if (decoded !== null) {
      return { pos: decoded.from, end: decoded.to, above: false, create: () => ({ dom: decoded.dom }) };
    }
    const word = view.state.wordAt(pos);
    if (word === null) {
      return null;
    }
    const definition = await definitionAt(hooks.client, pos);
    if (definition === null) {
      const dom = await builtinAt(view, hooks, pos, word);
      return dom === null ? null : { pos: word.from, end: word.to, above: false, create: () => ({ dom }) };
    }
    const count = (await referencesAt(hooks.client, pos)).length;
    return { pos: word.from, end: word.to, above: false, create: () => ({ dom: cardDom(view, hooks, pos, definition, count, open) }) };
  }, { hoverTime: 350 });
}
