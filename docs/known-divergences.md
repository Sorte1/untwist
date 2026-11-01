# Known divergences from the reference deobfuscator

The reference deobfuscator is the JavaScript tool Untwist reimplements (see `NOTICE`). Untwist's
output is verified against it by the `cargo xtask diff-*` harnesses, which run Untwist in a
conservative comparison mode: an internal test configuration, not exposed by the CLI or the
library, that runs only the passes the reference deobfuscator has, in its order. The items in these
files are deliberate differences, not bugs. One file per area:

- [Unminify](known-divergences/unminify.md)
- [Unpack](known-divergences/unpack.md), including the further bundlers (esbuild, Bun, Rollup/Vite,
  SystemJS, AMD/UMD, Parcel, Metro, Turbopack, Closure) and the multi-file premise: several
  input files are taken to be the whole program, since unpack never sees their names
- [Evaluator](known-divergences/evaluator.md)
- [Deobfuscate](known-divergences/deobfuscate.md)
- [JSX](known-divergences/jsx.md)
- [Plugins](known-divergences/plugins.md)
- [Rename](known-divergences/rename.md)
- [Codegen](known-divergences/codegen.md)
- [Limits](known-divergences/limits.md)
- [Untranspile](known-divergences/untranspile.md)
- [Layers](known-divergences/layers.md)
