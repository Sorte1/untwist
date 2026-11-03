import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { EditorView, keymap, showPanel, type Panel } from '@codemirror/view';
import { CLOSE_PATH, strokeIcon } from './icon';

export interface Shortcut {
  readonly action: string;
  readonly keys: string;
  readonly mac?: string;
}

export interface ShortcutGroup {
  readonly title: string;
  readonly shortcuts: readonly Shortcut[];
}

export const SHORTCUTS: readonly ShortcutGroup[] = [
  {
    title: 'Edit',
    shortcuts: [
      { action: 'Run', keys: 'Mod-Enter' },
      { action: 'Format document', keys: 'Shift-Alt-f' },
      { action: 'Toggle comment', keys: 'Mod-/' },
      { action: 'Move line up or down', keys: 'Alt-↑|↓' },
      { action: 'Copy line up or down', keys: 'Shift-Alt-↑|↓' },
      { action: 'Delete line', keys: 'Shift-Mod-k' },
      { action: 'Indent or outdent', keys: 'Mod-]|[' },
      { action: 'Undo or redo', keys: 'Mod-z|y', mac: 'Mod-z|Shift-Mod-z' },
    ],
  },
  {
    title: 'Select',
    shortcuts: [
      { action: 'Add a cursor', keys: 'Alt-click' },
      { action: 'Add a cursor above or below', keys: 'Mod-Alt-↑|↓' },
      { action: 'Column selection', keys: 'Shift-Alt-drag' },
      { action: 'Select the next match', keys: 'Mod-d' },
      { action: 'Select every match', keys: 'Shift-Mod-l' },
      { action: 'Select the line', keys: 'Alt-l', mac: 'Ctrl-l' },
      { action: 'Grow the selection', keys: 'Mod-i' },
    ],
  },
  {
    title: 'Find and fold',
    shortcuts: [
      { action: 'Find and replace', keys: 'Mod-f' },
      { action: 'Next or previous match', keys: 'F3|Shift-F3' },
      { action: 'Go to line', keys: 'Mod-Alt-g' },
      { action: 'Matching bracket', keys: 'Shift-Mod-\\' },
      { action: 'Fold or unfold', keys: 'Ctrl-Shift-[|]', mac: 'Mod-Alt-[|]' },
      { action: 'Fold or unfold all', keys: 'Ctrl-Alt-[|]' },
      { action: 'Keyboard shortcuts', keys: 'F1' },
    ],
  },
];

const MAC_NAMES: Readonly<Record<string, string>> = { Mod: '⌘', Ctrl: '⌃', Alt: '⌥', Shift: '⇧', Enter: '↵' };
const NAMES: Readonly<Record<string, string>> = { Mod: 'Ctrl', Enter: '↵' };

function keyName(part: string, mac: boolean): string {
  const name = (mac ? MAC_NAMES : NAMES)[part] ?? part;
  return name.length === 1 ? name.toUpperCase() : name;
}

export function keyLabel(keys: string, mac: boolean): string[][] {
  return keys.split('|').map((chord, index, chords) => {
    const parts = chord.split(/-(?!$)/);
    const previous = index === 0 ? [] : (chords[0] ?? '').split(/-(?!$)/);
    const full = parts.length === 1 && previous.length > 1 ? [...previous.slice(0, -1), ...parts] : parts;
    return full.map((part) => keyName(part, mac));
  });
}

export function isMac(platform: string): boolean {
  return /Mac|iPhone|iPad/.test(platform);
}

const toggleSheet = StateEffect.define<boolean>();
export function shownChords(chords: readonly (readonly string[])[]): string[][] {
  const first = chords[0] ?? [];
  return chords.map((chord, index) => {
    if (index === 0) {
      return [...chord];
    }
    let shared = 0;
    while (shared < chord.length - 1 && chord[shared] === first[shared]) {
      shared += 1;
    }
    return chord.slice(shared);
  });
}

function kbd(part: string): HTMLElement {
  const element = document.createElement('kbd');
  element.textContent = part;
  return element;
}

function kbdRow(shortcut: Shortcut, mac: boolean): HTMLTableRowElement {
  const row = document.createElement('tr');
  const action = document.createElement('th');
  action.scope = 'row';
  action.textContent = shortcut.action;
  const keys = document.createElement('td');
  shownChords(keyLabel(mac ? (shortcut.mac ?? shortcut.keys) : shortcut.keys, mac)).forEach((chord, index) => {
    if (index > 0) {
      keys.append(document.createTextNode(' / '));
    }
    chord.forEach((part, at) => {
      keys.append(...(at > 0 ? [document.createTextNode(' ')] : []), kbd(part));
    });
  });
  row.append(action, keys);
  return row;
}

