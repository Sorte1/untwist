import { expect, openApp, paste, runAndWait, test } from './fixtures';

for (const width of [1440, 1280, 1024]) {
  test(`the output pane head keeps its actions on one row at ${width} px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await openApp(page);
    await paste(page, 'var a = !0;');
    await runAndWait(page);
    const tops = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('#output-pane .pane-head > *')]
        .filter((element) => element.offsetParent !== null)
        .map((element) => Math.round(element.getBoundingClientRect().top + element.getBoundingClientRect().height / 2)),
    );
    expect(Math.max(...tops) - Math.min(...tops)).toBeLessThanOrEqual(4);
    await expect(page.locator('#compare-modes')).toHaveAccessibleName('Compare with Unsafe');
    await expect(page.locator('#download')).toHaveAccessibleName(/Download/);
  });
}
