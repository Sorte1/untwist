import { describe, expect, it } from 'vitest';
import { checkedWasm, hasWasmHeader, isWasmName, parseWasmReport, resolvedUrl, withSupplied } from './wasm';

const HEADER = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);

describe('wasm helpers', () => {
  it('reads the wasm report and falls back to nothing on bad json', () => {
    expect(parseWasmReport('')).toEqual({ files: [], unresolved: [], unchecked: false });
    expect(parseWasmReport('{')).toEqual({ files: [], unresolved: [], unchecked: false });
    expect(parseWasmReport('{"files":[],"unresolved":[{"name":"a.wasm","url":"x/a.wasm"}]}').unresolved[0]?.name).toBe('a.wasm');
  });

  it('checks the magic, the version and the size of a dropped file', () => {
    expect(hasWasmHeader(HEADER)).toBe(true);
    expect(hasWasmHeader(new Uint8Array([0, 97, 115, 109, 2, 0, 0, 0]))).toBe(false);
    expect(checkedWasm('a.wasm', HEADER)).toBeNull();
    expect(checkedWasm('a.wasm', new Uint8Array(4))).toContain('not a wasm module');
    expect(isWasmName('X.WASM')).toBe(true);
    expect(isWasmName('x.js')).toBe(false);
  });

  it('resolves a relative url only against an absolute base', () => {
    expect(resolvedUrl('https://cdn.dev/a.wasm', null)).toBe('https://cdn.dev/a.wasm');
    expect(resolvedUrl('pkg/a.wasm', null)).toBeNull();
    expect(resolvedUrl('pkg/a.wasm', 'pkg/')).toBeNull();
    expect(resolvedUrl('pkg/a.wasm', 'https://x.dev/app/index.html')).toBe('https://x.dev/app/pkg/a.wasm');
  });

  it('replaces a supplied file of the same name', () => {
    const first = { name: 'a.wasm', bytes: HEADER, url: null };
    const second = { name: 'a.wasm', bytes: new Uint8Array(HEADER), url: 'https://x/a.wasm' };
    expect(withSupplied([first], [second])).toEqual([second]);
  });
});
