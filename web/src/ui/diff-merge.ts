import { Change, diff, MergeView } from '@codemirror/merge';
import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { baseExtensions } from './editor';

const look = EditorView.theme({
  '&.cm-merge-a .cm-changedLine, .cm-deletedChunk': { backgroundColor: 'var(--diff-del-tint)' },
  '&.cm-merge-b .cm-changedLine, .cm-insertedLine': { backgroundColor: 'var(--diff-add-tint)' },
  '&.cm-merge-a .cm-changedText': { background: 'none', textDecoration: 'underline 2px var(--del-mark)' },
  '&.cm-merge-b .cm-changedText': { background: 'none', textDecoration: 'underline 2px var(--add-mark)' },
  '&.cm-merge-a .cm-changeGutter': { borderLeft: '2px solid var(--del-mark)' },
  '&.cm-merge-b .cm-changeGutter': { borderLeft: '2px solid var(--add-mark)' },
});

const SIDES = '.cm-mergeViewEditor { min-width: 0; }';

function lines(wrap: boolean): Extension {
  return wrap ? EditorView.lineWrapping : [];
}

export interface DiffHandle {
  readonly chunks: number;
  destroy(): void;
}

const REFINE_CHARS = 2_000;
const REFINE_BUDGET_CHARS = 1_000_000;
const REFINE_SCAN_LIMIT = 500;

export function refinedChanges(before: string, after: string, ranges: Int32Array): Change[] {
  const changes: Change[] = [];
  let budget = REFINE_BUDGET_CHARS;
  for (let at = 0; at < ranges.length; at += 4) {
    const [fromA, toA, fromB, toB] = [ranges[at] as number, ranges[at + 1] as number, ranges[at + 2] as number, ranges[at + 3] as number];
    const size = toA - fromA + toB - fromB;
    if (toA - fromA > REFINE_CHARS || toB - fromB > REFINE_CHARS || size > budget || fromA === toA || fromB === toB) {
      changes.push(new Change(fromA, toA, fromB, toB));
      continue;
    }
    budget -= size;
    for (const inner of diff(before.slice(fromA, toA), after.slice(fromB, toB), { scanLimit: REFINE_SCAN_LIMIT })) {
      changes.push(new Change(fromA + inner.fromA, fromA + inner.toA, fromB + inner.fromB, fromB + inner.toB));
    }
  }
  return changes;
}

export function mountCode(parent: HTMLElement, text: string, wrap: boolean): DiffHandle {
  const host = document.createElement('div');
  host.className = wrap ? 'editor-host diff-host diff-wrap' : 'editor-host diff-host';
  parent.append(host);
  const root = host.attachShadow({ mode: 'open' });
  const view = new EditorView({ doc: text, extensions: [baseExtensions(false), lines(wrap)], parent: root, root });
  return {
    chunks: 0,
    destroy: () => {
      view.destroy();
      host.remove();
    },
  };
}

export function mountDiff(parent: HTMLElement, before: string, after: string, wrap: boolean, ranges?: Int32Array): DiffHandle {
  const host = document.createElement('div');
  host.className = wrap ? 'editor-host diff-host diff-wrap' : 'editor-host diff-host';
  parent.append(host);
  const root = host.attachShadow({ mode: 'open' });
  const sides = new CSSStyleSheet();
  sides.replaceSync(SIDES);
  root.adoptedStyleSheets = [...root.adoptedStyleSheets, sides];
  const view = new MergeView({
    a: { doc: before, extensions: [baseExtensions(false), look, lines(wrap)] },
    b: { doc: after, extensions: [baseExtensions(false), look, lines(wrap)] },
    parent: root,
    root,
    gutter: true,
    highlightChanges: true,
    collapseUnchanged: { margin: 3, minSize: 8 },
    ...(ranges === undefined ? {} : { diffConfig: { override: () => refinedChanges(before, after, ranges) } }),
  });
  view.dom.style.height = '100%';
  view.dom.style.width = '100%';
  return {
    chunks: view.chunks.length,
    destroy: () => {
      view.destroy();
      host.remove();
    },
  };
}
