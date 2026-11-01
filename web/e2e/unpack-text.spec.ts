import { expect, openApp, paste, readRepo, runAndWait, test, skipWithoutGame } from './fixtures';

for (const rename of ['simple', 'none'] as const) {
  test(`game.js with rename ${rename} finishes Done with the unpack re-parse, not stopped by unpack-text`, async ({ page }) => {
    skipWithoutGame();
    await openApp(page, rename === 'none' ? '' : `#rename=${rename}`);
    await paste(page, readRepo('fixtures/real-world/game.js'));
    await runAndWait(page);
    await expect(page.locator('#status-label')).toHaveText('Done');
    await expect(page.locator('#limits')).toHaveText('none fired');
    await expect(page.locator('#skipped-chip')).toBeHidden();
    await expect(page.locator('#diag-list')).not.toContainText('nests too deeply to parse again');
  });
}
