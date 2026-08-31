#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <version> <checksums-directory> <output-formula>" >&2
  exit 2
fi

version=$1
checksums_directory=$2
output_formula=$3

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: invalid release version: $version" >&2
  exit 2
fi

checksum() {
  local target=$1
  local archive="hydra-${version}-${target}.tar.gz"
  local checksum_file="$checksums_directory/$archive.sha256"
  local digest
  if [[ ! -f $checksum_file ]]; then
    echo "error: missing checksum file: $checksum_file" >&2
    exit 2
  fi
  digest=$(awk 'NR == 1 { print $1 }' "$checksum_file")
  if [[ ! $digest =~ ^[0-9a-f]{64}$ ]]; then
    echo "error: invalid SHA-256 in $checksum_file" >&2
    exit 2
  fi
  printf '%s' "$digest"
}

macos_arm=$(checksum aarch64-apple-darwin)
macos_intel=$(checksum x86_64-apple-darwin)
linux_arm=$(checksum aarch64-unknown-linux-gnu)
linux_intel=$(checksum x86_64-unknown-linux-gnu)
mkdir -p "$(dirname "$output_formula")"

cat > "$output_formula" <<FORMULA
# typed: strict
# frozen_string_literal: true

# Formula for the Hydra Git-native workspace manager.
class HydraHeads < Formula
  desc "Git-native workspace manager for isolated development Heads"
  homepage "https://github.com/leonardoLoddo/hydra"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "git"

  on_macos do
    on_arm do
      url "https://github.com/leonardoLoddo/hydra/releases/download/v$version/hydra-$version-aarch64-apple-darwin.tar.gz"
      sha256 "$macos_arm"
    end
    on_intel do
      url "https://github.com/leonardoLoddo/hydra/releases/download/v$version/hydra-$version-x86_64-apple-darwin.tar.gz"
      sha256 "$macos_intel"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/leonardoLoddo/hydra/releases/download/v$version/hydra-$version-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "$linux_arm"
    end
    on_intel do
      url "https://github.com/leonardoLoddo/hydra/releases/download/v$version/hydra-$version-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$linux_intel"
    end
  end

  conflicts_with "hydra", because: "both install a hydra executable"
  conflicts_with "ory-hydra", because: "both install a hydra executable"

  def install
    bin.install "hydra"
    generate_completions_from_executable(bin/"hydra", shell_parameter_format: :clap)
    pkgshare.install "hydra-art.txt"
    pkgshare.install "skills"
    pkgshare.install "LICENSE", "LICENSE-MIT", "LICENSE-APACHE"
  end

  def caveats
    art = (pkgshare/"hydra-art.txt").read
    <<~EOS
      #{art}
      Get started:
        hydra --help

      Optional AI-agent skill (choose a provider):
        hydra skill install codex
        hydra skill install gemini
        hydra skill install agy
        hydra skill install antigravity
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hydra --version")
    ENV["HOME"] = testpath
    system bin/"hydra", "skill", "install", "codex", "--yes"
    system bin/"hydra", "skill", "status", "codex"
    system bin/"hydra", "skill", "remove", "codex", "--yes"
  end
end
FORMULA
