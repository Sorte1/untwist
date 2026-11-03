import { bracketMatching, codeFolding, foldGutter, foldKeymap, indentOnInput } from '@codemirror/language';
import { search } from '@codemirror/search';
import { Compartment, EditorState, type Extension } from '@codemirror/state';
import { EditorView, keymap, rectangularSelection } from '@codemirror/view';
import { CHEVRON_PATH, CLOSE_PATH, maskOf, strokeIcon } from './icon';
import { indentGuides } from './indent-guides';

const CLOSE_MASK = maskOf(CLOSE_PATH);
const PHRASES = {
  Find: 'Find',
  Replace: 'Replace',
  next: 'Next',
  previous: 'Previous',
  all: 'Select all',
  'match case': 'Match case',
  regexp: 'Regexp',
  'by word': 'By word',
  replace: 'Replace',
  'replace all': 'Replace all',
  close: 'Close',
  'Fold line': 'Fold',
  'Unfold line': 'Unfold',
};

export function foldMarker(open: boolean): HTMLElement {
  const marker = document.createElement('span');
  marker.className = 'cm-fold-marker';
  marker.dataset['open'] = String(open);
  marker.title = open ? 'Fold' : 'Unfold';
  marker.append(strokeIcon(CHEVRON_PATH, 10, 1.8));
  return marker;
}

const editingLook = EditorView.theme({
  '.cm-foldGutter .cm-gutterElement': { display: 'flex', alignItems: 'center', justifyContent: 'center', width: '14px', paddingRight: '4px' },
  '.cm-fold-marker': { display: 'inline-flex', color: 'var(--faint)', cursor: 'pointer', opacity: '0', transition: 'opacity 120ms' },
  '.cm-fold-marker[data-open="false"]': { opacity: '1', transform: 'rotate(-90deg)', color: 'var(--accent)' },
  '.cm-gutters:hover .cm-fold-marker': { opacity: '1' },
  '.cm-fold-marker:hover': { color: 'var(--text)' },
  '@media (hover: none)': { '.cm-fold-marker': { opacity: '1' } },
  '.cm-foldPlaceholder': {
    margin: '0 3px',
    padding: '0 6px',
    border: '1px solid var(--btn-border)',
    borderRadius: '4px',
    backgroundColor: 'var(--sel)',
    color: 'var(--muted)',
    cursor: 'pointer',
  },
  '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': {
    backgroundColor: 'var(--selection)',
    outline: '1px solid var(--faint)',
    borderRadius: '2px',
    color: 'inherit',
  },
  '.cm-nonmatchingBracket, &.cm-focused .cm-nonmatchingBracket': { backgroundColor: 'transparent', color: 'var(--err)', textDecoration: 'underline wavy var(--err)' },
  '.cm-searchMatch': { backgroundColor: 'color-mix(in srgb, var(--warn) 22%, transparent)', borderRadius: '2px' },
  '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: 'color-mix(in srgb, var(--accent) 32%, transparent)', outline: '1px solid var(--accent)' },
  '.cm-panels-top': { borderBottom: '1px solid var(--hair)' },
  '.cm-panels-bottom': { borderTop: 'none' },
  '.cm-panel.cm-search': {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '6px 8px',
    padding: '6px 34px 6px 12px',
    fontFamily: 'var(--sans)',
    fontSize: '12px',
    color: 'var(--muted)',
  },
  '.cm-panel.cm-search br': { flexBasis: '100%', height: '0' },
  '.cm-panel.cm-search label': { display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '12px', whiteSpace: 'nowrap' },
  '.cm-panel.cm-search input[type=checkbox]': { margin: '0', accentColor: 'var(--accent)' },
  '.cm-panel.cm-search [name=close]': { top: '6px', right: '8px', width: '24px', height: '24px', borderRadius: '5px', color: 'var(--muted)', fontSize: '0', cursor: 'pointer' },
  '.cm-panel.cm-search [name=close]::before': { content: '""', display: 'block', width: '14px', height: '14px', margin: 'auto', backgroundColor: 'currentColor', mask: `${CLOSE_MASK} center / 14px no-repeat` },
  '.cm-panel.cm-search [name=close]:hover': { backgroundColor: 'var(--sel)', color: 'var(--text)' },
  '.cm-textfield': {
    height: '24px',
    margin: '0',
    padding: '0 8px',
    boxSizing: 'border-box',
    borderRadius: '5px',
    border: '1px solid var(--btn-border)',
    backgroundColor: 'var(--code)',
    color: 'var(--text)',
    fontFamily: 'var(--mono)',
    fontSize: '12px',
    outline: 'none',
  },
  '.cm-textfield:focus': { borderColor: 'var(--accent)' },
  '.cm-button, .cm-button:active': {
    height: '24px',
    margin: '0',
    padding: '0 8px',
    borderRadius: '5px',
    border: '1px solid var(--btn-border)',
    backgroundColor: 'var(--btn)',
    backgroundImage: 'none',
    color: 'var(--text)',
    fontFamily: 'var(--sans)',
    fontSize: '12px',
    cursor: 'pointer',
  },
  '.cm-button:hover': { backgroundColor: 'var(--sel)' },
});

const IDLE_TIMEOUT_MS = 2000;

function whenIdle(run: () => void): void {
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(run, { timeout: IDLE_TIMEOUT_MS });
  } else {
    setTimeout(run, IDLE_TIMEOUT_MS / 10);
  }
}

export interface Editing {
  readonly extension: Extension;
  attach(view: EditorView, host: HTMLElement): void;
}

function foldPlaceholder(view: EditorView, unfold: (event: Event) => void): HTMLElement {
  const element = document.createElement('span');
  element.textContent = '…';
  element.className = 'cm-foldPlaceholder';
  element.setAttribute('aria-label', view.state.phrase('folded code'));
  element.title = view.state.phrase('unfold');
  element.addEventListener('mousedown', unfold);
  return element;
}

export function codeEditing(module: Promise<WebAssembly.Module>): Editing {
  const extras = new Compartment();
  return {
    extension: [
      EditorState.allowMultipleSelections.of(true),
      EditorState.phrases.of(PHRASES),
      EditorView.clickAddsSelectionRange.of((event) => event.altKey),
      rectangularSelection({ eventFilter: (event) => event.altKey && event.shiftKey }),
      codeFolding({ placeholderText: '…', placeholderDOM: foldPlaceholder }),
      foldGutter({ markerDOM: foldMarker }),
      bracketMatching(),
      indentOnInput(),
      indentGuides(),
      search({ top: true }),
      keymap.of(foldKeymap),
      editingLook,
      extras.of([]),
    ],
    attach(view, host) {
      let loading = false;
      const load = () => {
        if (!loading) {
          loading = true;
          import('./editing-extras').then(
            (loaded) => view.dispatch({ effects: extras.reconfigure(loaded.editingExtras(module)) }),
            () => {
              loading = false;
              host.addEventListener('focusin', load, { once: true });
            },
          );
        }
      };
      host.addEventListener('focusin', load, { once: true });
      whenIdle(load);
    },
  };
}
