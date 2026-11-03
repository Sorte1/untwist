# Untwist playground

The playground at [untwist.dev](https://untwist.dev) runs Untwist in the browser. Paste, drop or
open a script, or pick an example, and press Run. Ctrl+K (⌘K) opens a command palette for the
examples, settings, modules and actions. The WebAssembly build (`crates/untwist-wasm`)
works in a Web Worker and reports each stage it reaches, so a long run shows where it is. You
can read, edit and download the output and the unpacked modules, compare before and after, and
read the diagnostics and the run report. For the same flags the output is
byte for byte the `untwist` CLI's.

## Build

Requirements: the Rust toolchain from `rust-toolchain.toml` with the `wasm32-unknown-unknown`
target, `wasm-bindgen` 0.2.129 (`cargo install wasm-bindgen-cli --version 0.2.129 --locked`),
`wasm-opt` from binaryen 116, and Node 22.

```sh
cd web
npm ci
npm run build      # wasm module, then vite build into dist/
npm run preview    # serves dist/ on http://localhost:4173
npm run dev        # Vite dev server
```

`UNTWIST_WASM_PKG=<dir>` reuses an already built wasm package. Deployment is described in
[`DEPLOY.md`](DEPLOY.md).

## Test

```sh
npm run typecheck   # tsc over src/, e2e/ and scripts/
npm test            # vitest
npm run e2e         # Playwright; needs a built dist/ and the native CLI
npm run size        # byte budgets for JS, CSS, fonts and wasm
```

The end-to-end specs compare the page with the native CLI found through `UNTWIST_BIN` (default
`../target/release/untwist`); `UNTWIST_PORT` moves the preview server off 4173.

## Privacy

The input never leaves the browser. The page loads only its own static assets under a strict CSP,
has no analytics and self-hosts its fonts. Settings live in the URL hash (`#unsafe=1&rename=unique`),
which never reaches a server. Copy link on the input adds the input, deflated into the hash as
`code=`; an input whose link text would pass 32 KiB gets a settings-only link. Opening such a link
loads the input and waits for Run. The output never goes into a link. The Recent menu keeps the
last ten inputs you ran (up to 512 KB each) with their settings in this browser's `localStorage`;
Clear removes them, and without storage they last for the tab only.

## Offline

After the first visit a service worker keeps the page, its assets and the engine, so the
playground opens and runs without a network. A new version waits until you press Reload in
the prompt. See [`DEPLOY.md`](DEPLOY.md#offline).

## Limits

| What | Limit | Above it |
|---|---|---|
| Input editor | 1 MiB, longest line 20,000 characters | the text still runs; the editor shows a read-only preview |
| Diff | 2 MiB or 50,000 lines per side | the diff is off, with a note |
| Language features | 12 MiB of output | off, with a note |
| Run time | none | Cancel stops the worker |

## Layout

- `src/engine/`: the worker and its client. `src/settings/`: the URL hash and the CLI flags
  (`flags.json` is shared with a cargo test).
- `src/ui/`: panes, options, module tree, diff, diagnostics and details. `src/lang/`: editor
  language features.
- `e2e/`: Playwright specs. `e2e/design/Main.dc.html` is the design board the theme screenshots
  are compared against.
