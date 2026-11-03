# Deploying Untwist

Untwist is a static site. `npm run build` in `web/` builds the wasm engine
(`cargo`, `wasm-bindgen` 0.2.129, `wasm-opt` from binaryen 116) and the site into `web/dist/`.
Every asset name except `index.html`, `_headers` and the font licences carries a content hash.

## Cloudflare Pages through GitHub Actions

`.github/workflows/deploy-web.yml` runs on a push that touches `web/**`, `crates/**`,
`Cargo.toml`, `Cargo.lock` or the workflow itself, and on a manual run. It builds the site and
runs `wrangler pages deploy web/dist --project-name=untwist --branch=<branch>`.

- **Secrets.** `CLOUDFLARE_API_TOKEN` (Account › Cloudflare Pages › Edit) and
  `CLOUDFLARE_ACCOUNT_ID`, as repository secrets. Without them the workflow builds and skips
  the deploy. Only the deploy step reads them; the job's token is `contents: read`.
- **Project.** A Pages project named `untwist` with production branch `master`. A push to
  `master` deploys to production; a push to any other branch gets its own preview URL.
- **Domain.** `untwist.dev` is attached to the project in the Cloudflare dashboard.

## Headers

`web/public/_headers` is copied into `dist/` and applied by Pages:

- `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`
  everywhere, so a future threaded build can use `SharedArrayBuffer`. Everything the page
  loads is same-origin: the fonts are self-hosted and nothing is fetched elsewhere.
- `index.html` is `no-cache`; `/assets/*` is cached for a year as immutable.
- The engine sits in `/assets/wasm/` and is served as `application/wasm`, which
  `WebAssembly.compileStreaming` needs.

- `sw.js` is `no-cache`, so a deploy reaches the browsers on their next visit.

The page also carries its own Content-Security-Policy `<meta>` (see `vite.config.ts`).

## Offline

The build writes `sw.js`, a service worker that precaches the page, every hashed asset and the
wasm engine under a cache named after the build. The page registers it once the engine is
ready and the browser is idle. It serves those files from the cache first, with the headers
they were fetched with, so an offline page keeps COOP/COEP. A new deploy installs beside the
old one and waits; the page then offers a Reload, which switches to it and drops the old cache.
`e2e/offline.spec.ts` serves `dist/` with the rules in `_headers` to check all of this.

## By hand

```sh
cd web
npm ci
npm run build
npx wrangler pages deploy dist --project-name=untwist --branch=master
```
