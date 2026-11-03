import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { EditorView, showTooltip, type Tooltip } from '@codemirror/view';

export const NOTICE_MS = 3200;

export type Tone = 'info' | 'error';

interface Notice {
  readonly text: string;
  readonly tone: Tone;
  readonly pos: number;
}

const setNotice = StateEffect.define<Notice | null>();
const timers = new WeakMap<EditorView, ReturnType<typeof setTimeout>>();

function noticeDom(notice: Notice): HTMLElement {
  const dom = document.createElement('div');
  dom.className = 'cm-notice';
  dom.dataset['tone'] = notice.tone;
  dom.setAttribute('role', 'status');
  dom.textContent = notice.text;
  return dom;
}

function tooltipOf(notice: Notice): Tooltip {
  return { pos: notice.pos, above: true, strictSide: false, create: () => ({ dom: noticeDom(notice) }) };
}

const noticeField = StateField.define<Tooltip | null>({
  create: () => null,
  update(value, tr) {
    let next = tr.docChanged || tr.selection !== undefined ? null : value;
    for (const effect of tr.effects) {
      if (effect.is(setNotice)) {
        next = effect.value === null ? null : tooltipOf(effect.value);
      }
    }
    return next;
  },
  provide: (field) => showTooltip.from(field),
});

export function clearNotice(): StateEffect<Notice | null> {
  return setNotice.of(null);
}

export function showNotice(view: EditorView, text: string, tone: Tone = 'info'): void {
  const previous = timers.get(view);
  if (previous !== undefined) {
    clearTimeout(previous);
  }
  view.dispatch({ effects: setNotice.of({ text, tone, pos: view.state.selection.main.head }) });
  timers.set(
    view,
    setTimeout(() => {
      timers.delete(view);
      view.dispatch({ effects: setNotice.of(null) });
    }, NOTICE_MS),
  );
}

const noticeLook = EditorView.theme({
  '.cm-tooltip:has(> .cm-notice)': { border: 'none', backgroundColor: 'transparent' },
  '.cm-notice': {
    maxWidth: '360px',
    padding: '6px 10px',
    borderRadius: '6px',
    border: '1px solid var(--btn-border)',
    backgroundColor: 'var(--panel)',
    color: 'var(--text)',
    boxShadow: '0 8px 24px var(--shadow)',
    fontFamily: 'var(--sans)',
    fontSize: '12px',
  },
  '.cm-notice[data-tone="error"]': { color: 'var(--err)' },
});

export function notices(): Extension {
  return [noticeField, noticeLook];
}
