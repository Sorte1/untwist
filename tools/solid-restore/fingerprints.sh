#!/bin/bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
untwist="${UNTWIST:-$repo/target/release/untwist}"
table="${TABLE:-$repo/target/release/examples/solid_fingerprint_table}"
probes="$here/work/probe"
pairs=()
if [ "${PROBE:-singles}" != none ]; then
  PROBE="${PROBE:-singles}" untwist-run-slot timeout 1800 node "$here/harness.mjs" "$@"
fi
for dir in "$probes"/*/; do
  [ -f "$dir/terser.js" ] || continue
  for minifier in terser esbuild; do
    for build in "$minifier" "$minifier-mangled"; do
      if [ "${PROBE:-singles}" != none ] || [ ! -f "$dir$build.out/deobfuscated.js" ]; then
        "$untwist" "$dir$build.js" -o "$dir$build.out" -f > /dev/null 2>&1
      fi
    done
    pairs+=("$dir$minifier.out/deobfuscated.js=$dir$minifier-mangled.out/deobfuscated.js")
  done
done
"$table" "${pairs[@]}" > "$repo/crates/untwist-solid/src/fingerprints.rs"
