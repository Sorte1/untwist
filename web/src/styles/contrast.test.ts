import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { BUILT_IN, EDITOR_THEMES } from '../editor-theme/themes';
import { pairs, palettes } from '../../scripts/contrast';

const EDITOR_CSS = readFileSync(new URL('./editor-themes.css', import.meta.url), 'utf8');
const SITE_TOKENS = ['bg', 'panel', 'head', 'border', 'hair', 'text', 'muted', 'faint', 'track', 'knob', 'sel', 'btn', 'btn-border', 'overlay', 'strand-a', 'strand-b', 'accent', 'on-accent', 'del-bg', 'add-bg', 'del-mark', 'add-mark', 'warn', 'warn-bg', 'info', 'err', 'ok', 'shadow', 'focus', 'scroll-thumb', 'code', 'plain', 'selection', 'diff-del-tint', 'diff-add-tint'];

function blockOf(id: string): string {
  const start = EDITOR_CSS.indexOf(`:root[data-editor='${id}']`);
  return EDITOR_CSS.slice(start, EDITOR_CSS.indexOf('}', start));
}

describe('theme contrast (WCAG AA: 4.5:1 for text, 3:1 for controls and focus)', () => {
  for (const palette of palettes()) {
    it(`holds for every pair in ${palette.label} (${palette.mode})`, () => {
      const failing = pairs(palette).filter((pair) => !(pair.ratio >= pair.min));
      expect(failing).toEqual([]);
    });
  }

  it('has an editor theme block for every theme in the picker', () => {
    const blocks = [...EDITOR_CSS.matchAll(/:root\[data-editor='([a-z-]+)'\]/g)].map((match) => match[1] ?? '');
    expect(blocks.sort()).toEqual(EDITOR_THEMES.filter((theme) => theme.id !== BUILT_IN).map((theme) => theme.id).sort());
  });

  for (const theme of EDITOR_THEMES.filter((entry) => entry.id !== BUILT_IN)) {
    it(`gives the whole site its colours in ${theme.label}, not only the editors`, () => {
      const block = blockOf(theme.id);
      expect(SITE_TOKENS.filter((token) => !block.includes(`--${token}:`))).toEqual([]);
    });
  }
});
