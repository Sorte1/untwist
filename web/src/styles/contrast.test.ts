import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const HERE = dirname(fileURLToPath(import.meta.url));
const CSS = readFileSync(join(HERE, 'tokens.css'), 'utf8');
const EDITOR_CSS = readFileSync(join(HERE, 'editor-themes.css'), 'utf8');
const EDITOR_TEXT = ['plain', 'kw', 'str', 'num', 'fn', 'com', 'prop', 'type', 'op', 'gutter', 'faint', 'text', 'muted', 'warn', 'info', 'err', 'ok', 'del-mark', 'add-mark'];
const UI_ON_CODE = ['faint', 'text', 'muted', 'warn', 'info', 'err', 'ok', 'del-mark', 'add-mark'];
const CODE_TEXT = ['text', 'plain', 'kw', 'str', 'num', 'fn', 'com', 'faint'];
const CODE_SURFACES = ['code', 'selection', 'diff-del-tint', 'diff-add-tint'];
const UI_TEXT = ['text', 'muted', 'warn', 'info', 'err', 'ok', 'del-mark', 'add-mark'];
const UI_SURFACES = ['bg', 'panel', 'head', 'code', 'btn'];
const HOVER_TEXT = ['text', 'muted'];
const HOVER_SURFACES = ['sel'];
const PAIRS: ReadonlyArray<readonly [readonly string[], readonly string[]]> = [
  [CODE_TEXT, CODE_SURFACES],
  [UI_TEXT, UI_SURFACES],
  [HOVER_TEXT, HOVER_SURFACES],
];

function palette(selector: string, css = CSS): Map<string, string> {
  const start = css.indexOf(selector);
  const block = css.slice(start, css.indexOf('}', start));
  return new Map([...block.matchAll(/--([a-z-]+):\s*(#[0-9A-Fa-f]{6})/g)].map((match) => [match[1] ?? '', match[2] ?? '']));
}

function luminance(hex: string): number {
  const channel = (offset: number) => {
    const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

export function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (light + 0.05) / (dark + 0.05);
}

describe('text contrast (WCAG AA, 4.5:1)', () => {
  for (const theme of ["[data-theme='dark']", "[data-theme='light']"]) {
    it(`holds for every text colour on every surface it can sit on, selection and diff tints included, ${theme}`, () => {
      const colours = palette(theme);
      const failing = PAIRS.flatMap(([foregrounds, surfaces]) =>
        foregrounds.flatMap((fg) => surfaces.map((bg) => [fg, bg, contrast(colours.get(fg) ?? '', colours.get(bg) ?? '')] as const)),
      ).filter(([, , ratio]) => !(ratio >= 4.5));
      expect(failing).toEqual([]);
    });
  }

  for (const theme of ["[data-theme='dark']", "[data-theme='light']"]) {
    it(`holds for the label on the accent Run button and on the Unsafe mode tint, ${theme}`, () => {
      const colours = palette(theme);
      expect(contrast(colours.get('on-accent') ?? '', colours.get('accent') ?? '')).toBeGreaterThan(4.5);
      expect(contrast(colours.get('warn') ?? '', colours.get('warn-bg') ?? '')).toBeGreaterThan(4.5);
    });
  }

  const editorThemes = [...EDITOR_CSS.matchAll(/:root\[data-editor='([a-z-]+)'\]/g)].map((match) => match[1] ?? '');

  it('has an editor theme block for every theme in the picker', () => {
    expect(editorThemes.sort()).toEqual(['catppuccin-latte', 'catppuccin-mocha', 'github-dark', 'github-light', 'one-dark', 'solarized-light', 'tokyo-night']);
  });

  for (const id of editorThemes) {
    it(`holds for every code colour and the UI text drawn on the code surfaces of the ${id} editor theme`, () => {
      const own = palette(`[data-editor='${id}']`, EDITOR_CSS);
      const base = palette(contrast(own.get('code') ?? '', '#000000') > 10 ? "[data-theme='light']" : "[data-theme='dark']");
      const colours = new Map([...base, ...own]);
      const failing = EDITOR_TEXT.flatMap((fg) =>
        CODE_SURFACES.map((bg) => [fg, bg, contrast(colours.get(fg) ?? '', colours.get(bg) ?? '')] as const),
      ).filter(([fg, bg, ratio]) => !(ratio >= 4.5) && !(UI_ON_CODE.includes(fg) && bg !== 'code'));
      expect(failing).toEqual([]);
    });
  }
});
