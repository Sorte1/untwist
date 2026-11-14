import { browserStorage, type KeyValue } from '../recent/storage';
import type { Theme } from '../state';
import { activeEditorTheme, BUILT_IN, EDITOR_THEMES_KEY, parsePicks, type EditorThemePicks } from './themes';

export function loadPicks(store: KeyValue | null = browserStorage()): EditorThemePicks {
  try {
    return parsePicks(store?.getItem(EDITOR_THEMES_KEY) ?? null);
  } catch {
    return parsePicks(null);
  }
}

export function savePicks(picks: EditorThemePicks, store: KeyValue | null = browserStorage()): void {
  try {
    store?.setItem(EDITOR_THEMES_KEY, JSON.stringify(picks));
  } catch {
    return;
  }
}

export function applyEditorTheme(theme: Theme, picks: EditorThemePicks): void {
  const id = activeEditorTheme(theme, picks);
  if (id === BUILT_IN) {
    delete document.documentElement.dataset['editor'];
  } else {
    document.documentElement.dataset['editor'] = id;
  }
}
