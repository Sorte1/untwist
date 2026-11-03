import { describe, expect, it } from 'vitest';
import { injectPrecache, precacheEntries, versionOf } from './precache';

const FILES = [
  'index.html',
  'assets/index-AAAA.js',
  'assets/index-BBBB.css',
  'assets/wasm/untwist_wasm_bg-CCCC.wasm',
  'assets/run.worker-DDDD.js',
  '.vite/manifest.json',
  '_headers',
  'sw.js',
  'assets/index-AAAA.js.map',
  'fonts/OFL-Spline-Sans.txt',
  'favicon.svg',
  'site.webmanifest',
];

describe('precacheEntries', () => {
  it('keeps the page, the hashed assets, the wasm and the icons, sorted, with the page as ./', () => {
    expect(precacheEntries(FILES)).toEqual([
      './',
      'assets/index-AAAA.js',
      'assets/index-BBBB.css',
      'assets/run.worker-DDDD.js',
      'assets/wasm/untwist_wasm_bg-CCCC.wasm',
      'favicon.svg',
      'site.webmanifest',
    ]);
  });

  it('lists each file once', () => {
    expect(precacheEntries(['a.png', 'a.png', 'index.html'])).toEqual(['./', 'a.png']);
  });
});

describe('versionOf', () => {
  it('changes with the file list, the page and the unhashed files, and is stable otherwise', () => {
    const icon = new Uint8Array([1, 2, 3]);
    const base = versionOf(['./', 'assets/a-1.js', 'favicon.svg'], ['<html>', icon]);
    expect(base).toMatch(/^[0-9a-f]{16}$/);
    expect(versionOf(['./', 'assets/a-1.js', 'favicon.svg'], ['<html>', icon])).toBe(base);
    expect(versionOf(['./', 'assets/a-2.js', 'favicon.svg'], ['<html>', icon])).not.toBe(base);
    expect(versionOf(['./', 'assets/a-1.js', 'favicon.svg'], ['<html lang>', icon])).not.toBe(base);
    expect(versionOf(['./', 'assets/a-1.js', 'favicon.svg'], ['<html>', new Uint8Array([1, 2, 4])])).not.toBe(base);
  });
});

describe('injectPrecache', () => {
  it('replaces both placeholders', () => {
    const code = 'const a = __UNTWIST_PRECACHE__; const b = __UNTWIST_VERSION__;';
    expect(injectPrecache(code, ['./', 'x.js'], 'abc')).toBe('const a = ["./","x.js"]; const b = "abc";');
  });

  it('refuses code without the placeholders', () => {
    expect(() => injectPrecache('const a = 1;', [], 'abc')).toThrow(/placeholder/);
  });
});
