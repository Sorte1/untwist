#!/bin/bash
set -u
export UNTWIST_TEST_SUPERVISOR=off
if [ $# -lt 1 ]; then
  echo "usage: $0 <binary> <args...>   (peak in-use heap, static-threshold max RSS, RssAnon/RssFile at that peak, raw max RSS; every run in process, UNTWIST_TEST_SUPERVISOR=off, so the measured process is the one that runs the pipeline; every run under MemoryMax=4G)" >&2
  exit 2
fi
C4='systemd-run --user --scope -q -p MemoryMax=4G -p MemorySwapMax=0 timeout 120'
STATIC='GLIBC_TUNABLES=glibc.malloc.mmap_threshold=131072'
DIR=${RSS_SPLIT_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}}
mkdir -p "$DIR"
F=$DIR/rss-split.massif

$C4 valgrind --tool=massif --depth=1 --peak-inaccuracy=0.5 --massif-out-file="$F" "$@" >/dev/null 2>&1
heap=$(grep -o 'mem_heap_B=[0-9]*' "$F" | cut -d= -f2 | sort -n | tail -1)
echo "peak in-use heap B: ${heap:-?}"

static=$($C4 env "$STATIC" /usr/bin/time -f %M "$@" 2>&1 >/dev/null | tail -1)
echo "static-threshold max RSS kB: $static"

$C4 env "$STATIC" "$@" >/dev/null 2>&1 &
t=$!
p=
while [ -z "$p" ] && kill -0 $t 2>/dev/null; do
  p=$(pgrep -P $t -n)
done
best=0
keep=
while [ -n "$p" ] && kill -0 $p 2>/dev/null; do
  s=$(grep -E '^(VmRSS|RssAnon|RssFile):' /proc/$p/status 2>/dev/null | tr -s ' \n' ' ')
  r=$(echo "$s" | cut -d' ' -f2)
  [ -n "$r" ] && [ "$r" -gt "$best" ] && best=$r && keep=$s
  sleep 0.01
done
wait $t 2>/dev/null
echo "at the static-threshold peak: $keep"

raw=$($C4 /usr/bin/time -f %M "$@" 2>&1 >/dev/null | tail -1)
echo "raw max RSS kB: $raw"
