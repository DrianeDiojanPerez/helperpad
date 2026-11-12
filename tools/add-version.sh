#!/usr/bin/env bash
# Bumps the version in Cargo.toml, commits it, and tags it.
# Usage: tools/add-version.sh 1.2.3
set -euo pipefail

cd "$(dirname "$0")/.."

version=${1:-}
version=${version#v}
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Version must look like 1.0.0." >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "Commit or stash your changes first." >&2
    exit 1
fi

current=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo "$current -> $version"

sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
cargo update --workspace --quiet

# The tag is annotated so that `git push --follow-tags` sends it along.
git add Cargo.toml Cargo.lock
git commit -q -m "release v$version"
git tag -a "v$version" -m "release v$version"

echo "Tagged v$version. Run 'git push --follow-tags', then 'make release'."
