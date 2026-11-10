# Deploying Untwist

Untwist is a static site. `npm run build` in `web/` builds the wasm engine
(`cargo`, `wasm-bindgen` 0.2.129, `wasm-opt` from binaryen 116) and the site into `web/dist/`.
Every asset name except `index.html`, `_headers` and the font licences carries a content hash.

## Cloudflare Pages through GitHub Actions

`.github/workflows/deploy-web.yml` builds the site and runs
`wrangler pages deploy web/dist --project-name=untwist --branch=<branch>`.

- **Production.** Only a release deploys to production: pushing a `vX.Y.Z` tag runs
  `.github/workflows/release.yml`, which calls this workflow with `production: true` and
  deploys with `--branch=main`. To redeploy a release by hand, run "Deploy web" from the
  Actions tab on the release tag with "production" ticked. See `RELEASING.md`.
- **Previews.** A push to any branch other than `main` that touches `web/**`, `crates/**`,
  `Cargo.toml`, `Cargo.lock` or the workflow gets its own preview URL, and the link is posted
  on the branch's open pull request. A push to `main` deploys to the `main-latest` preview.
  A tag push deploys nothing by itself; the release workflow does.
- **Cache.** Previews keep a cargo and wasm target cache. The release and a production
  deploy build without it, so nothing an earlier build left behind reaches npm or the site.
- **Secrets.** `CLOUDFLARE_API_TOKEN` (Account › Cloudflare Pages › Edit) and
  `CLOUDFLARE_ACCOUNT_ID`, as repository secrets. Without them a preview build skips the
  deploy and a production deploy fails. Only the deploy step reads them.
- **Project.** A Pages project named `untwist` with production branch `main`.
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

- `Content-Security-Policy` is sent with every file, so it binds the workers and the service
  worker as well as the page. It forbids framing (`frame-ancestors 'none'`, with
  `X-Frame-Options: DENY` for older browsers), `<base>`, form targets and plugins. The build
  copies the same policy, without `frame-ancestors`, into a `<meta>` tag, so `vite preview` and
  any host that ignores `_headers` still apply it.

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
npx wrangler pages deploy dist --project-name=untwist --branch=main
```
