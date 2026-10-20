#!/usr/bin/env bash
# Checks that a release tag matches the versions declared in the repository.
#   scripts/check-release-version.sh v0.2.0
# Prints the version (without the leading "v") on success.
set -euo pipefail

tag="${1:?usage: $0 <tag>}"
if [[ ! "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::Tag « $tag » is not a version tag such as v1.2.3 or v1.2.3-rc.1." >&2
  exit 1
fi
version="${tag#v}"
base="${BASH_REMATCH[1]}"
status=0

# Versions as Cargo resolves them (workspace inheritance included), for every crate.
while read -r name crate_version; do
  if [[ "$crate_version" != "$version" ]]; then
    echo "::error::Crate $name is at version $crate_version but the tag is $tag." >&2
    status=1
  fi
done < <(cargo metadata --no-deps --format-version 1 --locked | jq -r '.packages[] | "\(.name) \(.version)"')

# Firefox only accepts dotted numbers, so pre-release tags compare against the base version.
manifest_version=$(jq -r .version extension/firefox/manifest.json)
if [[ "$manifest_version" != "$base" ]]; then
  echo "::error::extension/firefox/manifest.json is at version $manifest_version, expected $base for $tag." >&2
  status=1
fi

[[ $status -eq 0 ]] && echo "$version"
exit $status
