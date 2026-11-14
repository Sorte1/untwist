import { describe, expect, it } from 'vitest';
import { cachedStars, fetchStars, REPO_API, RETRY_AFTER_MS, starsLabel, STARS_TTL_MS } from './stars';

function respond(status: number, body: unknown): typeof fetch {
  return (async () => new Response(JSON.stringify(body), { status })) as typeof fetch;
}

describe('GitHub stars', () => {
  it('shows small counts in full and large counts compact', () => {
    expect(starsLabel(0)).toBe('0');
    expect(starsLabel(999)).toBe('999');
    expect(starsLabel(1234)).toBe('1.2k');
    expect(starsLabel(9999)).toBe('9.9k');
    expect(starsLabel(12_345)).toBe('12k');
  });

  it('uses a cached count only while it is fresh', () => {
    const now = 1_000_000_000;
    expect(cachedStars(JSON.stringify({ count: 7, at: now - 1000 }), now)).toEqual({ kind: 'count', count: 7 });
    expect(cachedStars(JSON.stringify({ count: 7, at: now - STARS_TTL_MS }), now)).toEqual({ kind: 'none' });
    expect(cachedStars(JSON.stringify({ count: 7, at: now + 1000 }), now)).toEqual({ kind: 'none' });
    expect(cachedStars(JSON.stringify({ count: -1, at: now }), now)).toEqual({ kind: 'none' });
    expect(cachedStars('{', now)).toEqual({ kind: 'none' });
    expect(cachedStars(null, now)).toEqual({ kind: 'none' });
  });

  it('waits an hour after a failed fetch before asking again', () => {
    const now = 1_000_000_000;
    expect(cachedStars(JSON.stringify({ count: null, at: now - 1000 }), now)).toEqual({ kind: 'wait' });
    expect(cachedStars(JSON.stringify({ count: null, at: now - RETRY_AFTER_MS }), now)).toEqual({ kind: 'none' });
  });

  it('reads stargazers_count from the repository API', async () => {
    let asked = '';
    const fetcher = (async (url: string) => {
      asked = url;
      return new Response(JSON.stringify({ stargazers_count: 42 }), { status: 200 });
    }) as unknown as typeof fetch;
    await expect(fetchStars(fetcher)).resolves.toBe(42);
    expect(asked).toBe(REPO_API);
  });

  it('gives no count on a rate limit, a bad body or a network error', async () => {
    await expect(fetchStars(respond(403, { message: 'API rate limit exceeded' }))).resolves.toBeNull();
    await expect(fetchStars(respond(200, { stargazers_count: '42' }))).resolves.toBeNull();
    await expect(fetchStars(respond(200, null))).resolves.toBeNull();
    await expect(fetchStars((async () => Promise.reject(new TypeError('offline'))) as typeof fetch)).resolves.toBeNull();
  });
});
