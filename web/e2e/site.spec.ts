import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Page } from '@playwright/test';
import { expect, GITHUB_API, openApp, openSettings, repoPath, runAndWait, STUB_STARS, test } from './fixtures';

const DIST = resolve(repoPath('web'), 'dist');
const CANONICAL = 'https://untwist.dev/';

async function meta(page: Page, selector: string): Promise<string | null> {
  return page.locator(`head ${selector}`).getAttribute('content');
}

function headerRules(text: string): Array<{ readonly pattern: string; readonly headers: readonly string[] }> {
  const rules: Array<{ pattern: string; headers: string[] }> = [];
  for (const line of text.split('\n')) {
    if (line.trim() === '' || line.trim().startsWith('#')) {
      continue;
    }
    if (/^\s/.test(line)) {
      rules.at(-1)?.headers.push(line.trim().toLowerCase());
    } else {
      rules.push({ pattern: line.trim(), headers: [] });
    }
  }
  return rules;
}

function matches(pattern: string, url: string): boolean {
  const absolute = pattern.startsWith('https://') ? pattern : `https://:host${pattern}`;
  const source = absolute
    .split(/(:[a-z]+|\*)/)
    .map((part) => (part === '*' ? '.*' : part.startsWith(':') ? '[^./]+' : part.replace(/[.?+^$()[\]{}|\\/]/g, '\\$&')))
    .join('');
  return new RegExp(`^${source}$`).test(url);
}

function robotsFor(rules: ReturnType<typeof headerRules>, url: string): string[] {
  return rules.filter((rule) => matches(rule.pattern, url)).flatMap((rule) => rule.headers.filter((header) => header.startsWith('x-robots-tag:')));
}

test('the page carries a title, description, canonical URL, social cards and structured data', async ({ page }) => {
  await openApp(page);
  await expect(page).toHaveTitle(/JavaScript deobfuscator and unbundler/);
  expect(await meta(page, 'meta[name="description"]')).toMatch(/fast JavaScript deobfuscator and unbundler/);
  expect(await page.locator('head link[rel="canonical"]').getAttribute('href')).toBe(CANONICAL);
  expect(await meta(page, 'meta[property="og:url"]')).toBe(CANONICAL);
  expect(await meta(page, 'meta[property="og:title"]')).toMatch(/untwist/);
  expect(await meta(page, 'meta[property="og:image"]')).toBe('https://untwist.dev/og.png');
  expect(await meta(page, 'meta[name="twitter:card"]')).toBe('summary_large_image');
  expect(await page.locator('head meta[name="theme-color"]').count()).toBe(2);
  expect(await page.locator('head meta[name="robots"]').count()).toBe(0);
  const data = JSON.parse((await page.locator('head script[type="application/ld+json"]').textContent()) ?? '') as Record<string, unknown>;
  expect(data['@type']).toBe('WebApplication');
  expect(data['url']).toBe(CANONICAL);
  expect(data['applicationCategory']).toBe('DeveloperApplication');
});

test('the about text is real, visible HTML until the first run replaces it', async ({ page }) => {
  await openApp(page);
  const intro = page.locator('#intro');
  await expect(intro).toBeVisible();
  await expect(page.locator('h1')).toHaveCount(1);
  await expect(page.locator('#intro h1')).toHaveText('JavaScript deobfuscator and unbundler');
  await expect(intro).toContainText('obfuscator.io');
  await expect(intro).toContainText('webpack');
  await expect(intro).toContainText('cargo install untwist-cli');
  await expect(intro).toContainText('never uploaded');
  await page.locator('#examples').click();
  await page.locator('#examples-menu [role=option]').first().click();
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 120_000 });
  await expect(intro).toBeHidden();
});

test('robots.txt, the sitemap and the social image are served', async ({ page, request }) => {
  await openApp(page);
  const robots = await request.get('robots.txt');
  expect(robots.ok()).toBe(true);
  expect(await robots.text()).toContain('Sitemap: https://untwist.dev/sitemap.xml');
  const sitemap = await request.get('sitemap.xml');
  expect(await sitemap.text()).toContain(`<loc>${CANONICAL}</loc>`);
  const image = readFileSync(resolve(DIST, 'og.png'));
  expect(image.subarray(1, 4).toString('latin1')).toBe('PNG');
  expect([image.readUInt32BE(16), image.readUInt32BE(20)]).toEqual([1200, 630]);
});

test('preview deployments on pages.dev get noindex and untwist.dev does not', () => {
  const rules = headerRules(readFileSync(resolve(DIST, '_headers'), 'utf8'));
  for (const preview of ['https://untwist-d8y.pages.dev/', 'https://main-latest.untwist-d8y.pages.dev/', 'https://3f2a1b9c.untwist-d8y.pages.dev/assets/x.js']) {
    expect(robotsFor(rules, preview), preview).toEqual(['x-robots-tag: noindex']);
  }
  for (const production of ['https://untwist.dev/', 'https://untwist.dev/robots.txt']) {
    expect(robotsFor(rules, production), production).toEqual([]);
  }
  expect(readFileSync(resolve(DIST, 'index.html'), 'utf8')).not.toContain('noindex');
});

