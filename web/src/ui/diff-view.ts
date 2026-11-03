import { comparePlan } from '../compare';
import { byId, setText } from '../dom';
import { diffInWorker, type PendingDiff, type WorkerDiff } from '../diff/client';
import type { Hunk } from '../diff/lines';
import { utf8Bytes, lineStats, type AppState } from '../state';
import type { Store } from '../store';
import { virtualList } from './details/virtual-list';
import { currentBundle } from './module-tree';

export const DIFF_CAP_BYTES = 2 * 1024 * 1024;
export const DIFF_CAP_LINES = 50_000;
export const BIG_DIFF_CAP_BYTES = 16 * 1024 * 1024;
export const BIG_DIFF_HUNK_CAP = 20_000;
export const BIG_DIFF_LINE_CAP = 1_000_000;

const SUMMARY_ROW_HEIGHT = 26;
const SUMMARY_ROWS = 24;

interface Sides {
  readonly before: string;
  readonly after: string;
  readonly beforeName: string;
  readonly afterName: string;
  readonly note: string | null;
}

export type DiffPlan =
  | { readonly kind: 'off'; readonly note: string }
  | ({ readonly kind: 'diff' } & Sides)
  | ({ readonly kind: 'big' } & Sides);

export type BigView = { readonly kind: 'merge' } | { readonly kind: 'summary'; readonly note: string };

export function bigView(result: WorkerDiff): BigView {
  const hunks = result.ranges.length / 4;
  if (hunks > BIG_DIFF_HUNK_CAP) {
    return { kind: 'summary', note: `${hunks.toLocaleString('en-US')} changes are too many to show side by side; this is the list of changes.` };
  }
  if (result.longestLine > BIG_DIFF_LINE_CAP) {
    return { kind: 'summary', note: `A line over ${BIG_DIFF_LINE_CAP.toLocaleString('en-US')} characters is too long to show side by side; this is the list of changes.` };
  }
  return { kind: 'merge' };
}

function span(from: number, to: number): string {
  return to - from === 1 ? String(from + 1) : `${from + 1}–${to}`;
}

export function hunkLabel(hunk: Hunk): string {
  const removed = hunk.toA - hunk.fromA;
  const added = hunk.toB - hunk.fromB;
  const before = removed === 0 ? (hunk.fromA === 0 ? 'before line 1' : `after line ${hunk.fromA}`) : `${removed === 1 ? 'line' : 'lines'} ${span(hunk.fromA, hunk.toA)}`;
  const after = added === 0 ? 'removed' : span(hunk.fromB, hunk.toB);
  const counts = [removed > 0 ? `−${removed}` : '', added > 0 ? `+${added}` : ''].filter((part) => part !== '').join(' ');
  return `${before} → ${after}: ${counts}`;
}

function hunksOf(lineRanges: Int32Array): Hunk[] {
  const hunks: Hunk[] = [];
  for (let at = 0; at < lineRanges.length; at += 4) {
    hunks.push({ fromA: lineRanges[at] as number, toA: lineRanges[at + 1] as number, fromB: lineRanges[at + 2] as number, toB: lineRanges[at + 3] as number });
  }
  return hunks;
}

function mountSummary(parent: HTMLElement, result: WorkerDiff): { readonly chunks: number; destroy(): void } {
  const hunks = hunksOf(result.lineRanges);
  const removed = hunks.reduce((sum, hunk) => sum + hunk.toA - hunk.fromA, 0);
  const added = hunks.reduce((sum, hunk) => sum + hunk.toB - hunk.fromB, 0);
  const holder = document.createElement('div');
  holder.className = 'diff-summary';
  const head = document.createElement('div');
  head.className = 'diff-summary-head';
  head.textContent = `${hunks.length.toLocaleString('en-US')} ${hunks.length === 1 ? 'change' : 'changes'} · −${removed.toLocaleString('en-US')} +${added.toLocaleString('en-US')} lines`;
  const list = virtualList<Hunk>(SUMMARY_ROW_HEIGHT, SUMMARY_ROWS, (hunk) => {
    const row = document.createElement('div');
    row.className = 'diff-summary-row';
    row.textContent = hunkLabel(hunk);
    return row;
  });
  holder.append(head, list.element);
  parent.append(holder);
  list.setRows(hunks);
  return { chunks: hunks.length, destroy: () => holder.remove() };
}

type Merge = typeof import('./diff-merge');

function tooBig(text: string): boolean {
  return text.length > DIFF_CAP_BYTES || utf8Bytes(text) > DIFF_CAP_BYTES || lineStats(text).lines > DIFF_CAP_LINES;
}

function overWorkerCap(text: string): boolean {
  return text.length > BIG_DIFF_CAP_BYTES || utf8Bytes(text) > BIG_DIFF_CAP_BYTES;
}

function edited(name: string, engine: string, current: string | null): { readonly after: string; readonly afterName: string } {
  return current === null || current === engine ? { after: engine, afterName: name } : { after: current, afterName: `${name} (edited)` };
}

