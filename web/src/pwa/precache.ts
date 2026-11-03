import { createHash } from 'node:crypto';

const SKIPPED: readonly RegExp[] = [/^\.vite\//, /^_headers$/, /^sw\.js$/, /\.map$/, /^fonts\/.*\.txt$/];
const PRECACHE_TOKEN = '__UNTWIST_PRECACHE__';
const VERSION_TOKEN = '__UNTWIST_VERSION__';
const VERSION_CHARS = 16;

export function precacheEntries(files: readonly string[]): string[] {
  const kept = files.filter((file) => !SKIPPED.some((pattern) => pattern.test(file))).map((file) => (file === 'index.html' ? './' : file));
  return [...new Set(kept)].sort();
}

export function versionOf(entries: readonly string[], html: string): string {
  return createHash('sha256').update(entries.join('\n')).update('\0').update(html).digest('hex').slice(0, VERSION_CHARS);
}

export function injectPrecache(code: string, entries: readonly string[], version: string): string {
  if (!code.includes(PRECACHE_TOKEN) || !code.includes(VERSION_TOKEN)) {
    throw new Error('the service worker lacks its precache placeholders');
  }
  return code.replaceAll(PRECACHE_TOKEN, JSON.stringify(entries)).replaceAll(VERSION_TOKEN, JSON.stringify(version));
}