function groupTable(group: ShortcutGroup, mac: boolean): HTMLTableElement {
  const table = document.createElement('table');
  const caption = document.createElement('caption');
  caption.textContent = group.title;
  const body = document.createElement('tbody');
  body.append(...group.shortcuts.map((shortcut) => kbdRow(shortcut, mac)));
  table.append(caption, body);
  return table;
}

function sheetPanel(view: EditorView): Panel {
  const mac = isMac(navigator.platform);
  const dom = document.createElement('div');
  dom.className = 'cm-shortcuts';
  dom.setAttribute('role', 'region');
  dom.setAttribute('aria-label', 'Keyboard shortcuts');
  const head = document.createElement('div');
  head.className = 'cm-shortcuts-head';
  const title = document.createElement('span');
  title.textContent = 'Keyboard shortcuts';
  const close = document.createElement('button');
  close.type = 'button';
  close.className = 'cm-shortcuts-close';
  close.setAttribute('aria-label', 'Close the shortcuts');
  close.append(strokeIcon(CLOSE_PATH, 14));
  close.addEventListener('mousedown', (event) => event.preventDefault());
  close.addEventListener('click', () => {
    view.dispatch({ effects: toggleSheet.of(false) });
    view.focus();
  });
  head.append(title, close);
  const groups = document.createElement('div');
  groups.className = 'cm-shortcuts-groups';
  groups.append(...SHORTCUTS.map((group) => groupTable(group, mac)));
  dom.append(head, groups);
  return { dom };
}

const sheetField = StateField.define<boolean>({
  create: () => false,
  update(open, tr) {
    return tr.effects.reduce((value, effect) => (effect.is(toggleSheet) ? effect.value : value), open);
  },
  provide: (field) => showPanel.from(field, (open) => (open ? sheetPanel : null)),
});

function setSheet(view: EditorView, open: boolean): boolean {
  if (view.state.field(sheetField) === open) {
    return false;
  }
  view.dispatch({ effects: toggleSheet.of(open) });
  return true;
}

const sheetLook = EditorView.theme({
  '.cm-shortcuts': {
    maxHeight: '45vh',
    overflow: 'auto',
    padding: '8px 14px 12px',
    borderTop: '1px solid var(--hair)',
    fontFamily: 'var(--sans)',
    fontSize: '12px',
    color: 'var(--text)',
  },
  '.cm-shortcuts-head': { display: 'flex', alignItems: 'center', justifyContent: 'space-between', color: 'var(--muted)', marginBottom: '4px' },
  '.cm-shortcuts-close': {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '24px',
    height: '24px',
    padding: '0',
    border: 'none',
    borderRadius: '5px',
    backgroundColor: 'transparent',
    color: 'var(--muted)',
    cursor: 'pointer',
  },
  '.cm-shortcuts-close:hover': { backgroundColor: 'var(--sel)', color: 'var(--text)' },
  '.cm-shortcuts-groups': { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(270px, 1fr))', gap: '6px 28px' },
  '.cm-shortcuts table': { width: '100%', borderCollapse: 'collapse' },
  '.cm-shortcuts caption': { textAlign: 'left', padding: '6px 0 2px', color: 'var(--faint)', fontSize: '11px', fontWeight: '500' },
  '.cm-shortcuts th': { textAlign: 'left', fontWeight: '400', padding: '3px 12px 3px 0' },
  '.cm-shortcuts td': { textAlign: 'right', whiteSpace: 'nowrap', color: 'var(--faint)', fontFamily: 'var(--mono)', fontSize: '11px' },
  '.cm-shortcuts kbd': {
    display: 'inline-block',
    minWidth: '10px',
    padding: '1px 5px',
    borderRadius: '4px',
    backgroundColor: 'var(--sel)',
    color: 'var(--text)',
    fontFamily: 'var(--mono)',
    fontSize: '11px',
    fontWeight: '500',
    textAlign: 'center',
  },
});

export function shortcutSheet(): Extension {
  return [
    sheetField,
    sheetLook,
    keymap.of([
      { key: 'F1', run: (view) => setSheet(view, !view.state.field(sheetField)) || true, preventDefault: true },
      { key: 'Escape', run: (view) => setSheet(view, false) },
    ]),
  ];
}
