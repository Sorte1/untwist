#!/bin/bash
set -u
if [ $# -lt 2 ]; then
  printf 'usage: %s <rounds> <label=binary>... -- <untwist args...>   (the statm-before-large-free peak probe: the bench binary from `cargo bench --no-run -p untwist-cli --features peak-probe --bench peak`, target/release/deps/peak-*; five malloc policies, each binary under UNTWIST_TEST_CHECKSUM=compiled and walker__; every run in process and under MemoryMax=4G)\n' "$0" >&2
  exit 2
fi
C4='systemd-run --user --scope -q -p MemoryMax=4G -p MemorySwapMax=0 timeout 300'
rounds=$1; shift
binaries=()
while [ $# -gt 0 ] && [ "$1" != "--" ]; do binaries+=("$1"); shift; done
shift
for round in $(seq 1 "$rounds"); do
  for policy in aslr empty toppad trim static; do
    case $policy in
      aslr) tune=""; pre="";;
      empty) tune="GLIBC_TUNABLES="; pre="setarch -R";;
      toppad) tune="GLIBC_TUNABLES=glibc.malloc.top_pad=16777216"; pre="setarch -R";;
      trim) tune="GLIBC_TUNABLES=glibc.malloc.trim_threshold=1073741824"; pre="setarch -R";;
      static) tune="GLIBC_TUNABLES=glibc.malloc.mmap_threshold=131072"; pre="setarch -R";;
    esac
    for pair in "${binaries[@]}"; do
      label=${pair%%=*}; binary=${pair#*=}
      for switch in ${PEAK_SWITCHES:-compiled walker__}; do
        line=$($C4 env UNTWIST_TEST_SUPERVISOR=off UNTWIST_TEST_CHECKSUM=$switch $tune $pre "$binary" "$@" 2>&1 >/dev/null | grep '^peak ')
        echo "$round $policy $label $switch $line"
      done
    done
  done
done
