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
    $latestArchiveName = "hydra-windows-x86_64.zip"
    $latestArchive = Join-Path $assets $latestArchiveName
    if (-not (Test-Path -LiteralPath $latestArchive -PathType Leaf)) {
        throw "release assets are missing the stable Windows download"
    }
    if (-not (Test-Path -LiteralPath "$latestArchive.sha256" -PathType Leaf)) {
        throw "stable Windows download checksum is missing"
    }
    $canonicalDigest = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $latestDigest = (Get-FileHash -LiteralPath $latestArchive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($latestDigest -ne $canonicalDigest) {
        throw "stable Windows download does not match the versioned archive"
    }
    $expectedLatestChecksum = "$latestDigest  $latestArchiveName"
    $actualLatestChecksum = (Get-Content -LiteralPath "$latestArchive.sha256" -Raw).Trim()
    if ($actualLatestChecksum -ne $expectedLatestChecksum) {
        throw "stable Windows download checksum does not name the downloadable asset"
    }
    $readme = Get-Content -LiteralPath (Join-Path $repositoryRoot "README.md") -Raw
    $latestDownloadUrl = "https://github.com/leonardoLoddo/hydra/releases/latest/download/$latestArchiveName"
    if (-not $readme.Contains($latestDownloadUrl)) {
        throw "repository README does not link to the stable Windows download"
    }
    $expanded = Join-Path $testRoot "expanded"
    Expand-Archive -LiteralPath $archive -DestinationPath $expanded
    $expected = @(
        "hydra.exe",
        "hydra-art.txt",
        "completions\hydra.bash",
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
    $completion = Join-Path $expanded "completions\hydra.bash"
    & bash -n $completion
    if ($LASTEXITCODE -ne 0) {
        throw "packaged Git Bash completion has invalid syntax"
    }
    if (-not (Select-String -LiteralPath $completion -SimpleMatch "_clap_complete_hydra" -Quiet)) {
        throw "packaged Git Bash completion is not Hydra's generated registration"
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