function sides(state: AppState, current: string | null): DiffPlan {
  const compared = comparePlan(state);
  if (compared !== null) {
    const note = compared.safe === compared.unsafe ? 'Safe and Unsafe give the same output.' : null;
    return { kind: 'diff', before: compared.safe, after: compared.unsafe, beforeName: 'Safe output', afterName: 'Unsafe output', note };
  }
  const result = state.run.result;
  if (result === null) {
    return { kind: 'off', note: 'Run something to compare its input and output.' };
  }
  const bundle = currentBundle(state);
  const index = state.selectedModule;
  if (bundle === null || index === null) {
    return { kind: 'diff', before: state.input.text, ...edited('output.js', result.stdout, current), beforeName: 'input', note: null };
  }
  const module = bundle.modules[index];
  const code = result.modules[index] ?? '';
  const name = module?.path ?? `module ${index}`;
  if (module?.span === undefined) {
    const note = 'This module has no span in the bundle text, so the left side is the whole processed bundle.';
    return { kind: 'diff', before: result.stdout, ...edited(name, code, current), beforeName: 'bundle', note };
  }
  const [from, to] = module.span;
  return { kind: 'diff', before: result.stdout.slice(from, to), ...edited(name, code, current), beforeName: 'bundle slice', note: null };
}

export function diffPlan(state: AppState, current: string | null = null): DiffPlan {
  const plan = sides(state, current);
  if (plan.kind !== 'diff' || (!tooBig(plan.before) && !tooBig(plan.after))) {
    return plan;
  }
  if (overWorkerCap(plan.before) || overWorkerCap(plan.after)) {
    return { kind: 'off', note: 'The diff is off when either side is over 16 MB.' };
  }
  return { ...plan, kind: 'big' };
}

function setView(store: Store<AppState>, view: 'output' | 'diff'): void {
  store.update((state) => ({ ...state, settings: { ...state.settings, view } }));
}

export function mountDiffView(store: Store<AppState>, current: () => string): void {
  const container = byId('diff');
  const note = byId('diff-note');
  let merge: Merge | null = null;
  let handle: { destroy(): void } | null = null;
  let shown: DiffPlan | null = null;
  let pending: PendingDiff | null = null;
  byId('view-output').addEventListener('click', () => setView(store, 'output'));
  byId('view-diff').addEventListener('click', () => setView(store, 'diff'));
  const showNote = (text: string | null) => {
    note.hidden = text === null;
    setText(note, text ?? '');
  };
  const clear = () => {
    pending?.cancel();
    pending = null;
    handle?.destroy();
    handle = null;
    container.dataset['chunks'] = '';
    container.dataset['view'] = '';
  };
  const showBig = (loaded: Merge, plan: Extract<DiffPlan, { kind: 'big' }>) => {
    showNote('Comparing in a worker…');
    const started = performance.now();
    const running = diffInWorker(plan.before, plan.after);
    pending = running;
    running.result.then(
      (result) => {
        if (shown !== plan) {
          return;
        }
        pending = null;
        const view = bigView(result);
        const mounted = view.kind === 'merge' ? loaded.mountDiff(container, plan.before, plan.after, result.ranges) : mountSummary(container, result);
        handle = mounted;
        container.dataset['chunks'] = String(mounted.chunks);
        container.dataset['view'] = view.kind;
        const inexact = result.exact ? null : 'Ranges with nothing in common are shown as one change.';
        showNote(view.kind === 'summary' ? view.note : plan.note ?? inexact);
        performance.mark('untwist:diff-shown', { detail: { workerMs: result.ms, totalMs: performance.now() - started, hunks: mounted.chunks, view: view.kind } });
      },
      (error: unknown) => {
        if (shown === plan) {
          showNote(`The diff failed: ${error instanceof Error ? error.message : String(error)}`);
        }
      },
    );
  };
  const paint = (state: AppState) => {
    clear();
    const plan = diffPlan(state, current());
    shown = plan;
    showNote(plan.note);
    if (plan.kind === 'off' || merge === null) {
      return;
    }
    setText(byId('diff-after'), plan.afterName);
    setText(byId('diff-before'), plan.beforeName);
    if (plan.kind === 'big') {
      showBig(merge, plan);
      return;
    }
    const mounted = merge.mountDiff(container, plan.before, plan.after);
    container.dataset['chunks'] = String(mounted.chunks);
    container.dataset['view'] = 'merge';
    handle = mounted;
  };
  store.subscribe((state, previous) => {
    const diff = state.settings.view === 'diff';
    document.body.classList.toggle('diff-mode', diff);
    for (const [id, on] of [['view-output', !diff], ['view-diff', diff]] as const) {
      byId(id).setAttribute('aria-pressed', String(on));
    }
    const changed =
      state.run.result !== previous.run.result ||
      state.selectedModule !== previous.selectedModule ||
      state.input !== previous.input ||
      state.compare !== previous.compare;
    if (!diff) {
      clear();
      shown = null;
      return;
    }
    if (shown !== null && !changed) {
      return;
    }
    if (merge === null) {
      shown = diffPlan(state, current());
      import('./diff-merge').then((module) => {
        merge = module;
        paint(store.get());
      }, () => setText(note, 'The diff view failed to load.'));
      return;
    }
    paint(state);
  });
}
