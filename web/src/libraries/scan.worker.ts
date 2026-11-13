import { tablesIn, type KeyedTable } from './scan';

interface ScanRequest {
  readonly text: string;
  readonly tables: readonly KeyedTable[];
}

self.onmessage = (event: MessageEvent<ScanRequest>) => {
  const started = performance.now();
  const ids = tablesIn(event.data.text, event.data.tables);
  self.postMessage({ ids, ms: performance.now() - started });
};
