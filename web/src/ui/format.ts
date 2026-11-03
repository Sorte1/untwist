import type { Text } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { Engine } from '../engine/engine';
import type { RunResult } from '../engine/types';
import { clearNotice, showNotice } from './notice';

export const FORMAT_FLAGS = '--no-unpack --no-deobfuscate --no-jsx --rename none';

export type Formatted = { readonly text: string } | { readonly refused: string };

export type Formatter = (view: EditorView) => boolean;

export function formatOutcome(result: Pick<RunResult, 'stdout' | 'stderr' | 'code' | 'trap'>): Formatted {
  if (result.trap !== null) {
    return { refused: 'the engine ran out of stack' };
  }
  if (result.code !== 0) {
    const reason = result.stderr.split('\n').find((line) => line.trim() !== '')?.trim();
    return { refused: reason ?? `the engine exited with ${result.code}` };
  }
  return { text: result.stdout };
}

export function keptLine(doc: Text, head: number, next: Text): number {
  const number = Math.min(doc.lineAt(head).number, next.lines);
  return next.line(number).from;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function apply(view: EditorView, before: Text, outcome: Formatted): void {
  if ('refused' in outcome) {
    showNotice(view, `Not formatted: ${outcome.refused}`, 'error');
    return;
  }
  if (view.state.doc !== before) {
    showNotice(view, 'Not formatted: the text changed while formatting', 'error');
    return;
  }
  if (outcome.text === before.toString()) {
    showNotice(view, 'Already formatted');
    return;
  }
  const head = view.state.selection.main.head;
  const next = view.state.toText(outcome.text);
  view.dispatch({
    changes: { from: 0, to: before.length, insert: next },
    selection: { anchor: keptLine(before, head, next) },
    scrollIntoView: true,
    userEvent: 'format',
    effects: clearNotice(),
  });
}

export function formatter(module: Promise<WebAssembly.Module>): Formatter {
  let engine: Engine | null = null;
  let busy = false;
  return (view) => {
    if (view.state.readOnly) {
      showNotice(view, 'Not formatted: this preview is read-only', 'error');
      return true;
    }
    if (busy) {
      return true;
    }
    busy = true;
    engine ??= new Engine(module, false);
    const before = view.state.doc;
    showNotice(view, 'Formatting…');
    engine
      .run(before.toString(), FORMAT_FLAGS)
      .then(
        (result) => apply(view, before, formatOutcome(result)),
        (error: unknown) => showNotice(view, `Not formatted: ${message(error)}`, 'error'),
      )
      .finally(() => {
        busy = false;
      });
    return true;
  };
}
