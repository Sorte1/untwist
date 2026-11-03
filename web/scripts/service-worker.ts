import { readdirSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import type { Plugin, ResolvedConfig } from 'vite';
import { injectPrecache, precacheEntries, versionOf } from '../src/pwa/precache.ts';

const SW_ENTRY = 'src/pwa/sw.ts';
const SW_FILE = 'sw.js';

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry): string[] => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

function publicFiles(dir: string): string[] {
  return dir === '' ? [] : walk(dir).map((path) => relative(dir, path).split(sep).join('/'));
}

export function serviceWorker(): Plugin {
  let config: ResolvedConfig | null = null;
  return {
    name: 'untwist-service-worker',
    apply: 'build',
    enforce: 'post',
    configResolved(resolved) {
      config = resolved;
    },
    buildStart() {
      if (config !== null) {
        this.emitFile({ type: 'chunk', id: resolve(config.root, SW_ENTRY), fileName: SW_FILE });
      }
    },
    generateBundle(_, bundle) {
      const sw = bundle[SW_FILE];
      const html = bundle['index.html'];
      if (sw?.type !== 'chunk' || html?.type !== 'asset') {
        this.error(`the build lacks ${SW_FILE} or index.html`);
      }
      const entries = precacheEntries([...Object.keys(bundle), ...publicFiles(config?.publicDir ?? '')]);
      sw.code = injectPrecache(sw.code, entries, versionOf(entries, String(html.source)));
    },
  };
}
