import { describe, expect, it } from 'vitest';
import { checkedWasm, hasWasmHeader, isWasmName, limitedBody, parseWasmReport, resolvedUrl, withSupplied } from './wasm';

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

  it('reads a fetched body up to the limit', async () => {
    const body = await limitedBody(new Response(new Uint8Array([1, 2, 3, 4])), 4);
    expect(body).toEqual(new Uint8Array([1, 2, 3, 4]));
    expect(await limitedBody(new Response(null), 4)).toEqual(new Uint8Array(0));
  });

  it('refuses a body whose declared length is over the limit without reading it', async () => {
    let pulled = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        pulled += 1;
        controller.enqueue(new Uint8Array(1));
      },
    }, { highWaterMark: 0 });
    const response = new Response(stream, { headers: { 'content-length': '5' } });
    expect(await limitedBody(response, 4)).toBeNull();
    expect(pulled).toBe(0);
  });

  it('stops reading a body that grows past the limit', async () => {
    let pulled = 0;
    let cancelled = false;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        pulled += 1;
        controller.enqueue(new Uint8Array(3));
      },
      cancel() {
        cancelled = true;
      },
    }, { highWaterMark: 0 });
    expect(await limitedBody(new Response(stream), 4)).toBeNull();
    expect(pulled).toBe(2);
    expect(cancelled).toBe(true);
  });

  it('replaces a supplied file of the same name', () => {
    const first = { name: 'a.wasm', bytes: HEADER, url: null };
    const second = { name: 'a.wasm', bytes: new Uint8Array(HEADER), url: 'https://x/a.wasm' };
    expect(withSupplied([first], [second])).toEqual([second]);
  });
});
