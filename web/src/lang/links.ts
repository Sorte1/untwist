import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { Decoration, EditorView, type DecorationSet } from '@codemirror/view';
import type { LangClient } from './client';
import { definitionAt, type Span } from './queries';

const setLink = StateEffect.define<Span | null>();
const linkMark = Decoration.mark({ class: 'cm-lang-link' });

const linkField = StateField.define<Span | null>({
  create: () => null,
  update(link, tr) {
    let next = tr.docChanged ? null : link;
    for (const effect of tr.effects) {
      if (effect.is(setLink)) {
        next = effect.value;
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field, (link): DecorationSet => (link === null ? Decoration.none : Decoration.set([linkMark.range(link.from, link.to)]))),
});

function modified(event: MouseEvent | KeyboardEvent): boolean {
  return event.ctrlKey || event.metaKey;
}

function clear(view: EditorView): boolean {
  if (view.state.field(linkField) !== null) {
    view.dispatch({ effects: setLink.of(null) });
  }
  return false;
}

export function definitionLinks(client: LangClient): Extension {
  let asked: Span | null = null;
  const hover = (event: MouseEvent, view: EditorView): boolean => {
    if (!modified(event)) {
      asked = null;
      return clear(view);
    }
    const pos = view.posAtCoords(event);
    const word = pos === null ? null : view.state.wordAt(pos);
    if (pos === null || word === null) {
      asked = null;
      return clear(view);
    }
    if (asked?.from === word.from && asked.to === word.to) {
      return false;
    }
    asked = { from: word.from, to: word.to };
    const doc = view.state.doc;
    void definitionAt(client, pos).then((definition) => {
      const still = asked?.from === word.from && view.state.doc === doc;
      if (still) {
        view.dispatch({ effects: setLink.of(definition === null ? null : { from: word.from, to: word.to }) });
      }
    });
    return false;
  };
  return [
    linkField,
    EditorView.domEventHandlers({
      mousemove: hover,
      mouseleave: (_event, view) => {
        asked = null;
        return clear(view);
      },
      keyup: (event, view) => {
        if (!modified(event)) {
          asked = null;
          clear(view);
        }
        return false;
      },
    }),
  ];
}
