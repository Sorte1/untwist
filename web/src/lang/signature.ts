import { Prec, StateEffect, StateField, type Extension } from '@codemirror/state';
import { EditorView, keymap, showTooltip, ViewPlugin, type Tooltip, type ViewUpdate } from '@codemirror/view';
import { globals, memberBuiltin } from './builtins';
import type { LangClient } from './client';
import { describeAt, ownerOf } from './complete';
import { callAt, type Call } from './context';

export interface Signature {
  readonly label: string;
  readonly params: readonly string[];
  readonly returns: string;
  readonly doc: string;
}

interface Shown {
  readonly call: Call;
  readonly signature: Signature;
}

const TRIGGERS = /[(,]/;
const setShown = StateEffect.define<Shown | null>();

export function activeParam(params: readonly string[], index: number): number {
  if (index < params.length) {
    return index;
  }
  return params.at(-1)?.startsWith('...') === true ? params.length - 1 : -1;
}

function sameCall(a: Call, b: Call): boolean {
  return a.callee === b.callee && a.open === b.open;
}

const shownField = StateField.define<Shown | null>({
  create: () => null,
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setShown)) {
        return effect.value;
      }
    }
    if (value === null || (!tr.docChanged && tr.selection === undefined)) {
      return value;
    }
    const call = callAt(tr.state, tr.state.selection.main.head);
    const moved = { ...value.call, open: tr.changes.mapPos(value.call.open) };
    return call !== null && sameCall(call, moved) ? { ...value, call } : null;
  },
  provide: (field) => showTooltip.compute([field], (state) => tooltipOf(state.field(field))),
});

function part(className: string, text: string): HTMLElement {
  const node = document.createElement('span');
  node.className = className;
  node.textContent = text;
  return node;
}

export function signatureDom(shown: Shown): HTMLElement {
  const { signature, call } = shown;
  const active = activeParam(signature.params, call.argIndex);
  const dom = document.createElement('div');
  dom.className = 'cm-lang-card cm-lang-signature';
  dom.setAttribute('role', 'tooltip');
  const line = document.createElement('div');
  line.className = 'cm-lang-sig';
  line.append(part('cm-lang-name', signature.label), document.createTextNode('('));
  signature.params.forEach((param, index) => {
    line.append(...(index > 0 ? [document.createTextNode(', ')] : []), part(index === active ? 'cm-lang-param cm-lang-active' : 'cm-lang-param', param));
  });
  line.append(document.createTextNode(')'));
  if (signature.returns !== '') {
    line.append(document.createTextNode(': '), part('cm-lang-kind', signature.returns));
  }
  dom.append(line);
  if (signature.doc !== '') {
    dom.append(part('cm-lang-doc', signature.doc));
  }
  return dom;
}

function tooltipOf(shown: Shown | null): Tooltip | null {
  if (shown === null) {
    return null;
  }
  return { pos: shown.call.open, above: true, strictSide: false, create: () => ({ dom: signatureDom(shown) }) };
}

async function resolveSignature(client: LangClient, call: Call): Promise<Signature | null> {
  if (call.subject === null) {
    const described = await describeAt(client, call.calleeFrom, call.callee);
    if (described?.params !== undefined && described.kind !== 'global') {
      return { label: call.callee, params: described.params, returns: '', doc: '' };
    }
    const builtin = globals().get(call.callee);
    if (builtin === undefined || builtin.params === null) {
      return null;
    }
    return { label: call.callee, params: builtin.params, returns: builtin.returns, doc: builtin.doc };
  }
  const { subject } = call;
  const described = subject.name === null ? null : await describeAt(client, subject.from, subject.name);
  const owner = ownerOf(subject, described);
  const builtin = memberBuiltin(owner, call.callee);
  if (builtin === null || builtin.params === null) {
    return null;
  }
  const label = owner.global ? `${owner.name}.${call.callee}` : call.callee;
  return { label, params: builtin.params, returns: builtin.returns, doc: builtin.doc };
}

class Hints {
  #ticket = 0;

  constructor(readonly client: LangClient) {}

  trigger(view: EditorView): boolean {
    const call = callAt(view.state, view.state.selection.main.head);
    if (call === null) {
      return false;
    }
    const shown = view.state.field(shownField);
    if (shown !== null && sameCall(shown.call, call)) {
      return true;
    }
    this.#ticket += 1;
    const ticket = this.#ticket;
    void resolveSignature(this.client, call).then((signature) => {
      const now = callAt(view.state, view.state.selection.main.head);
      if (ticket === this.#ticket && signature !== null && now !== null && sameCall(now, call)) {
        view.dispatch({ effects: setShown.of({ call: now, signature }) });
      }
    });
    return true;
  }

  dismiss(view: EditorView): boolean {
    this.#ticket += 1;
    if (view.state.field(shownField) === null) {
      return false;
    }
    view.dispatch({ effects: setShown.of(null) });
    return true;
  }
}

function typedTrigger(update: ViewUpdate): boolean {
  let typed = false;
  update.changes.iterChanges((_fromA, _toA, _fromB, _toB, inserted) => {
    typed ||= TRIGGERS.test(inserted.toString());
  });
  return typed && update.transactions.some((tr) => tr.isUserEvent('input'));
}

export function signatureHelp(client: LangClient): Extension {
  const hints = new Hints(client);
  return [
    shownField,
    ViewPlugin.define((view) => ({
      update(update: ViewUpdate) {
        if (update.docChanged && typedTrigger(update)) {
          queueMicrotask(() => hints.trigger(view));
        }
      },
    })),
    keymap.of([{ key: 'Mod-Shift-Space', run: (view) => (hints.trigger(view), true) }]),
    Prec.highest(keymap.of([{ key: 'Escape', run: (view) => hints.dismiss(view) }])),
  ];
}
