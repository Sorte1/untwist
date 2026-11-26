#!/bin/bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
untwist="${UNTWIST:-$repo/target/release/untwist}"
table="${TABLE:-$repo/target/release/examples/fingerprint_table}"
probes="$here/work/probe"
pairs=()
for version in ${VERSIONS:-5.57.3 5.25.12 5.0.5}; do
  if [ "${PROBE:-app}" != none ]; then
    SVELTE="$version" PROBE="${PROBE:-app}" node "$here/harness.mjs" "$@"
  fi
  for dir in "$probes/$version"/*/; do
    [ -f "$dir/terser.js" ] || continue
    for minifier in terser esbuild; do
      for build in "$minifier" "$minifier-mangled"; do
        if [ "${PROBE:-app}" != none ] || [ -n "${REUNTWIST:-}" ] || [ ! -f "$dir$build.out/deobfuscated.js" ]; then
          "$untwist" "$dir$build.js" -o "$dir$build.out" -f > /dev/null 2>&1
        fi
      done
      pairs+=("$dir$minifier.out/deobfuscated.js=$dir$minifier-mangled.out/deobfuscated.js")
    done
    if [ -f "$dir/plain.js" ]; then
      if [ "${PROBE:-app}" != none ] || [ -n "${REUNTWIST:-}" ] || [ ! -f "${dir}plain.out/deobfuscated.js" ]; then
        "$untwist" "${dir}plain.js" -o "${dir}plain.out" -f > /dev/null 2>&1
      fi
      pairs+=("exports:${dir}plain.out/deobfuscated.js")
    fi
  done
done
"$table" "${pairs[@]}" > "$repo/crates/untwist-svelte/src/fingerprints.rs"
