import { expect, test } from './fixtures';

const LOADS = 5;

interface Timing {
  readonly load: number;
  readonly editors: number;
  readonly engine: number;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)] ?? Number.NaN;
}

test('the page loads cold in under a second, with the engine ready soon after', async ({ browser }) => {
  const timings: Timing[] = [];
  for (let index = 0; index < LOADS; index += 1) {
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto('./');
    await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
    timings.push(
      await page.evaluate(() => {
        const mark = (name: string) => performance.getEntriesByName(name)[0]?.startTime ?? Number.NaN;
        const navigation = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
        return { load: navigation.loadEventEnd, editors: mark('untwist:editors-painted'), engine: mark('untwist:engine-ready') };
      }),
    );
    await context.close();
  }
  const load = median(timings.map((t) => t.load));
  const editors = median(timings.map((t) => t.editors));
  const engine = median(timings.map((t) => t.engine));
  test.info().annotations.push({ type: 'cold load', description: `load ${load.toFixed(0)} ms, editors ${editors.toFixed(0)} ms, engine ${engine.toFixed(0)} ms` });
  expect(load).toBeLessThanOrEqual(1000);
  expect(editors).toBeLessThanOrEqual(1000);
  expect(engine).toBeLessThanOrEqual(1500);
});
