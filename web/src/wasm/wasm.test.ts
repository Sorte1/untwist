import { describe, expect, it } from 'vitest';
import { checkedUrl, checkedWasm, fetchTarget, hasScheme, hasWasmHeader, isWasmName, limitedBody, parseWasmReport, withSupplied } from './wasm';

function href(reference: string, base: string | null = null): string | null {
  const target = fetchTarget(reference, base);
  return target.ok ? target.href : null;
}

function reason(reference: string, base: string | null = null): string {
  const target = fetchTarget(reference, base);
  return target.ok ? '' : target.reason;
}

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
    expect(href('https://cdn.dev/a.wasm')).toBe('https://cdn.dev/a.wasm');
    expect(href('pkg/a.wasm')).toBeNull();
    expect(href('pkg/a.wasm', 'pkg/')).toBeNull();
    expect(href('pkg/a.wasm', 'https://x.dev/app/index.html')).toBe('https://x.dev/app/pkg/a.wasm');
    expect(href('/static/a.wasm', ' https://x.dev/app/ ')).toBe('https://x.dev/static/a.wasm');
    expect(href('//cdn.dev/a.wasm', 'https://x.dev/app/')).toBe('https://cdn.dev/a.wasm');
    expect(href('../../../e.wasm', 'https://x.dev/a/c.js')).toBe('https://x.dev/e.wasm');
    expect(href('a.wasm?h=2#top', 'https://x.dev/app/index.html?v=1#x')).toBe('https://x.dev/app/a.wasm?h=2');
  });

  it('tells a url with a scheme from a relative one as the cli does', () => {
    expect(hasScheme('https://x.dev/a.wasm')).toBe(true);
    expect(hasScheme('ftp://x.dev/a.wasm')).toBe(true);
    expect(hasScheme('data:application/wasm,a')).toBe(true);
    expect(hasScheme('pkg/a.wasm')).toBe(false);
    expect(hasScheme('//cdn.dev/a.wasm')).toBe(false);
    expect(hasScheme('1x:a.wasm')).toBe(false);
  });

  it('gives the parsed host and the href without its fragment', () => {
    expect(fetchTarget('HTTPS://CDN.Dev:8443/a.wasm#x', null)).toEqual({ ok: true, href: 'https://cdn.dev:8443/a.wasm', host: 'cdn.dev:8443' });
    expect(fetchTarget('https://bücher.example/a.wasm', null).ok).toBe(false);
    expect(fetchTarget('https://xn--bcher-kva.example/a.wasm', null)).toMatchObject({ ok: true, host: 'xn--bcher-kva.example' });
  });

  it('refuses other schemes and urls that cannot be requested', () => {
    for (const url of ['ftp://x.dev/a.wasm', 'file:///etc/a.wasm', 'data:application/wasm,a.wasm', 'blob:https://x.dev/1', 'javascript:alert(1)', 'http:///a.wasm', 'https:x.dev/a.wasm', 'https:\\\\x.dev\\a.wasm', 'https://x.dev/a b.wasm', 'http://x.dev/\u001b[31m/a.wasm']) {
      expect(reason(url), url).toContain('is not an http(s) URL that can be requested');
    }
    expect(reason('file:///etc/a.wasm', 'https://x.dev/')).toContain('is not an http(s) URL');
    expect(reason('a.wasm', 'ftp://x.dev/')).toContain('is not an http(s) URL');
    expect(reason('a b.wasm', 'https://x.dev/')).toContain('is not an http(s) URL');
    expect(reason('\\\\evil.dev\\a.wasm', 'https://x.dev/')).toContain('is not an http(s) URL');
  });

  it('refuses a url with a user name or password, also through the base', () => {
    expect(reason('https://user@x.dev/a.wasm')).toContain('carries a user name or password');
    expect(reason('https://user:pw@x.dev/a.wasm')).toContain('carries a user name or password');
    expect(reason('https://:pw@x.dev/a.wasm')).toContain('carries a user name or password');
    expect(reason('https://x.dev@evil.dev/a.wasm')).toContain('carries a user name or password');
    expect(reason('a.wasm', 'https://user:pw@x.dev/app/')).toContain('carries a user name or password');
    expect(reason('//user@cdn.dev/a.wasm', 'https://x.dev/')).toContain('carries a user name or password');
  });

  it('refuses local and private hosts', () => {
    expect(reason('http://127.0.0.1:4173/a.wasm')).toContain('the host is the loopback address 127.0.0.1');
    expect(reason('http://0x7f.1/a.wasm')).toContain('loopback address 127.0.0.1');
    expect(reason('http://2130706433/a.wasm')).toContain('loopback address 127.0.0.1');
    expect(reason('https://[::ffff:10.0.0.1]/a.wasm')).toContain('private address');
    expect(reason('https://169.254.169.254/latest')).toContain('link-local');
    expect(reason('https://localhost/a.wasm')).toContain('loopback name');
    expect(reason('a.wasm', 'https://192.168.1.1/app/')).toContain('private address 192.168.1.1');
    expect(reason('//10.0.0.1/a.wasm', 'https://x.dev/')).toContain('private address 10.0.0.1');
    expect(checkedUrl('https://10.0.0.1/a.wasm').ok).toBe(false);
    for (const url of ['http://127.0.0.1../', 'http://10.0.0.1../', 'http://0x7f.0.0.1../', 'http://localhost../', 'http://x.internal../', 'http://a..b.dev/']) {
      expect(reason(url), url).toContain('empty label');
    }
    expect(href('https://93.184.216.34/a.wasm')).toBe('https://93.184.216.34/a.wasm');
    expect(href('https://[2606:4700:4700::1111]/a.wasm')).toBe('https://[2606:4700:4700::1111]/a.wasm');
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
