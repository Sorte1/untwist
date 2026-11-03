import { cacheName, isCachedFirst, shellKey, staleCaches } from './cache';

declare const __UNTWIST_PRECACHE__: readonly string[];
declare const __UNTWIST_VERSION__: string;

const worker = self as unknown as ServiceWorkerGlobalScope;
const CACHE = cacheName(__UNTWIST_VERSION__);
const HASHED = /^assets\//;

function scope(): URL {
  return new URL(worker.registration.scope);
}

function precacheRequest(path: string): Request {
  return new Request(new URL(path, scope()), { cache: HASHED.test(path) ? 'default' : 'reload' });
}

worker.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(__UNTWIST_PRECACHE__.map(precacheRequest))));
});

worker.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(staleCaches(keys, CACHE).map((key) => caches.delete(key))))
      .then(() => worker.clients.claim()),
  );
});

worker.addEventListener('message', (event) => {
  if ((event.data as { kind?: unknown } | null)?.kind === 'skip-waiting') {
    void worker.skipWaiting();
  }
});

worker.addEventListener('fetch', (event) => {
  const request = event.request;
  if (!isCachedFirst(request, scope())) {
    return;
  }
  const shell = request.mode === 'navigate' ? shellKey(new URL(request.url), scope()) : null;
  event.respondWith(
    caches
      .open(CACHE)
      .then((cache) => cache.match(shell ?? request))
      .then((hit) => hit ?? fetch(request)),
  );
});
