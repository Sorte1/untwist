import { autocompletion, type Completion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
import { setDiagnostics, type Diagnostic } from '@codemirror/lint';
import { Prec, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, keymap, tooltips, type DecorationSet } from '@codemirror/view';
import type { LangClient, Problem } from './client';

export interface ModuleHost {
  open(module: string, name: string): Promise<boolean>;
}

export interface LangHooks {
  readonly client: LangClient;
  readonly host: ModuleHost | null;
}

export interface DefinitionReply {
  readonly span: readonly [number, number];
  readonly kind: string;
  readonly module?: string;
  readonly name?: string;
}

interface CompletionItem {
  readonly label: string;
  readonly kind: string;
}

export type RenameReply = { readonly edits: readonly (readonly [number, number, string])[] } | { readonly refused: string };

export function parsed<T>(value: unknown): T | null {
  if (typeof value !== 'string') {
    return null;
  }
  try {
    return JSON.parse(value) as T;
  } catch {
    return null;
  }
}

export async function definitionAt(client: LangClient, pos: number): Promise<DefinitionReply | null> {
  return parsed<DefinitionReply>(await client.query('definition', pos));
}

export async function referencesAt(client: LangClient, pos: number): Promise<{ from: number; to: number }[]> {
  const value = await client.query('references', pos);
  if (!(value instanceof Uint32Array)) {
    return [];
  }
  const ranges: { from: number; to: number }[] = [];
  for (let index = 0; index + 1 < value.length; index += 2) {
    ranges.push({ from: value[index] ?? 0, to: value[index + 1] ?? 0 });
  }
  return ranges;
}

export function within(state: EditorState, from: number, to: number): { from: number; to: number } {
  const length = state.doc.length;
  const start = Math.min(Math.max(from, 0), length);
  return { from: start, to: Math.min(Math.max(to, start), length) };
}

export async function goToDefinition(view: EditorView, hooks: LangHooks, pos: number): Promise<boolean> {
  const definition = await definitionAt(hooks.client, pos);
  if (definition === null) {
    return false;
  }
  const { module, name } = definition;
  if (module !== undefined && name !== undefined && hooks.host !== null && (await hooks.host.open(module, name))) {
    return true;
  }
  const { from, to } = within(view.state, definition.span[0], definition.span[1]);
  view.dispatch({ selection: { anchor: from, head: to }, scrollIntoView: true });
  view.focus();
  return true;
}

const setReferences = StateEffect.define<readonly { from: number; to: number }[]>();
const referenceMark = Decoration.mark({ class: 'cm-lang-reference' });

const referenceField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(marks, tr) {
    let next = tr.docChanged ? Decoration.none : marks;
    for (const effect of tr.effects) {
      if (effect.is(setReferences)) {
        const ranges = effect.value.map(({ from, to }) => within(tr.state, from, to)).filter(({ from, to }) => to > from);
        next = Decoration.set(ranges.map(({ from, to }) => referenceMark.range(from, to)), true);
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

export async function showReferences(view: EditorView, client: LangClient, pos: number): Promise<number> {
  const ranges = await referencesAt(client, pos);
  view.dispatch({ effects: setReferences.of(ranges) });
  return ranges.length;
}

export function applyProblems(view: EditorView, problems: readonly Problem[]): void {
  const diagnostics: Diagnostic[] = problems.map((problem) => {
    const { from, to } = within(view.state, problem.from, problem.to);
    return { from, to, severity: problem.severity, message: problem.message, source: 'oxc' };
  });
  view.dispatch(setDiagnostics(view.state, diagnostics));
}

function typeOf(kind: string): string {
  switch (kind) {
    case 'function':
    case 'class':
    case 'keyword':
      return kind;
    default:
      return 'variable';
  }
}

function completionSource(client: LangClient) {
  return async (context: CompletionContext): Promise<CompletionResult | null> => {
    const word = context.matchBefore(/[\w$]*/);
    if (word === null || (word.from === word.to && !context.explicit)) {
      return null;
    }
    const items = parsed<CompletionItem[]>(await client.query('completions', context.pos));
    if (items === null || context.aborted) {
      return null;
    }
    const options: Completion[] = items.map((item, index) => ({ label: item.label, type: typeOf(item.kind), detail: item.kind, boost: Math.max(-99, 99 - index) }));
    return { from: word.from, options, validFor: /^[\w$]*$/ };
  };
}

export function navigation(hooks: LangHooks, rename: (view: EditorView) => boolean): Extension {
  const at = (view: EditorView) => view.state.selection.main.head;
  return [
    Prec.high(
      keymap.of([
        { key: 'F12', run: (view) => (void goToDefinition(view, hooks, at(view)), true) },
        { key: 'Shift-F12', run: (view) => (void showReferences(view, hooks.client, at(view)), true) },
        { key: 'F2', run: rename },
      ]),
    ),
    EditorView.domEventHandlers({
      mousedown(event, view) {
        if (!(event.ctrlKey || event.metaKey) || event.button !== 0) {
          return false;
        }
        const pos = view.posAtCoords(event);
        if (pos === null) {
          return false;
        }
        event.preventDefault();
        void goToDefinition(view, hooks, pos);
        return true;
      },
    }),
    referenceField,
    tooltips({ position: 'fixed' }),
    autocompletion({ override: [completionSource(hooks.client)], icons: true }),
  ];
}
