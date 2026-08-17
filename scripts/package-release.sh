#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <version> <target> <binary> <output-directory>" >&2
  exit 2
fi

version=$1
target=$2
binary=$3
output_directory=$4
repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: invalid release version: $version" >&2
  exit 2
fi
if [[ ! $target =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "error: invalid release target: $target" >&2
  exit 2
fi
if [[ ! -f $binary ]]; then
  echo "error: release binary does not exist: $binary" >&2
  exit 2
fi

mkdir -p "$output_directory"
staging_root=$(mktemp -d "${TMPDIR:-/tmp}/hydra-release.XXXXXX")
trap 'rm -rf "$staging_root"' EXIT
package_root="$staging_root/package"
mkdir -p "$package_root/skills/hydra/agents"

cp "$binary" "$package_root/hydra"
cp "$repository_root/hydra-art.txt" "$package_root/hydra-art.txt"
cp "$repository_root/skills/hydra/SKILL.md" "$package_root/skills/hydra/SKILL.md"
cp "$repository_root/skills/hydra/agents/openai.yaml" "$package_root/skills/hydra/agents/openai.yaml"
cp "$repository_root/LICENSE" "$package_root/LICENSE"
cp "$repository_root/LICENSE-MIT" "$package_root/LICENSE-MIT"
cp "$repository_root/LICENSE-APACHE" "$package_root/LICENSE-APACHE"
cp "$repository_root/packaging/release/README.md" "$package_root/README.md"
cp "$repository_root/CHANGELOG.md" "$package_root/CHANGELOG.md"

archive_name="hydra-${version}-${target}.tar.gz"
archive_path="$output_directory/$archive_name"
tar -C "$package_root" -czf "$archive_path" .

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$output_directory" && sha256sum "$archive_name" > "$archive_name.sha256")
else
  digest=$(shasum -a 256 "$archive_path" | awk '{print $1}')
  printf '%s  %s\n' "$digest" "$archive_name" > "$archive_path.sha256"
fi
