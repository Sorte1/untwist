#!/bin/bash
set -euo pipefail
if [ $# -ne 2 ]; then
  echo "usage: unpack.sh <dir with the tarball and its .sha256> <destination>" >&2
  exit 2
fi
release=$1
dest=$2
shopt -s nullglob
tarballs=("$release"/untwist-libraries-*.tar.gz)
if [ ${#tarballs[@]} -ne 1 ]; then
  echo "expected one untwist-libraries-*.tar.gz in $release, found ${#tarballs[@]}" >&2
  exit 1
fi
tarball=${tarballs[0]}
(cd "$release" && sha256sum -c "$(basename "$tarball").sha256")
if tar -tvzf "$tarball" | cut -c1 | grep -v '^[-d]$' | grep -q .; then
  echo "$tarball holds an entry that is neither a file nor a folder" >&2
  exit 1
fi
if tar -tzf "$tarball" | grep -Ev '^(collection\.json|shards/|shards/[0-9a-f]{64}\.(utl|bloom))$' | grep -q .; then
  echo "$tarball holds a path other than collection.json and shards/<sha256>" >&2
  exit 1
fi
if [ -e "$dest/collection.json" ] || [ -e "$dest/shards" ]; then
  echo "$dest already has a collection" >&2
  exit 1
fi
mkdir -p "$dest"
tar -xzf "$tarball" --no-same-owner --no-same-permissions -C "$dest"
echo "$(find "$dest/shards" -type f | wc -l) files under $dest/shards"
