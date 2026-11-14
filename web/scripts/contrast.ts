import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { BUILT_IN, EDITOR_THEMES } from '../src/editor-theme/themes.ts';

const FALLBACKS: Readonly<Record<string, string>> = { prop: 'plain', type: 'plain', op: 'plain', gutter: 'faint' };

const STYLES = resolve(dirname(fileURLToPath(import.meta.url)), '../src/styles');
const TOKENS = readFileSync(join(STYLES, 'tokens.css'), 'utf8');
const EDITOR = readFileSync(join(STYLES, 'editor-themes.css'), 'utf8');

export const TEXT_AA = 4.5;
export const UI_AA = 3;

export interface Check {
  readonly name: string;
  readonly foregrounds: readonly string[];
  readonly surfaces: readonly string[];
  readonly min: number;
}

export const CHECKS: readonly Check[] = [
  { name: 'code', foregrounds: ['plain', 'kw', 'str', 'num', 'fn', 'com', 'prop', 'type', 'op', 'gutter', 'faint', 'text'], surfaces: ['code', 'selection', 'diff-del-tint', 'diff-add-tint'], min: TEXT_AA },
  { name: 'ui text', foregrounds: ['text', 'muted', 'faint', 'warn', 'info', 'err', 'ok', 'del-mark', 'add-mark'], surfaces: ['bg', 'panel', 'head', 'code', 'btn'], min: TEXT_AA },
  { name: 'hover', foregrounds: ['text', 'muted'], surfaces: ['sel'], min: TEXT_AA },
  { name: 'run button', foregrounds: ['on-accent'], surfaces: ['accent'], min: TEXT_AA },
  { name: 'unsafe tint', foregrounds: ['warn'], surfaces: ['warn-bg'], min: TEXT_AA },
  { name: 'controls', foregrounds: ['accent', 'focus', 'track'], surfaces: ['bg', 'panel', 'head', 'code'], min: UI_AA },
  { name: 'switch knob', foregrounds: ['on-accent'], surfaces: ['accent'], min: UI_AA },
];

export interface Palette {
  readonly id: string;
  readonly mode: 'dark' | 'light';
  readonly label: string;
  readonly colours: ReadonlyMap<string, string>;
}

export interface Pair {
  readonly check: string;
  readonly foreground: string;
  readonly surface: string;
  readonly ratio: number;
  readonly min: number;
}

function block(css: string, selector: string): ReadonlyMap<string, string> {
  const start = css.indexOf(selector);
  if (start < 0) {
    throw new Error(`no ${selector} block`);
  }
  const body = css.slice(start, css.indexOf('}', start));
  return new Map([...body.matchAll(/--([a-z-]+):\s*(#[0-9A-Fa-f]{6})/g)].map((match) => [match[1] ?? '', match[2] ?? '']));
}

export function palettes(): readonly Palette[] {
  return EDITOR_THEMES.map((theme) => {
    const base = block(TOKENS, `:root[data-theme='${theme.theme}']`);
    const own = theme.id === BUILT_IN ? new Map<string, string>() : block(EDITOR, `:root[data-editor='${theme.id}']`);
    return { id: theme.id, mode: theme.theme, label: theme.label, colours: new Map([...base, ...own]) };
  });
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

export function pairs(palette: Palette): readonly Pair[] {
  const colour = (name: string) => {
    const value = palette.colours.get(name) ?? palette.colours.get(FALLBACKS[name] ?? '');
    if (value === undefined) {
      throw new Error(`${palette.label} (${palette.mode}) has no --${name}`);
    }
    return value;
  };
  return CHECKS.flatMap((check) =>
    check.foregrounds.flatMap((foreground) =>
      check.surfaces.map((surface) => ({ check: check.name, foreground, surface, ratio: contrast(colour(foreground), colour(surface)), min: check.min })),
    ),
  );
}

export function weakest(palette: Palette, min: number): Pair {
  return pairs(palette)
    .filter((pair) => pair.min === min)
    .reduce((low, pair) => (pair.ratio < low.ratio ? pair : low));
}

function report(): void {
  let failed = 0;
  for (const palette of palettes()) {
    const text = weakest(palette, TEXT_AA);
    const ui = weakest(palette, UI_AA);
    const fails = pairs(palette).filter((pair) => pair.ratio < pair.min);
    failed += fails.length;
    console.log(
      `${`${palette.label} (${palette.mode})`.padEnd(26)} text ${text.ratio.toFixed(2)} (${text.foreground} on ${text.surface})  ui ${ui.ratio.toFixed(2)} (${ui.foreground} on ${ui.surface})${fails.length === 0 ? '' : `  FAIL ${fails.map((pair) => `${pair.foreground}/${pair.surface} ${pair.ratio.toFixed(2)}`).join(', ')}`}`,
    );
  }
  process.exitCode = failed === 0 ? 0 : 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  report();
}
