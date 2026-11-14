import type { Theme } from '../state';

export interface EditorTheme {
  readonly id: string;
  readonly label: string;
  readonly theme: Theme;
}

export interface EditorThemePicks {
  readonly dark: string;
  readonly light: string;
}

export const BUILT_IN = 'untwist';
export const EDITOR_THEMES_KEY = 'untwist.editorThemes';

export const EDITOR_THEMES: readonly EditorTheme[] = [
  { id: 'tokyo-night', label: 'Tokyo Night', theme: 'dark' },
  { id: 'github-dark', label: 'GitHub Dark', theme: 'dark' },
  { id: 'one-dark', label: 'One Dark', theme: 'dark' },
  { id: 'catppuccin-mocha', label: 'Catppuccin Mocha', theme: 'dark' },
  { id: BUILT_IN, label: 'Untwist', theme: 'dark' },
  { id: 'github-light', label: 'GitHub Light', theme: 'light' },
  { id: 'solarized-light', label: 'Solarized Light', theme: 'light' },
  { id: 'catppuccin-latte', label: 'Catppuccin Latte', theme: 'light' },
  { id: BUILT_IN, label: 'Untwist', theme: 'light' },
];

export const DEFAULT_PICKS: EditorThemePicks = { dark: 'github-dark', light: 'github-light' };

export function themesFor(theme: Theme): readonly EditorTheme[] {
  return EDITOR_THEMES.filter((entry) => entry.theme === theme);
}

function known(theme: Theme, id: unknown): id is string {
  return typeof id === 'string' && themesFor(theme).some((entry) => entry.id === id);
}

export function parsePicks(raw: string | null): EditorThemePicks {
  if (raw === null) {
    return DEFAULT_PICKS;
  }
  try {
    const value = JSON.parse(raw) as Partial<Record<Theme, unknown>> | null;
    return {
      dark: known('dark', value?.dark) ? value.dark : DEFAULT_PICKS.dark,
      light: known('light', value?.light) ? value.light : DEFAULT_PICKS.light,
    };
  } catch {
    return DEFAULT_PICKS;
  }
}

export function activeEditorTheme(theme: Theme, picks: EditorThemePicks): string {
  return picks[theme];
}

export function withPick(picks: EditorThemePicks, theme: Theme, id: string): EditorThemePicks {
  return known(theme, id) ? { ...picks, [theme]: id } : picks;
}
