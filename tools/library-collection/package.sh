#!/bin/bash
set -euo pipefail
if [ $# -ne 2 ]; then
  echo "usage: package.sh <collection-dir> <out-dir>" >&2
  exit 2
fi
collection=$1
out=$2
index="$collection/collection.json"
version=$(node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).version)' "$index")
if ! printf '%s' "$version" | grep -Eq '^[A-Za-z0-9._-]+$'; then
  echo "collection version $version has characters a file name should not hold" >&2
  exit 1
fi
node -e '
const fs = require("fs"), path = require("path"), crypto = require("crypto");
const dir = process.argv[1];
const index = JSON.parse(fs.readFileSync(path.join(dir, "collection.json"), "utf8"));
const files = index.packages.flatMap((p) => p.shards).concat(index.filters ? [index.filters] : []);
for (const file of files) {
  const bytes = fs.readFileSync(path.join(dir, file.file));
  const sha = crypto.createHash("sha256").update(bytes).digest("hex");
  if (sha !== file.sha256) throw new Error(`${file.file}: SHA-256 differs from collection.json`);
}
const named = new Set(files.map((file) => path.basename(file.file)));
const extra = fs.readdirSync(path.join(dir, "shards")).filter((name) => !named.has(name));
if (extra.length) throw new Error(`files collection.json does not name: ${extra.join(", ")}`);
' "$collection"
name="untwist-libraries-$version"
mkdir -p "$out"
LC_ALL=C tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --mode=u=rwX,go=rX --format=gnu \
  -C "$collection" -cf - collection.json shards | gzip -9n > "$out/$name.tar.gz"
(cd "$out" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
echo "$out/$name.tar.gz"
