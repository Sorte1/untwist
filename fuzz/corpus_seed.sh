set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
for target in pipeline passes evaluator; do
  mkdir -p "$root/fuzz/corpus/$target"
  find "$root/fixtures" -name '*.js' -size -512k -exec cp {} "$root/fuzz/corpus/$target/" \;
done
