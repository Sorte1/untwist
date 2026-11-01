import { charDiff } from './lines';

self.onmessage = (event: MessageEvent<{ readonly a: string; readonly b: string }>) => {
  const started = performance.now();
  const result = charDiff(event.data.a, event.data.b);
  self.postMessage({ ...result, ms: performance.now() - started }, { transfer: [result.ranges.buffer, result.lineRanges.buffer] });
};
