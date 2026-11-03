import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Page } from '@playwright/test';
import { REPO } from './fixtures';

export interface Rect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

const FONTS: ReadonlyArray<readonly [string, string, number]> = [
  ['Spline Sans', 'spline-sans-latin-400-normal.woff2', 400],
  ['Spline Sans', 'spline-sans-latin-500-normal.woff2', 500],
  ['Spline Sans', 'spline-sans-latin-600-normal.woff2', 600],
  ['Spline Sans Mono', 'spline-sans-mono-latin-400-normal.woff2', 400],
  ['Spline Sans Mono', 'spline-sans-mono-latin-500-normal.woff2', 500],
];

function fontFaces(): string {
  return FONTS.map(([family, file, weight]) => {
    const data = readFileSync(join(REPO, 'web/src/fonts', file)).toString('base64');
    return `@font-face{font-family:'${family}';font-weight:${weight};src:url(data:font/woff2;base64,${data}) format('woff2')}`;
  }).join('\n');
}

function renderInPage({ html, theme }: { html: string; theme: string }): void {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  const script = doc.querySelector('script[data-dc-script]')?.textContent ?? '';
  class DCLogic {
    props: Record<string, unknown>;
    state: Record<string, unknown> = {};
    constructor(props: Record<string, unknown>) {
      this.props = props;
    }
    setState(): void {}
  }
  const Component = new Function('DCLogic', `${script}\nreturn Component;`)(DCLogic);
  const vals = new Component({ theme }).renderVals();
  const evaluate = (expr: string, scope: Record<string, unknown>): unknown =>
    new Function(...Object.keys(scope), `return (${expr});`)(...Object.values(scope));
  const fill = (text: string, scope: Record<string, unknown>): string =>
    text.replace(/\{\{([^}]*)\}\}/g, (_, expr: string) => String(evaluate(expr, scope)));
  const only = (text: string, scope: Record<string, unknown>): unknown => {
    const match = /^\{\{([^}]*)\}\}$/.exec(text.trim());
    return match === null ? text : evaluate(match[1] ?? '', scope);
  };
  const render = (node: Node, scope: Record<string, unknown>): Node[] => {
    if (node.nodeType === Node.TEXT_NODE) {
      return [document.createTextNode(fill(node.textContent ?? '', scope))];
    }
    if (!(node instanceof Element)) {
      return [];
    }
    const tag = node.tagName.toLowerCase();
    const children = (inner: Record<string, unknown>) => [...node.childNodes].flatMap((child) => render(child, inner));
    if (tag === 'helmet') {
      return [];
    }
    if (tag === 'sc-if') {
      return only(node.getAttribute('value') ?? '', scope) ? children(scope) : [];
    }
    if (tag === 'sc-for') {
      const list = only(node.getAttribute('list') ?? '', scope) as unknown[];
      const as = node.getAttribute('as') ?? 'item';
      return list.flatMap((item) => children({ ...scope, [as]: item }));
    }
    const element = document.createElementNS(node.namespaceURI, node.localName);
    for (const attribute of [...node.attributes]) {
      if (attribute.name.startsWith('on') || attribute.name.startsWith('hint-')) {
        continue;
      }
      const value = only(attribute.value, scope);
      if (value === false) {
        continue;
      }
      element.setAttribute(attribute.name, value === true ? '' : fill(attribute.value, scope));
    }
    element.append(...children(scope));
    return [element];
  };
  const root = doc.querySelector('x-dc');
  document.body.replaceChildren(...(root === null ? [] : [...root.childNodes].flatMap((child) => render(child, vals))));
}

export async function showDesign(page: Page, theme: 'dark' | 'light'): Promise<void> {
  const html = readFileSync(join(REPO, 'web/e2e/design/Main.dc.html'), 'utf8');
  await page.setContent(`<!doctype html><html><head><style>body{margin:0}${fontFaces()}</style></head><body></body></html>`);
  await page.evaluate(renderInPage, { html, theme });
  await page.evaluate(() => document.fonts.ready.then(() => undefined));
}

function compareInPage({ a, b, masks }: { a: string; b: string; masks: Rect[] }): Promise<number> {
  const load = (src: string) =>
    new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = reject;
      image.src = src;
    });
  const pixels = (image: HTMLImageElement) => {
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    if (context === null) {
      throw new Error('no 2d context');
    }
    context.drawImage(image, 0, 0);
    return context.getImageData(0, 0, image.width, image.height);
  };
  return Promise.all([load(a), load(b)]).then(([first, second]) => {
    const x = pixels(first);
    const y = pixels(second);
    const masked = (px: number, py: number) =>
      masks.some((m) => px >= m.x && px < m.x + m.width && py >= m.y && py < m.y + m.height);
    let differ = 0;
    for (let py = 0; py < x.height; py += 1) {
      for (let px = 0; px < x.width; px += 1) {
        const at = (py * x.width + px) * 4;
        const delta = Math.max(
          Math.abs((x.data[at] ?? 0) - (y.data[at] ?? 0)),
          Math.abs((x.data[at + 1] ?? 0) - (y.data[at + 1] ?? 0)),
          Math.abs((x.data[at + 2] ?? 0) - (y.data[at + 2] ?? 0)),
        );
        if (delta > 32 && !masked(px, py)) {
          differ += 1;
        }
      }
    }
    return differ / (x.width * x.height);
  });
}

export async function differingShare(page: Page, a: Buffer, b: Buffer, masks: Rect[]): Promise<number> {
  const url = (png: Buffer) => `data:image/png;base64,${png.toString('base64')}`;
  return page.evaluate(compareInPage, { a: url(a), b: url(b), masks });
}
