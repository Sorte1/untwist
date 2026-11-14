import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { activeEditorTheme, BUILT_IN, DEFAULT_PICKS, EDITOR_THEMES, parsePicks, themesFor, withPick } from './themes';

describe('editor theme picks', () => {
  it('defaults to Tokyo Night in dark mode and GitHub Light in light mode', () => {
    expect(parsePicks(null)).toEqual({ dark: 'tokyo-night', light: 'github-light' });
    expect(activeEditorTheme('dark', DEFAULT_PICKS)).toBe('tokyo-night');
    expect(activeEditorTheme('light', DEFAULT_PICKS)).toBe('github-light');
  });

  it('reads stored picks and drops unknown or mismatched ids', () => {
    expect(parsePicks('{"dark":"one-dark","light":"solarized-light"}')).toEqual({ dark: 'one-dark', light: 'solarized-light' });
    expect(parsePicks('{"dark":"github-light","light":"nope"}')).toEqual(DEFAULT_PICKS);
    expect(parsePicks('not json')).toEqual(DEFAULT_PICKS);
    expect(parsePicks('null')).toEqual(DEFAULT_PICKS);
  });

  it('keeps the built-in palette selectable in both modes', () => {
    expect(themesFor('dark').map((entry) => entry.id)).toContain(BUILT_IN);
    expect(themesFor('light').map((entry) => entry.id)).toContain(BUILT_IN);
  });

  it('changes only the pick for the given mode and ignores ids of the other mode', () => {
    expect(withPick(DEFAULT_PICKS, 'dark', 'catppuccin-mocha')).toEqual({ dark: 'catppuccin-mocha', light: 'github-light' });
    expect(withPick(DEFAULT_PICKS, 'dark', 'github-light')).toBe(DEFAULT_PICKS);
  });

  it('lists the same themes in the settings panel as in the theme table', () => {
    const html = readFileSync(new URL('../../index.html', import.meta.url), 'utf8');
    for (const theme of ['dark', 'light'] as const) {
      const group = html.match(new RegExp(`<div [^>]*id="editor-theme-${theme}"[^>]*>(.*?)</div>`))?.[1] ?? '';
      const listed = [...group.matchAll(/<button [^>]*data-pick="([^"]+)"[^>]*>([^<]+)<\/button>/g)].map((match) => [match[1], match[2]]);
      expect(listed).toEqual(EDITOR_THEMES.filter((entry) => entry.theme === theme).map((entry) => [entry.id, entry.label]));
    }
  });
});
