$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hydra-release-tooling-" + [guid]::NewGuid().ToString("N"))

try {
    cargo build --locked -p hydra-cli
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    $version = (& .\target\debug\hydra.exe --version).Split(' ')[1]
    $assets = Join-Path $testRoot "assets"
    & "$PSScriptRoot\package-release.ps1" `
        -Version $version `
        -Target "x86_64-pc-windows-msvc" `
        -Binary ".\target\debug\hydra.exe" `
        -OutputDirectory $assets

    $archive = Join-Path $assets "hydra-$version-x86_64-pc-windows-msvc.zip"
    $expanded = Join-Path $testRoot "expanded"
    Expand-Archive -LiteralPath $archive -DestinationPath $expanded
    $expected = @(
        "hydra.exe",
        "hydra-art.txt",
        "skills\hydra\SKILL.md",
        "skills\hydra\agents\openai.yaml",
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "README.md",
        "CHANGELOG.md"
    )
    foreach ($relativePath in $expected) {
        if (-not (Test-Path -LiteralPath (Join-Path $expanded $relativePath) -PathType Leaf)) {
            throw "release archive is missing $relativePath"
        }
    }
    if (Test-Path -LiteralPath (Join-Path $expanded "assets\hydra-banner.png")) {
        throw "release archive includes the repository banner"
    }
    $readmeHeading = Get-Content -LiteralPath (Join-Path $expanded "README.md") -TotalCount 1
    if ($readmeHeading -ne "# Hydra release archive") {
        throw "release archive README is not the compact release README"
    }
    if (-not (Test-Path -LiteralPath "$archive.sha256" -PathType Leaf)) {
        throw "release archive checksum is missing"
    }
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
