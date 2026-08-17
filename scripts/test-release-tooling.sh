#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repository_root"

bash -n \
  scripts/package-release.sh \
  scripts/render-homebrew-formula.sh \
  scripts/verify-homebrew-caveats.sh

cargo build --locked -p hydra-cli
version=$(target/debug/hydra --version | awk '{print $2}')
test_root=$(mktemp -d "${TMPDIR:-/tmp}/hydra-release-tooling.XXXXXX")
trap 'rm -rf "$test_root"' EXIT
assets="$test_root/assets"
scripts/package-release.sh "$version" aarch64-apple-darwin target/debug/hydra "$assets"

archive="$assets/hydra-$version-aarch64-apple-darwin.tar.gz"
for expected in \
  ./hydra \
  ./hydra-art.txt \
  ./skills/hydra/SKILL.md \
  ./skills/hydra/agents/openai.yaml \
  ./LICENSE \
  ./LICENSE-MIT \
  ./LICENSE-APACHE \
  ./README.md \
  ./CHANGELOG.md; do
  tar -tzf "$archive" | grep -F -x "$expected" >/dev/null
done

cp \
  "$archive.sha256" \
  "$assets/hydra-$version-x86_64-apple-darwin.tar.gz.sha256"
formula="$test_root/Formula/hydra-heads.rb"
scripts/render-homebrew-formula.sh "$version" "$assets" "$formula"
ruby -c "$formula"
brew style "$formula"

installation_log="$test_root/homebrew-install.log"
{
  echo "==> Caveats"
  cat hydra-art.txt
  echo
  echo "Get started:"
  echo "  hydra --help"
  echo
  echo "Optional Codex skill:"
  echo "  hydra skill install codex"
} > "$installation_log"
scripts/verify-homebrew-caveats.sh "$installation_log" hydra-art.txt

printf 'no caveats\n' > "$installation_log"
if scripts/verify-homebrew-caveats.sh "$installation_log" hydra-art.txt 2>/dev/null; then
  echo "error: caveat verification accepted missing artwork" >&2
  exit 1
fi
