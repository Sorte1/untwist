import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { palettes } from '../../scripts/contrast';
import { headerPolicy } from '../csp';
import { settingsFromHash } from '../settings/hash';
import type { Theme } from '../state';
import { activeEditorTheme, BUILT_IN, EDITOR_THEMES_KEY, parsePicks } from './themes';

const PAGE = readFileSync(new URL('../../index.html', import.meta.url), 'utf8');
const HEADERS = readFileSync(new URL('../../public/_headers', import.meta.url), 'utf8');
const BOOT = PAGE.match(/<script id="theme-boot">(.*?)<\/script>/)?.[1] ?? '';

interface Booted {
  readonly attributes: ReadonlyMap<string, string>;
  readonly themeColours: readonly string[];
}

function boot(hash: string, systemLight: boolean, stored: string | null, storageThrows = false): Booted {
  const attributes = new Map<string, string>();
  const metas = [0, 1].map(() => ({ content: '', setAttribute(name: string, value: string) { if (name === 'content') this.content = value; } }));
  const document = {
    documentElement: { setAttribute: (name: string, value: string) => attributes.set(name, value) },
    querySelectorAll: (selector: string) => (selector === 'meta[name="theme-color"]' ? metas : []),
  };
  const localStorage = {
    getItem: (key: string) => {
      if (storageThrows) {
        throw new Error('blocked');
      }
      return key === EDITOR_THEMES_KEY ? stored : null;
    },
  };
  const matchMedia = (query: string) => ({ matches: query === '(prefers-color-scheme: light)' && systemLight });
  new Function('document', 'location', 'matchMedia', 'localStorage', BOOT)(document, { hash }, matchMedia, localStorage);
  return { attributes, themeColours: metas.map((meta) => meta.content) };
}

function expected(hash: string, systemLight: boolean, stored: string | null): { theme: Theme; editor: string; background: string } {
  const theme = settingsFromHash(hash).settings.theme ?? (systemLight ? 'light' : 'dark');
  const editor = activeEditorTheme(theme, parsePicks(stored));
  const background = palettes().find((palette) => palette.id === editor && palette.mode === theme)?.colours.get('bg') ?? '';
  return { theme, editor, background };
}

const HASHES = ['', '#theme=light', '#theme=dark', '#unsafe=0&theme=light', '#theme=light&wrap=1', '#theme=blue', '#xtheme=light'];
const STORED = [
  null,
  '{"dark":"one-dark","light":"solarized-light"}',
  '{"dark":"untwist","light":"untwist"}',
  '{"dark":"github-light","light":"nope"}',
  '{"dark":["one-dark"],"light":7}',
  '{"dark":"__proto__","light":"toString"}',
  'not json',
  'null',
];

describe('the inline theme boot script', () => {
  it('exists, so the theme is set before the first paint', () => {
    expect(BOOT.length).toBeGreaterThan(0);
    expect(BOOT.length).toBeLessThan(1_024);
  });

  for (const systemLight of [false, true]) {
    for (const hash of HASHES) {
      for (const stored of STORED) {
        it(`picks the same theme as the app for hash "${hash}", ${systemLight ? 'light' : 'dark'} system and stored ${stored}`, () => {
          const want = expected(hash, systemLight, stored);
          const got = boot(hash, systemLight, stored);
          expect(got.attributes.get('data-theme')).toBe(want.theme);
          expect(got.attributes.get('data-editor')).toBe(want.editor === BUILT_IN ? undefined : want.editor);
          expect(got.themeColours).toEqual([want.background, want.background]);
        });
      }
    }
  }

  it('falls back to the default theme when storage is blocked', () => {
    const got = boot('', false, null, true);
    expect(got.attributes.get('data-theme')).toBe('dark');
    expect(got.attributes.get('data-editor')).toBe('tokyo-night');
  });

  it('is allowed by its hash in script-src and nothing else inline is', () => {
    const scriptSrc = headerPolicy(HEADERS).split(';').map((directive) => directive.trim()).find((directive) => directive.startsWith('script-src')) ?? '';
    const digest = createHash('sha256').update(BOOT, 'utf8').digest('base64');
    expect(scriptSrc).toBe(`script-src 'self' 'wasm-unsafe-eval' 'sha256-${digest}'`);
    const inline = [...PAGE.matchAll(/<script(?![^>]*\bsrc=)(?![^>]*type="application\/ld\+json")[^>]*>/g)];
    expect(inline).toHaveLength(1);
  });
});
