import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from '@codemirror/view';
import type { LangClient } from './client';
import { referencesAt, within, type Span } from './queries';

export const OCCURRENCE_MS = 200;

const setOccurrences = StateEffect.define<readonly Span[]>();
const occurrenceMark = Decoration.mark({ class: 'cm-lang-occurrence' });

const occurrenceField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(marks, tr) {
    let next = tr.docChanged ? Decoration.none : marks;
    for (const effect of tr.effects) {
      if (effect.is(setOccurrences)) {
        const ranges = effect.value.map(({ from, to }) => within(tr.state, from, to)).filter(({ from, to }) => to > from);
        next = Decoration.set(ranges.map(({ from, to }) => occurrenceMark.range(from, to)), true);
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

export function cursorWord(view: EditorView): number | null {
  const { main, ranges } = view.state.selection;
  if (ranges.length !== 1 || !main.empty) {
    return null;
  }
  return view.state.wordAt(main.head) === null ? null : main.head;
}

class Occurrences {
  #timer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    readonly view: EditorView,
    readonly client: LangClient,
  ) {}

  update(update: ViewUpdate): void {
    if (!update.selectionSet || update.docChanged) {
      return;
    }
    this.#cancel();
    const pos = cursorWord(update.view);
    if (pos === null) {
      queueMicrotask(() => this.#clear());
      return;
    }
    this.#timer = setTimeout(() => void this.#mark(pos), OCCURRENCE_MS);
  }

  destroy(): void {
    this.#cancel();
  }

  async #mark(pos: number): Promise<void> {
    const doc = this.view.state.doc;
    const refs = await referencesAt(this.client, pos);
    if (this.view.state.doc === doc && this.view.state.selection.main.head === pos) {
      this.view.dispatch({ effects: setOccurrences.of(refs) });
    }
  }

  #clear(): void {
    if (this.view.state.field(occurrenceField).size > 0) {
      this.view.dispatch({ effects: setOccurrences.of([]) });
    }
  }

  #cancel(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
  }
}

export function occurrences(client: LangClient): Extension {
  return [occurrenceField, ViewPlugin.define((view) => new Occurrences(view, client))];
}
