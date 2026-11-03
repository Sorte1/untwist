import { EditorSelection } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { byId, setText } from '../dom';
import { seconds, type AppState, type InputState, type RunState } from '../state';
import type { Store } from '../store';
import { virtualList } from './details/virtual-list';
import { showInput } from './focus-mode';
import {
  diagnosticsOf,
  docPosition,
  levelLabel,
  lineColumn,
  lineIndex,
  shownMessage,
  stripLabel,
  warningCount,
  type DiagnosticRow,
  type LineIndex,
} from './diagnostics-model';

const ROW_HEIGHT = 27;
const VISIBLE_ROWS = 8;

interface Shown {
  readonly row: DiagnosticRow;
  readonly location: string;
  readonly clickable: boolean;
}

function noteFor(run: RunState, engineError: string | null): string | null {
  if (engineError !== null) {
    return `The engine failed to load: ${engineError}`;
  }
  switch (run.status) {
    case 'idle':
      return 'Paste some code and press Run.';
    case 'running':
      return 'Waiting for the worker. Diagnostics arrive with the result.';
    case 'cancelled':
      return `Worker terminated after ${seconds(run.elapsedMs)}. No partial output was kept; the last result is still shown above.`;
    case 'failed':
      return `The run failed: ${run.error ?? 'unknown error'}`;
    default:
      return null;
  }
}

function cell(className: string, text: string): HTMLElement {
  const element = document.createElement('span');
  element.className = className;
  element.textContent = text;
  return element;
}

function noteElement(text: string): HTMLElement {
  const note = document.createElement('div');
  note.className = 'diag-note';
  note.textContent = text;
  return note;
}

function settingsRow(problem: string): HTMLElement {
  const element = drawRow({ row: { level: 'info', pass: 'settings', message: problem }, location: '', clickable: false }, () => undefined);
  element.classList.add('diag-info');
  return element;
}

function drawRow(shown: Shown, reveal: (row: DiagnosticRow) => void): HTMLElement {
  const { row } = shown;
  const element = document.createElement(shown.clickable ? 'button' : 'div');
  element.className = 'diag-row';
  element.dataset['level'] = row.level;
  element.title = row.message;
  element.append(
    cell('diag-lvl', levelLabel(row.level)),
    cell('diag-src', row.pass),
    cell('diag-text', shownMessage(row)),
    cell('diag-loc', shown.location),
  );
  if (element instanceof HTMLButtonElement) {
    element.type = 'button';
    element.addEventListener('click', () => reveal(row));
  }
  return element;
}

function shownRows(rows: readonly DiagnosticRow[], ran: InputState | null, current: InputState): Shown[] {
  let index: LineIndex | null = null;
  const clickable = ran !== null && ran.revision === current.revision && !ran.large;
  return rows.map((row) => {
    if (row.inputSpan === undefined || ran === null) {
      return { row, location: '', clickable: false };
    }
    index ??= lineIndex(ran.text);
    const { line, column } = lineColumn(index, row.inputSpan[0]);
    return { row, location: `input ${line}:${column}`, clickable };
  });
}

export function revealIn(view: EditorView, text: string, span: readonly [number, number]): void {
  const length = view.state.doc.length;
  const from = Math.min(docPosition(text, span[0]), length);
  const to = Math.min(Math.max(docPosition(text, span[1]), from === length ? from : from + 1), length);
  view.dispatch({
    selection: EditorSelection.single(from, to),
    effects: EditorView.scrollIntoView(from, { y: 'center' }),
  });
  view.focus();
}

export function mountDiagnostics(store: Store<AppState>, input: EditorView): void {
  const holder = byId('diag-list');
  const count = byId('warn-count');
  const strip = byId('diag-count');
  let ran: InputState | null = null;
  const reveal = (row: DiagnosticRow) => {
    const span = row.inputSpan;
    const text = ran?.text;
    if (span === undefined || text === undefined) {
      return;
    }
    if (store.get().settings.input === 'hidden') {
      showInput(store);
      requestAnimationFrame(() => revealIn(input, text, span));
      return;
    }
    revealIn(input, text, span);
  };
  const list = virtualList<Shown>(ROW_HEIGHT, VISIBLE_ROWS, (shown) => drawRow(shown, reveal));
  list.element.classList.add('diag-rows');
  let first = true;
  store.subscribe((state, previous) => {
    const run = state.run;
    if (run.status === 'running' && previous.run.status !== 'running') {
      ran = state.input;
    }
    const changed = run.status !== previous.run.status || run.result !== previous.run.result;
    const moved = state.input.revision !== previous.input.revision;
    const settings = state.settingsProblems !== previous.settingsProblems;
    if (!first && !changed && !moved && !settings && state.engineError === previous.engineError) {
      return;
    }
    first = false;
    const note = noteFor(run, state.engineError);
    const rows = note === null && run.result !== null ? diagnosticsOf(run.result.report, run.result.stderr) : [];
    const finished = note === null && run.result !== null;
    setText(count, finished ? String(warningCount(rows)) : '—');
    const label = stripLabel(finished ? rows : null, state.settingsProblems.length);
    setText(strip, label.text);
    strip.dataset['tone'] = label.tone;
    const problems = state.settingsProblems.map(settingsRow);
    if (note !== null || rows.length === 0) {
      holder.replaceChildren(...problems, noteElement(note ?? 'No diagnostics.'));
      return;
    }
    holder.replaceChildren(...problems, list.element);
    list.setRows(shownRows(rows, ran, state.input));
  });
}
