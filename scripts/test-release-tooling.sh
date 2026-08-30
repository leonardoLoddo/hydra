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
require "rubygems"
require "yaml"

version_pattern = /\A[0-9]+\.[0-9]+\.[0-9]+(?:[.-][0-9A-Za-z.-]+)?\z/

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
initial_version = config.fetch("initial-version")
abort "error: initial release version is invalid" unless initial_version.match?(version_pattern)
abort "error: release version marker is invalid" unless release_version.match?(version_pattern)
if Gem::Version.new(initial_version) > Gem::Version.new(release_version)
  abort "error: initial release version is newer than version.txt"
end
metadata = JSON.parse(`cargo metadata --locked --no-deps --format-version 1`)
abort "error: cargo metadata failed" unless $?.success?
package_versions = metadata.fetch("packages").map { |package| package.fetch("version") }.uniq
abort "error: version.txt and Cargo package versions disagree" unless package_versions == [release_version]

workflow = YAML.safe_load(File.read(".github/workflows/release.yml"), aliases: true)
build_job = workflow.fetch("jobs").fetch("build")
build_matrix = build_job.fetch("strategy").fetch("matrix").fetch("include")
expected_build_matrix = [
  { "runner" => "macos-15", "target" => "aarch64-apple-darwin" },
  { "runner" => "macos-15-intel", "target" => "x86_64-apple-darwin" },
  { "runner" => "ubuntu-22.04", "target" => "x86_64-unknown-linux-gnu" },
  { "runner" => "ubuntu-22.04-arm", "target" => "aarch64-unknown-linux-gnu" },
  { "runner" => "windows-2025", "target" => "x86_64-pc-windows-msvc" },
]
unless build_matrix == expected_build_matrix
  abort "error: release build matrix must cover the supported native targets"
end
windows_package = build_job.fetch("steps").find do |candidate|
  candidate["name"] == "Package Windows release archive"
end
abort "error: missing Windows release packaging step" unless windows_package
unless windows_package["if"] == "runner.os == 'Windows'"
  abort "error: Windows release packaging must run only on Windows"
end
windows_build = build_job.fetch("steps").find do |candidate|
  candidate["name"] == "Build Windows release binary"
end
abort "error: missing Windows release build step" unless windows_build
unless windows_build["if"] == "runner.os == 'Windows'" && windows_build["shell"] == "pwsh"
  abort "error: Windows release build must use PowerShell only on Windows"
end
windows_completion_check = build_job.fetch("steps").find do |candidate|
  candidate["name"] == "Validate Windows Git Bash completion"
end
abort "error: missing Windows Git Bash completion validation step" unless windows_completion_check
unless windows_completion_check["if"] == "runner.os == 'Windows'" && windows_completion_check["shell"] == "bash"
  abort "error: Windows completion validation must use Git Bash only on Windows"
end
unless windows_completion_check.fetch("run").include?("completions/hydra.bash")
  abort "error: Windows completion validation does not inspect the packaged Git Bash script"
end
windows_completion_extract = build_job.fetch("steps").find do |candidate|
  candidate["name"] == "Extract Windows release archive for verification"
end
abort "error: missing Windows completion extraction step" unless windows_completion_extract
unless windows_completion_extract["if"] == "runner.os == 'Windows'" && windows_completion_extract["shell"] == "pwsh"
  abort "error: Windows completion extraction must use PowerShell only on Windows"
end
unless windows_completion_extract.fetch("run").include?("Expand-Archive") &&
       windows_completion_extract.fetch("run").include?("completions/hydra.bash")
  abort "error: Windows completion extraction does not verify the packaged script"
end

windows_packaging = File.read("scripts/package-release.ps1")
[
  '"completions"',
  '"hydra.bash"',
  'COMPLETE',
  'bash',
].each do |required_text|
  unless windows_packaging.include?(required_text)
    abort "error: Windows package does not generate Git Bash completion: #{required_text}"
  end
end

windows_release_test = File.read("scripts/test-windows-release-tooling.ps1")
unless windows_release_test.include?('"completions\hydra.bash"') &&
       windows_release_test.include?("bash -n")
  abort "error: Windows release tooling test does not validate packaged completion"
end

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
homebrew_matrix = workflow.fetch("jobs").fetch("homebrew-smoke").fetch("strategy").fetch("matrix").fetch("include")
expected_homebrew_matrix = [
  { "runner" => "macos-15", "platform" => "macos", "architecture" => "arm64" },
  { "runner" => "macos-15-intel", "platform" => "macos", "architecture" => "x86_64" },
  { "runner" => "ubuntu-22.04", "platform" => "linux", "architecture" => "x86_64" },
  { "runner" => "ubuntu-22.04-arm", "platform" => "linux", "architecture" => "arm64" },
]
unless homebrew_matrix == expected_homebrew_matrix
  abort "error: Homebrew smoke-test matrix must cover macOS and Linux on both release architectures"
end

setup_homebrew = homebrew_steps.find { |candidate| candidate["name"] == "Set up Homebrew on Linux" }
abort "error: missing Linux Homebrew setup step" unless setup_homebrew
unless setup_homebrew["if"] == "runner.os == 'Linux'"
  abort "error: Homebrew setup must run only on Linux"
end
unless setup_homebrew["uses"] == "Homebrew/actions/setup-homebrew@8f3d1ec8a696b3b9d9a6c3696b6c73033cab69e4"
  abort "error: Homebrew setup action must be pinned to the reviewed commit"
end

smoke_step = homebrew_steps.find do |candidate|
  candidate["name"] == "Audit and smoke-test the Formula from a temporary tap"
end
abort "error: missing Homebrew smoke-test step" unless smoke_step
unless smoke_step.fetch("run").include?('brew audit --strict "$tap_name/hydra-heads"')
  abort "error: Homebrew audit must use the tap-qualified Formula name"
end
[
  'etc/bash_completion.d/hydra',
  'share/zsh/site-functions/_hydra',
  'share/fish/vendor_completions.d/hydra.fish',
].each do |completion_path|
  unless smoke_step.fetch("run").include?(completion_path)
    abort "error: Homebrew smoke test does not verify completion: #{completion_path}"
  end
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
cp \
  "$archive.sha256" \
  "$assets/hydra-$version-aarch64-unknown-linux-gnu.tar.gz.sha256"
cp \
  "$archive.sha256" \
  "$assets/hydra-$version-x86_64-unknown-linux-gnu.tar.gz.sha256"
formula="$test_root/Formula/hydra-heads.rb"
scripts/render-homebrew-formula.sh "$version" "$assets" "$formula"
if grep -Eq '^[[:space:]]+version "' "$formula"; then
  echo "error: Homebrew Formula contains a redundant explicit version" >&2
  exit 1
fi
grep -F "/releases/download/v$version/hydra-$version-aarch64-apple-darwin.tar.gz" "$formula" >/dev/null
grep -F "/releases/download/v$version/hydra-$version-x86_64-apple-darwin.tar.gz" "$formula" >/dev/null
grep -F "/releases/download/v$version/hydra-$version-aarch64-unknown-linux-gnu.tar.gz" "$formula" >/dev/null
grep -F "/releases/download/v$version/hydra-$version-x86_64-unknown-linux-gnu.tar.gz" "$formula" >/dev/null
grep -F 'generate_completions_from_executable(bin/"hydra", shell_parameter_format: :clap)' "$formula" >/dev/null
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
