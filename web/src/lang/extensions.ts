import { autocompletion } from '@codemirror/autocomplete';
import { lintKeymap, setDiagnostics, type Diagnostic } from '@codemirror/lint';
import { Prec, type Extension } from '@codemirror/state';
import { EditorView, keymap, tooltips } from '@codemirror/view';
import type { LangClient, Problem } from './client';
import { completionSource } from './complete';
import { definitionLinks } from './links';
import { occurrences } from './occurrences';
import { parsed } from './parsed';
import { definitionAt, referencesAt, within, type DefinitionReply } from './queries';
import { referencesPanelExtension, showReferences } from './references';
import { signatureHelp } from './signature';

export { definitionAt, parsed, referencesAt, showReferences, within, type DefinitionReply };

export interface ModuleHost {
  open(module: string, name: string): Promise<boolean>;
}

export interface LangHooks {
  readonly client: LangClient;
  readonly host: ModuleHost | null;
}

export type RenameReply = { readonly edits: readonly (readonly [number, number, string])[] } | { readonly refused: string };

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
  if (module === undefined && from <= pos && pos <= to) {
    return (await showReferences(view, hooks.client, pos)) > 0;
  }
  view.dispatch({ selection: { anchor: from, head: to }, scrollIntoView: true });
  view.focus();
  return true;
}

export function applyProblems(view: EditorView, problems: readonly Problem[]): void {
  const diagnostics: Diagnostic[] = problems.map((problem) => {
    const { from, to } = within(view.state, problem.from, problem.to);
    return { from, to, severity: problem.severity, message: problem.message, source: 'oxc' };
  });
  view.dispatch(setDiagnostics(view.state, diagnostics));
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
    keymap.of(lintKeymap),
    signatureHelp(hooks.client),
    referencesPanelExtension(),
    occurrences(hooks.client),
    definitionLinks(hooks.client),
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
    tooltips({ position: 'fixed' }),
    autocompletion({ override: [completionSource(hooks.client)], icons: true }),
  ];
}
