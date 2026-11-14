import type { Locator, Page } from '@playwright/test';
import { expect, openApp, runAndWait, test } from './fixtures';

const FUNCTIONS = 3000;
const WIDE_EVERY = 40;
const WIDE_ITEMS = 3000;
const SWEEP_STEPS = 60;
const DRAG_STEPS = 60;
const DRAG_TOLERANCE = 0.02;

test.use({ launchOptions: { ignoreDefaultArgs: ['--hide-scrollbars'] } });

function wideArray(index: number): string {
  return Array.from({ length: WIDE_ITEMS }, (_, item) => `"w${index}_${item}"`).join(',');
}

function bigInput(): string {
  const parts: string[] = [];
  for (let index = 0; index < FUNCTIONS; index++) {
    const table = index % WIDE_EVERY === 0 ? `var t = [${wideArray(index)}];` : 'var t = [a, b];';
    parts.push(`function f${index}(a, b) { ${table} if (a > b) { return t[0] + ${index}; } var c = a * b; return c + t.length; }`);
  }
  return parts.join('\n');
}

function frame(page: Page): Promise<void> {
  return page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
}

type Box = { x: number; y: number; right: number; paneRight: number; top: number; scrollHeight: number; clientHeight: number; clientWidth: number; offsetWidth: number };

function box(scroller: Locator): Promise<Box> {
  return scroller.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    const pane = (element.getRootNode() as ShadowRoot).host.closest('.pane');
    return {
      x: rect.x,
      y: rect.y,
      right: rect.right,
      paneRight: pane?.getBoundingClientRect().right ?? 0,
      top: element.scrollTop,
      scrollHeight: element.scrollHeight,
      clientHeight: element.clientHeight,
      clientWidth: element.clientWidth,
      offsetWidth: (element as HTMLElement).offsetWidth,
    };
  });
}

function scrolled(at: Box): number {
  return at.top / (at.scrollHeight - at.clientHeight);
}

async function showBigOutput(page: Page): Promise<Locator> {
  await openApp(page);
  await page.locator('#open-file-input').setInputFiles({ name: 'big.js', mimeType: 'text/javascript', buffer: Buffer.from(bigInput()) });
  await runAndWait(page);
  const scroller = page.locator('#output-editor .cm-scroller');
  await expect.poll(async () => (await box(scroller)).scrollHeight).toBeGreaterThan(100_000);
  return scroller;
}

test('the output scrollbar stays in the pane past lines wider than the pane', async ({ page }) => {
  const scroller = await showBigOutput(page);
  const outside: string[] = [];
  for (let step = 0; step <= SWEEP_STEPS; step++) {
    await scroller.evaluate((element, share) => {
      element.scrollTop = share * (element.scrollHeight - element.clientHeight);
    }, step / SWEEP_STEPS);
    await frame(page);
    const at = await box(scroller);
    if (at.right > at.paneRight + 1) {
      outside.push(`${step}/${SWEEP_STEPS}: ${Math.round(at.right - at.paneRight)} px`);
    }
  }
  expect(outside).toEqual([]);
});

test('dragging the output scrollbar thumb follows the pointer to the end', async ({ page, browserName }) => {
  test.skip(browserName !== 'chromium', 'only Chromium drags a native scrollbar from synthetic mouse events');
  const scroller = await showBigOutput(page);
  const start = await box(scroller);
  const bar = start.offsetWidth - start.clientWidth;
  expect(bar).toBeGreaterThan(0);
  const track = start.clientHeight - 2 * bar;
  const thumb = Math.max(20, (start.clientHeight / start.scrollHeight) * track);
  const x = start.x + start.clientWidth + bar / 2;
  const pointerAt = (share: number) => start.y + bar + share * (track - thumb) + thumb / 2;
  await page.mouse.move(x, pointerAt(0));
  await page.mouse.down();
  const lags: string[] = [];
  for (let step = 1; step <= DRAG_STEPS; step++) {
    const share = step / DRAG_STEPS;
    await page.mouse.move(x, pointerAt(share));
    await frame(page);
    const got = scrolled(await box(scroller));
    if (Math.abs(got - share) > DRAG_TOLERANCE) {
      lags.push(`pointer ${share.toFixed(3)}, scroll ${got.toFixed(3)}`);
    }
  }
  await page.mouse.up();
  expect(lags).toEqual([]);
  expect(scrolled(await box(scroller))).toBeGreaterThan(0.99);
});
