import { existsSync, readFileSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { extname, join, normalize, sep } from 'node:path';

interface Rule {
  readonly pattern: string;
  readonly headers: ReadonlyArray<readonly [string, string]>;
}

const TYPES: Readonly<Record<string, string>> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.json': 'application/json',
  '.webmanifest': 'application/manifest+json',
  '.txt': 'text/plain; charset=utf-8',
  '.table': 'application/octet-stream',
};

export function parseHeaders(text: string): Rule[] {
  const rules: Array<{ pattern: string; headers: Array<[string, string]> }> = [];
  for (const line of text.split('\n')) {
    if (line.trim() === '' || line.trim().startsWith('#')) {
      continue;
    }
    if (!/^\s/.test(line)) {
      rules.push({ pattern: line.trim(), headers: [] });
      continue;
    }
    const split = line.indexOf(':');
    rules.at(-1)?.headers.push([line.slice(0, split).trim(), line.slice(split + 1).trim()]);
  }
  return rules;
}

function matches(pattern: string, path: string): boolean {
  return pattern.endsWith('*') ? path.startsWith(pattern.slice(0, -1)) : path === pattern;
}

export interface DistServer {
  readonly url: string;
  patchWorker: ((code: string) => string) | null;
  close(): Promise<void>;
}

export async function serveDist(dist: string): Promise<DistServer> {
  const rules = parseHeaders(readFileSync(join(dist, '_headers'), 'utf8'));
  const state: { patchWorker: ((code: string) => string) | null } = { patchWorker: null };
  const server = createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url ?? '/', 'http://localhost').pathname);
    const file = normalize(join(dist, path === '/' ? 'index.html' : path));
    if (!file.startsWith(dist + sep) || !existsSync(file) || !statSync(file).isFile()) {
      response.writeHead(404).end();
      return;
    }
    const headers: Record<string, string> = { 'Content-Type': TYPES[extname(file)] ?? 'application/octet-stream' };
    for (const rule of rules.filter((item) => matches(item.pattern, path))) {
      for (const [name, value] of rule.headers) {
        headers[name] = value;
      }
    }
    const body = readFileSync(file);
    const patched = path === '/sw.js' && state.patchWorker !== null ? Buffer.from(state.patchWorker(body.toString('utf8'))) : body;
    response.writeHead(200, headers).end(patched);
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address() as AddressInfo;
  return {
    url: `http://localhost:${port}/`,
    get patchWorker() {
      return state.patchWorker;
    },
    set patchWorker(patch) {
      state.patchWorker = patch;
    },
    close: () => new Promise<void>((resolve, reject) => server.close((error) => (error === undefined ? resolve() : reject(error)))),
  };
}
