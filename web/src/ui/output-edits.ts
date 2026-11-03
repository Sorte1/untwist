import { ChangeSet, type Extension, type Text } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { byId, linesLabel, setText } from '../dom';
import { replaceDoc } from './editor';

export function editedLines(changes: ChangeSet, doc: Text): number {
  let count = 0;
  let last = 0;
  changes.iterChangedRanges((_fromA, _toA, fromB, toB) => {
    const first = Math.max(doc.lineAt(fromB).number, last + 1);
    const end = doc.lineAt(toB).number;
    if (end >= first) {
      count += end - first + 1;
      last = end;
    }
  });
  return count;
}

export function editMark(changes: ChangeSet, doc: Text): string {
  const lines = changes.empty ? 0 : editedLines(changes, doc);
  return lines === 0 ? '' : `Edited · ${linesLabel(lines)}`;
}

export interface OutputEdits {
  readonly extension: Extension;
  replace(view: EditorView, text: string): void;
}

export function outputEdits(): OutputEdits {
  const mark = byId('output-edit-mark');
  const actions = byId('output-edit-actions');
  let changes = ChangeSet.empty(0);
  let replacing = false;
  const paint = (doc: Text) => {
    const text = editMark(changes, doc);
    setText(mark, text);
    mark.hidden = text === '';
    actions.hidden = text === '';
  };
  return {
    extension: EditorView.updateListener.of((update) => {
      if (update.docChanged && !replacing) {
        changes = changes.compose(update.changes);
        paint(update.state.doc);
      }
    }),
    replace(view, text) {
      replacing = true;
      try {
        replaceDoc(view, text, false);
      } finally {
        replacing = false;
      }
      changes = ChangeSet.empty(view.state.doc.length);
      paint(view.state.doc);
    },
  };
}
