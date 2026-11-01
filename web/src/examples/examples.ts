import manifest from './manifest.json';

export interface Example {
  readonly name: string;
  readonly source: string;
  readonly licence: string | null;
  readonly bytes: number;
}

export const EXAMPLES: readonly Example[] = manifest;

const LOADERS: Readonly<Record<string, () => Promise<string>>> = {
  'fixtures/reference/deobfuscate/obfuscator.io.js': () => Promise.all([import('./files/obfuscator.io.js?raw'), import('./files/LICENSE-reference.txt?raw')]).then(([module]) => module.default),
  'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js': () => Promise.all([import('./files/obfuscator.io-control-flow.js?raw'), import('./files/LICENSE-reference.txt?raw')]).then(([module]) => module.default),
  'fixtures/reference/unpack/webpack-5.js': () => Promise.all([import('./files/webpack-5.js?raw'), import('./files/LICENSE-reference.txt?raw')]).then(([module]) => module.default),
  'fixtures/reference/unpack/webpack-4.js': () => Promise.all([import('./files/webpack-4.js?raw'), import('./files/LICENSE-reference.txt?raw')]).then(([module]) => module.default),
  'fixtures/unpack-bundlers/rollup/vite-iife.js': () => Promise.all([import('./files/vite-iife.js?raw'), import('./files/LICENSE-vite.txt?raw')]).then(([module]) => module.default),
  'fixtures/reference/unpack/browserify.js': () => Promise.all([import('./files/browserify.js?raw'), import('./files/LICENSE-reference.txt?raw')]).then(([module]) => module.default),
  'fixtures/jsx/classic.js': () => Promise.all([import('./files/classic.js?raw')]).then(([module]) => module.default),
  'fixtures/untranspile/esbuild/async.0.28.1-es2015.js': () => Promise.all([import('./files/async.0.28.1-es2015.js?raw'), import('./files/LICENSE-esbuild.txt?raw')]).then(([module]) => module.default),
};

export function loadExample(source: string): Promise<string> {
  const load = LOADERS[source];
  return load === undefined ? Promise.reject(new Error(`no example ${source}`)) : load();
}