test('the GitHub link sits top right with the star count from the API', async ({ page }) => {
  await openApp(page);
  const link = page.locator('#github-link');
  await expect(link).toHaveAttribute('href', 'https://github.com/Sorte1/untwist');
  await expect(page.locator('#github-stars')).toHaveText(String(STUB_STARS));
  await expect(link).toHaveAttribute('aria-label', `untwist on GitHub, ${STUB_STARS} stars`);
  const box = await link.boundingBox();
  const width = page.viewportSize()?.width ?? 0;
  expect(box !== null && box.x + box.width > width - 40 && box.y < 48).toBe(true);
});

test('the star count is cached, so a reload asks the API nothing', async ({ page, context }) => {
  let asked = 0;
  await context.route(GITHUB_API, (route) => {
    asked += 1;
    return route.fulfill({ json: { stargazers_count: 12_345 } });
  });
  await openApp(page);
  await expect(page.locator('#github-stars')).toHaveText('12k');
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator('#github-stars')).toHaveText('12k');
  expect(asked).toBe(1);
});

test('a rate-limited API leaves the link without a count and is not asked again on reload', async ({ page, context, pageErrors }) => {
  let asked = 0;
  await context.route(GITHUB_API, (route) => {
    asked += 1;
    return route.fulfill({ status: 403, json: { message: 'API rate limit exceeded' } });
  });
  await openApp(page);
  await expect.poll(() => page.evaluate(() => localStorage.getItem('untwist.githubStars'))).toMatch(/"count":null/);
  await expect(page.locator('#github-stars')).toBeHidden();
  await expect(page.locator('#github-link')).toHaveAttribute('aria-label', 'untwist on GitHub');
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await page.evaluate(() => new Promise((done) => setTimeout(done, 1500)));
  expect(asked).toBe(1);
  pageErrors.splice(0, pageErrors.length, ...pageErrors.filter((error) => !/403|Failed to load resource/.test(error)));
});

test('the GitHub link stays usable at phone width', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 800 });
  await openApp(page);
  const box = await page.locator('#github-link').boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThanOrEqual(30);
  expect(box!.x + box!.width).toBeLessThanOrEqual(390);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
});

async function codeBackground(page: Page): Promise<string> {
  return page.evaluate(() => {
    const host = document.querySelector('#output-editor .editor-host');
    const editor = host?.shadowRoot?.querySelector('.cm-editor');
    return editor === null || editor === undefined ? '' : getComputedStyle(editor).backgroundColor;
  });
}

test.describe('editor themes in dark mode', () => {
  test.use({ colorScheme: 'dark' });

  test('dark mode defaults to GitHub Dark, and a picked theme applies at once and survives a reload', async ({ page }) => {
    await openApp(page);
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(page.locator('html')).toHaveAttribute('data-editor', 'github-dark');
    expect(await codeBackground(page)).toBe('rgb(13, 17, 23)');
    await openSettings(page);
    await page.locator('#editor-theme-dark [data-pick="one-dark"]').click();
    await expect(page.locator('html')).toHaveAttribute('data-editor', 'one-dark');
    expect(await codeBackground(page)).toBe('rgb(40, 44, 52)');
    await page.reload();
    await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
    await expect(page.locator('html')).toHaveAttribute('data-editor', 'one-dark');
    await expect(page.locator('#editor-theme-dark [data-pick="one-dark"]')).toHaveAttribute('aria-pressed', 'true');
  });

  test('the built-in palette is still there, and picking a light theme switches to light mode', async ({ page }) => {
    await openApp(page);
    await openSettings(page);
    await page.locator('#editor-theme-dark [data-pick="untwist"]').click();
    await expect(page.locator('html')).not.toHaveAttribute('data-editor', /.*/);
    expect(await codeBackground(page)).toBe('rgb(22, 20, 30)');
    await page.locator('#editor-theme-light [data-pick="solarized-light"]').click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    await expect(page.locator('html')).toHaveAttribute('data-editor', 'solarized-light');
    await page.locator('#theme').click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(page.locator('html')).not.toHaveAttribute('data-editor', /.*/);
  });
});

test.describe('editor themes in light mode', () => {
  test.use({ colorScheme: 'light' });

  test('light mode defaults to GitHub Light', async ({ page }) => {
    await openApp(page);
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    await expect(page.locator('html')).toHaveAttribute('data-editor', 'github-light');
    expect(await codeBackground(page)).toBe('rgb(255, 255, 255)');
  });
});
