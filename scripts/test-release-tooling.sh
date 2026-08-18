#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repository_root"

bash -n \
  scripts/package-release.sh \
  scripts/render-homebrew-formula.sh \
  scripts/verify-homebrew-caveats.sh

ruby <<'RUBY'
require "json"
require "yaml"

config = JSON.parse(File.read("release-please-config.json"))
expected_extra_files = [
  {
    "type" => "toml",
    "path" => "Cargo.toml",
    "jsonpath" => "$.workspace.package.version",
  },
  {
    "type" => "toml",
    "path" => "Cargo.lock",
    "jsonpath" => "$.package[?(@.name.value=='hydra-cli'||@.name.value=='hydra-core')].version",
  },
]

abort "error: release strategy must support inherited Cargo versions" unless config["release-type"] == "simple"
abort "error: release version marker is not configured" unless config["version-file"] == "version.txt"
abort "error: cargo-workspace cannot parse version.workspace" if Array(config["plugins"]).any? { |plugin| plugin["type"] == "cargo-workspace" }
abort "error: release version files are not configured atomically" unless config["extra-files"] == expected_extra_files

release_version = File.read("version.txt").strip
abort "error: initial release version and version.txt disagree" unless config["initial-version"] == release_version
metadata = JSON.parse(`cargo metadata --locked --no-deps --format-version 1`)
abort "error: cargo metadata failed" unless $?.success?
package_versions = metadata.fetch("packages").map { |package| package.fetch("version") }.uniq
abort "error: version.txt and Cargo package versions disagree" unless package_versions == [release_version]

workflow = YAML.safe_load(File.read(".github/workflows/release.yml"), aliases: true)
publish_steps = workflow.fetch("jobs").fetch("publish").fetch("steps")
repository_bound_steps = [
  "Upload assets to the draft release",
  "Publish the GitHub release",
]
repository_bound_steps.each do |step_name|
  step = publish_steps.find { |candidate| candidate["name"] == step_name }
  abort "error: missing release publication step: #{step_name}" unless step
  unless step.fetch("env", {})["GH_REPO"] == "leonardoLoddo/hydra"
    abort "error: release publication step does not identify the repository: #{step_name}"
  end
end

homebrew_steps = workflow.fetch("jobs").fetch("homebrew-smoke").fetch("steps")
smoke_step = homebrew_steps.find do |candidate|
  candidate["name"] == "Audit and smoke-test the Formula from a temporary tap"
end
abort "error: missing Homebrew smoke-test step" unless smoke_step
unless smoke_step.fetch("run").include?('brew audit --strict "$tap_name/hydra-heads"')
  abort "error: Homebrew audit must use the tap-qualified Formula name"
end

["formula", "homebrew-smoke"].each do |job_name|
  checkout = workflow.fetch("jobs").fetch(job_name).fetch("steps").find do |candidate|
    candidate["name"] == "Check out release source"
  end
  abort "error: missing release tooling checkout: #{job_name}" unless checkout
  unless checkout.fetch("with", {})["ref"] == "${{ github.sha }}"
    abort "error: manual release recovery does not use the selected workflow revision: #{job_name}"
  end
end
RUBY

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

if tar -tzf "$archive" | grep -F -x './assets/hydra-banner.png' >/dev/null; then
  echo "error: release archive includes the repository banner" >&2
  exit 1
fi
tar -xOf "$archive" ./README.md | grep -F -x '# Hydra release archive' >/dev/null

cp \
  "$archive.sha256" \
  "$assets/hydra-$version-x86_64-apple-darwin.tar.gz.sha256"
formula="$test_root/Formula/hydra-heads.rb"
scripts/render-homebrew-formula.sh "$version" "$assets" "$formula"
if grep -Eq '^[[:space:]]+version "' "$formula"; then
  echo "error: Homebrew Formula contains a redundant explicit version" >&2
  exit 1
fi
grep -F "/releases/download/v$version/hydra-$version-aarch64-apple-darwin.tar.gz" "$formula" >/dev/null
grep -F "/releases/download/v$version/hydra-$version-x86_64-apple-darwin.tar.gz" "$formula" >/dev/null
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
