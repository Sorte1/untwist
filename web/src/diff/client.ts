import DiffWorker from './diff.worker.ts?worker';
import type { CharDiff } from './lines';

export interface WorkerDiff extends CharDiff {
  readonly ms: number;
}

export interface PendingDiff {
  readonly result: Promise<WorkerDiff>;
  cancel(): void;
}

export function diffInWorker(a: string, b: string): PendingDiff {
  const worker = new DiffWorker();
  const result = new Promise<WorkerDiff>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<WorkerDiff>) => resolve(event.data);
    worker.onerror = (event) => reject(new Error(event.message));
    worker.postMessage({ a, b });
  });
  const done = () => worker.terminate();
  result.then(done, done);
  return { result, cancel: done };
}
