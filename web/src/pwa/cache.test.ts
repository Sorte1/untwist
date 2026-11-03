import { describe, expect, it } from 'vitest';
import { cacheName, isCachedFirst, shellKey, staleCaches } from './cache';

const SCOPE = new URL('https://untwist.dev/');

describe('cacheName and staleCaches', () => {
  it('names a cache per version and finds the other versions to delete', () => {
    expect(cacheName('abc')).toBe('untwist-abc');
    expect(staleCaches(['untwist-abc', 'untwist-old', 'other-app'], 'untwist-abc')).toEqual(['untwist-old']);
  });
});

describe('shellKey', () => {
  it('maps a navigation to the page, with or without index.html or a query, to the scope', () => {
    for (const path of ['https://untwist.dev/', 'https://untwist.dev/index.html', 'https://untwist.dev/?x=1']) {
      expect(shellKey(new URL(path), SCOPE)).toBe('https://untwist.dev/');
    }
  });

  it('leaves other paths alone', () => {
    expect(shellKey(new URL('https://untwist.dev/favicon.svg'), SCOPE)).toBeNull();
    expect(shellKey(new URL('https://other.dev/'), SCOPE)).toBeNull();
  });
});

describe('isCachedFirst', () => {
  it('serves only same-origin GET requests from the cache', () => {
    expect(isCachedFirst({ method: 'GET', url: 'https://untwist.dev/assets/a.js' }, SCOPE)).toBe(true);
    expect(isCachedFirst({ method: 'POST', url: 'https://untwist.dev/assets/a.js' }, SCOPE)).toBe(false);
    expect(isCachedFirst({ method: 'GET', url: 'https://cdn.example/a.js' }, SCOPE)).toBe(false);
  });
});
