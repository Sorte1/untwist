import { readFileSync } from 'node:fs';
import { defineConfig, type Plugin } from 'vite';
import { metaPolicy } from './src/csp.ts';
import { serviceWorker } from './scripts/service-worker.ts';
import { libraryTables } from './scripts/library-tables.ts';

const CSP = metaPolicy(readFileSync(new URL('./public/_headers', import.meta.url), 'utf8'));

const output = {
  assetFileNames: (asset: { readonly names: readonly string[] }) =>
    asset.names.some((name) => name.endsWith('.wasm')) ? 'assets/wasm/[name]-[hash][extname]' : 'assets/[name]-[hash][extname]',
};

function contentSecurityPolicy(): Plugin {
  return {
    name: 'untwist-csp',
    apply: 'build',
    transformIndexHtml: () => [
      { tag: 'meta', attrs: { 'http-equiv': 'Content-Security-Policy', content: CSP }, injectTo: 'head-prepend' },
    ],
  };
}

export default defineConfig({
  base: './',
  plugins: [contentSecurityPolicy(), libraryTables(), serviceWorker()],
  build: {
    target: 'es2022',
    manifest: true,
    sourcemap: false,
    modulePreload: { polyfill: false },
    assetsInlineLimit: 0,
    rolldownOptions: { output },
  },
  worker: {
    format: 'es',
    rolldownOptions: { output },
  },
  preview: {
    port: 4173,
    strictPort: true,
  },
});
