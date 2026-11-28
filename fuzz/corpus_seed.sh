set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
corpus="$root/fuzz/corpus"
sources() {
  find "$root/fixtures" "$root/crates/untwist-svelte/tests/fixtures" "$root/crates/untwist-solid/tests/fixtures" \
    -path "$root/fixtures/bla" -prune -o \
    \( -name '*.js' -o -name '*.mjs' -o -name '*.cjs' \) -size -512k -type f -print
}
for target in pipeline passes evaluator; do
  mkdir -p "$corpus/$target"
  sources | while read -r file; do
    cp "$file" "$corpus/$target/$(printf '%s' "$file" | sha1sum | cut -c1-16)"
  done
done
mkdir -p "$corpus/options"
sources | while read -r file; do
  case "$file" in
    *untwist-svelte*) flags='\002\000\000' ;;
    *untwist-solid*) flags='\004\000\000' ;;
    *.cjs) flags='\060\000\000' ;;
    *.mjs) flags='\020\000\000' ;;
    *js-confuser*|*encoders*|*layers*) flags='\001\000\000' ;;
    *library-collection*) flags='\010\000\000' ;;
    *) flags="$(printf '\\%03o\\%03o\\000' "$(( $(printf '%s' "$file" | cksum | cut -d' ' -f1) % 256 ))" "$(( $(printf '%s' "$file" | wc -c) % 256 ))")" ;;
  esac
  { printf "$flags"; cat "$file"; } > "$corpus/options/$(printf '%s' "$file" | sha1sum | cut -c1-16)"
done
mkdir -p "$corpus/libraries"
collection="$root/fixtures/library-collection"
tables="$(for shard in "$collection"/shards/*.utl; do gzip -dc < "$shard"; printf '\n&&\n'; done)"
keys="$(sed -n 's/^    "\(.*\)",\{0,1\}$/\1/p' "$collection/collection.json")"
bloom="$(ls "$collection"/shards/*.bloom)"
for input in "$collection"/inputs/*.js "$collection"/canary/*.js "$root"/fixtures/libraries/*.js; do
  [ -f "$input" ] || continue
  name="$(printf '%s' "$input" | sha1sum | cut -c1-16)"
  for flags in '\000' '\001' '\004' '\020'; do
    {
      printf "$flags"
      cat "$input"
      printf '\n%%%%\n%s\n%%%%\n%s\n%%%%\n' "$tables" "$keys"
      tail -c +9 "$bloom"
    } > "$corpus/libraries/$name-$(printf "$flags" | od -An -tu1 | tr -d ' ')"
  done
done
