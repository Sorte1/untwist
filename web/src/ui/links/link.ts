import { StateEffect, StateField, type Extension, type Text } from '@codemirror/state';
import { Decoration, EditorView, type DecorationSet, type ViewUpdate } from '@codemirror/view';
import type { RunResult } from '../../engine/types';
import type { AppState } from '../../state';
import type { Store } from '../../store';
import { docPosition } from '../diagnostics-model';
import { currentBundle } from '../module-tree';
import { shownText } from '../output-pane';
import { fromInput, fromOutput, moduleRegion, pairsOf, textOffset, type Pair, type Range } from './model';

const setLinked = StateEffect.define<readonly Range[]>();
const mark = Decoration.mark({ class: 'cm-linked' });

const linked = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(set, tr) {
    let next = set.map(tr.changes);
    for (const effect of tr.effects) {
      if (effect.is(setLinked)) {
        const ranges = effect.value.filter(([from, to]) => from < to && to <= tr.state.doc.length);
        next = Decoration.set(ranges.map(([from, to]) => mark.range(from, to)), true);
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

const look = EditorView.theme({
  '.cm-linked': {
    backgroundColor: 'color-mix(in srgb, var(--accent) 24%, transparent)',
    borderRadius: '2px',
  },
});

function offscreen(view: EditorView, at: number): boolean {
  const coords = view.coordsAtPos(Math.min(at, view.state.doc.length));
  const frame = view.scrollDOM.getBoundingClientRect();
  return coords === null || coords.top < frame.top || coords.bottom > frame.bottom;
}

function show(view: EditorView, ranges: readonly Range[], scroll: boolean): void {
  const first = ranges[0];
  const moved = scroll && first !== undefined && offscreen(view, first[0]);
  view.dispatch({
    effects: [setLinked.of(ranges), ...(moved ? [EditorView.scrollIntoView(first[0], { y: 'center' })] : [])],
  });
}

interface Sides {
  readonly input: EditorView;
  readonly output: EditorView;
}

function sameText(doc: Text, text: string, seen: WeakMap<Text, boolean>): boolean {
  const known = seen.get(doc);
  if (known !== undefined) {
    return known;
  }
  const same = doc.length === text.length && doc.toString() === text;
  seen.set(doc, same);
  return same;
}

function linker(store: Store<AppState>, sides: Sides) {
  let pairs: { readonly of: RunResult | null; readonly list: Pair[] } = { of: null, list: [] };
  const seen = new WeakMap<Text, boolean>();
  const pairsFor = (result: RunResult): Pair[] => {
    if (pairs.of !== result) {
      pairs = { of: result, list: pairsOf(result.links ?? '') };
    }
    return pairs.list;
  };
  const usable = () => {
    const state = store.get();
    const { result, outputInput: ran } = state.run;
    if (result === null || ran === null || ran.revision !== state.input.revision || ran.large || state.settings.view !== 'output') {
      return null;
    }
    if (!sameText(sides.output.state.doc, shownText(state).text, seen)) {
      return null;
    }
    return { state, result, text: ran.text, list: pairsFor(result) };
  };
  const inputRanges = (text: string, range: Range): Range => [docPosition(text, range[0]), docPosition(text, range[1])];
  const clear = () => {
    show(sides.input, [], false);
    show(sides.output, [], false);
  };
  const fromOutputAt = (at: number) => {
    const live = usable();
    if (live === null) {
      return clear();
    }
    const index = live.state.selectedModule;
    if (index !== null) {
      const span = currentBundle(live.state)?.modules[index]?.span;
      const region = span === undefined ? null : moduleRegion(live.list, span);
      show(sides.input, region === null ? [] : [inputRanges(live.text, region)], true);
      return;
    }
    const link = fromOutput(live.list, at);
    show(sides.output, link === null ? [] : [link.output], false);
    show(sides.input, link === null ? [] : [inputRanges(live.text, link.input)], true);
  };
  const fromInputAt = (at: number) => {
    const live = usable();
    if (live === null || live.state.selectedModule !== null) {
      return clear();
    }
    const link = fromInput(live.list, textOffset(live.text, at));
    show(sides.input, link === null ? [] : [inputRanges(live.text, link.input)], false);
    show(sides.output, link?.outputs ?? [], true);
  };
  return { fromOutputAt, fromInputAt, clear };
}

function listener(follow: (at: number) => void): Extension {
  return EditorView.updateListener.of((update: ViewUpdate) => {
    const picked = update.transactions.some((tr) => tr.isUserEvent('select'));
    const main = update.state.selection.main;
    if (update.selectionSet && picked && main.empty) {
      const at = main.head;
      queueMicrotask(() => follow(at));
    }
  });
}

export function attachLinks(store: Store<AppState>, input: EditorView, output: EditorView): void {
  const link = linker(store, { input, output });
  input.dispatch({ effects: StateEffect.appendConfig.of([linked, look, listener(link.fromInputAt)]) });
  output.dispatch({ effects: StateEffect.appendConfig.of([linked, look, listener(link.fromOutputAt)]) });
  store.subscribe((state, previous) => {
    if (state.run.result !== previous.run.result || state.selectedModule !== previous.selectedModule || state.input.revision !== previous.input.revision) {
      queueMicrotask(link.clear);
    }
  });
}
