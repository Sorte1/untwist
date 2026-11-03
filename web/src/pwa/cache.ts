export const CACHE_PREFIX = 'untwist-';

export function cacheName(version: string): string {
  return `${CACHE_PREFIX}${version}`;
}

export function staleCaches(keys: readonly string[], current: string): string[] {
  return keys.filter((key) => key.startsWith(CACHE_PREFIX) && key !== current);
}

export function shellKey(url: URL, scope: URL): string | null {
  if (url.origin !== scope.origin) {
    return null;
  }
  return url.pathname === scope.pathname || url.pathname === `${scope.pathname}index.html` ? scope.href : null;
}

export function isCachedFirst(request: { readonly method: string; readonly url: string }, scope: URL): boolean {
  return request.method === 'GET' && new URL(request.url).origin === scope.origin;
}
