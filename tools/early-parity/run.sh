#!/bin/bash
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
fixtures=$here/../../fixtures
version=$(node -p 'process.versions.node')
unicode=$(node -p 'process.versions.unicode')
work=$(mktemp -d)
for file in PropertyAliases.txt PropertyValueAliases.txt; do
  curl -sf "https://www.unicode.org/Public/$unicode.0/ucd/$file" -o "$work/$file"
done
python3 -I "$here/regex_rows.py" "$work" "$here/oxc_props.json" > "$work/regex.jsonl"
node "$here/regex_judge.js" "$work/regex.jsonl" "$fixtures/regex-parity/node-$version.tsv"
python3 -I "$here/redeclaration_rows.py" > "$work/redeclaration.rows"
node --experimental-vm-modules --no-warnings "$here/redeclaration_judge.mjs" "$work/redeclaration.rows" "$fixtures/redeclaration-parity/node-$version.tsv"
rm -r "$work"
