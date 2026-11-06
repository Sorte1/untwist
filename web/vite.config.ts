import { defineConfig, type Plugin } from 'vite';
import { serviceWorker } from './scripts/service-worker.ts';

const CSP = [
  "default-src 'self'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "worker-src 'self'",
  "connect-src 'self' https:",
  "img-src 'self' data:",
  "style-src 'self'",
  "font-src 'self'",
].join('; ');

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
  plugins: [contentSecurityPolicy(), serviceWorker()],
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
