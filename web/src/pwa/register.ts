const IDLE_TIMEOUT_MS = 3000;
const SCRIPT = './sw.js';

function idle(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof window.requestIdleCallback === 'function') {
      window.requestIdleCallback(() => resolve(), { timeout: IDLE_TIMEOUT_MS });
    } else {
      window.setTimeout(resolve, IDLE_TIMEOUT_MS);
    }
  });
}

function offer(worker: ServiceWorker): void {
  import('./update-prompt').then(
    ({ showUpdatePrompt }) => showUpdatePrompt(worker),
    () => undefined,
  );
}

function watch(registration: ServiceWorkerRegistration): void {
  if (registration.waiting !== null && navigator.serviceWorker.controller !== null) {
    offer(registration.waiting);
  }
  registration.addEventListener('updatefound', () => {
    const installing = registration.installing;
    installing?.addEventListener('statechange', () => {
      if (installing.state === 'installed' && navigator.serviceWorker.controller !== null) {
        offer(installing);
      }
    });
  });
}

export function registerServiceWorker(ready: Promise<void>): void {
  if (!import.meta.env.PROD || !('serviceWorker' in navigator)) {
    return;
  }
  ready
    .then(idle)
    .then(() => navigator.serviceWorker.register(SCRIPT, { scope: './', updateViaCache: 'none' }))
    .then(watch, () => undefined);
}
