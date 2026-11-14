import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(HERE, '..');
const OUT = join(WEB, 'public/og.png');

function font(file: string): string {
  return `data:font/woff2;base64,${readFileSync(join(WEB, 'src/fonts', file)).toString('base64')}`;
}

const MARK = '<svg width="96" height="96" viewBox="0 0 24 24"><g fill="none" stroke-linecap="round"><path d="M3 16C7.5 16 8 8 12.5 8H21" stroke="#52DCC8" stroke-width="2.4"/><path d="M3 8C7.5 8 8 16 12.5 16H21" stroke="#1C1A25" stroke-width="5.6"/><path d="M3 8C7.5 8 8 16 12.5 16H21" stroke="#FF7EB6" stroke-width="2.4"/></g></svg>';

const BEFORE = `<span class="k">var</span> _0x3f2a=[<span class="s">'log'</span>,<span class="s">'aGVsbG8='</span>];
(<span class="k">function</span>(a,b){<span class="k">while</span>(--b)a.push(a.shift())})(_0x3f2a,<span class="n">0x1b3</span>);
console[_0x4e1c(<span class="n">0x0</span>)](_0x4e1c(<span class="n">0x1</span>));`;

const AFTER = `console.<span class="f">log</span>(<span class="s">"hello"</span>);`;

const PAGE = `<!doctype html><html><head><style>
@font-face { font-family: 'Spline Sans'; font-weight: 600; src: url('${font('spline-sans-latin-600-normal.woff2')}') format('woff2'); }
@font-face { font-family: 'Spline Sans'; font-weight: 400; src: url('${font('spline-sans-latin-400-normal.woff2')}') format('woff2'); }
@font-face { font-family: 'Spline Sans Mono'; font-weight: 400; src: url('${font('spline-sans-mono-latin-400-normal.woff2')}') format('woff2'); }
* { box-sizing: border-box; margin: 0; }
body { width: 1200px; height: 630px; overflow: hidden; background: #1C1A25; color: #EDEBF5; font-family: 'Spline Sans', sans-serif; }
.top { height: 6px; background: linear-gradient(90deg, #FF7EB6 0 50%, #52DCC8 50% 100%); }
.wrap { padding: 64px 72px 0; }
.brand { display: flex; align-items: center; gap: 22px; }
.name { font-size: 84px; font-weight: 600; letter-spacing: -0.03em; }
.tag { margin-top: 26px; font-size: 40px; font-weight: 600; letter-spacing: -0.01em; line-height: 1.2; }
.sub { margin-top: 14px; font-size: 25px; color: #A9A4C0; }
.code { margin-top: 40px; display: grid; grid-template-columns: 1.45fr auto 1fr; gap: 20px; align-items: center; }
pre { font-family: 'Spline Sans Mono', monospace; font-size: 17px; line-height: 1.6; padding: 18px 22px; border-radius: 12px; background: #16141E; border: 1px solid #353145; color: #E4E1EE; white-space: pre-wrap; }
.arrow { font-size: 48px; color: #52DCC8; }
.k { color: #C9A2FF; } .s { color: #A8E0A0; } .n { color: #FFB68C; } .f { color: #86CBFF; }
.url { position: absolute; right: 72px; bottom: 44px; font-size: 26px; color: #52DCC8; font-weight: 600; }
</style></head><body><div class="top"></div><div class="wrap">
<div class="brand">${MARK}<span class="name">untwist</span></div>
<p class="tag">Fast JavaScript deobfuscator and unbundler</p>
<p class="sub">obfuscator.io, minifiers, webpack, Rollup and Browserify. Runs in your browser.</p>
<div class="code"><pre>${BEFORE}</pre><span class="arrow">→</span><pre>${AFTER}</pre></div>
</div><span class="url">untwist.dev</span></body></html>`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
await page.setContent(PAGE);
await page.evaluate(() => document.fonts.ready);
await page.screenshot({ path: OUT, type: 'png' });
await browser.close();
console.log(`og-image: wrote ${OUT}`);
