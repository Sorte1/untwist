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

function paintBrowserChrome(): void {
  const background = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
  if (background === '') {
    return;
  }
  for (const meta of document.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) {
    meta.content = background;
  }
}

export function applyEditorTheme(theme: Theme, picks: EditorThemePicks): void {
  const id = activeEditorTheme(theme, picks);
  document.documentElement.dataset['theme'] = theme;
  if (id === BUILT_IN) {
    delete document.documentElement.dataset['editor'];
  } else {
    document.documentElement.dataset['editor'] = id;
  }
  paintBrowserChrome();
}
