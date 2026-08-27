param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [Parameter(Mandatory = $true)]
    [string]$Target,
    [Parameter(Mandatory = $true)]
    [string]$Binary,
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"

if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$') {
    throw "invalid release version: $Version"
}
if ($Target -notmatch '^[A-Za-z0-9_.-]+$') {
    throw "invalid release target: $Target"
}
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "release binary does not exist: $Binary"
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$resolvedOutput = (Resolve-Path -LiteralPath $OutputDirectory).Path
$stagingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hydra-release-" + [guid]::NewGuid().ToString("N"))
$packageRoot = Join-Path $stagingRoot "package"

try {
    New-Item -ItemType Directory -Path (Join-Path $packageRoot "skills\hydra\agents") -Force | Out-Null
    Copy-Item -LiteralPath $Binary -Destination (Join-Path $packageRoot "hydra.exe")
    Copy-Item -LiteralPath (Join-Path $repositoryRoot "hydra-art.txt") -Destination $packageRoot
    Copy-Item -LiteralPath (Join-Path $repositoryRoot "skills\hydra\SKILL.md") -Destination (Join-Path $packageRoot "skills\hydra")
    Copy-Item -LiteralPath (Join-Path $repositoryRoot "skills\hydra\agents\openai.yaml") -Destination (Join-Path $packageRoot "skills\hydra\agents")
    foreach ($fileName in @("LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "CHANGELOG.md")) {
        Copy-Item -LiteralPath (Join-Path $repositoryRoot $fileName) -Destination $packageRoot
    }
    Copy-Item -LiteralPath (Join-Path $repositoryRoot "packaging\release\README.md") -Destination (Join-Path $packageRoot "README.md")

    $archiveName = "hydra-$Version-$Target.zip"
    $archivePath = Join-Path $resolvedOutput $archiveName
    Compress-Archive -Path (Join-Path $packageRoot "*") -DestinationPath $archivePath
    $digest = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath "$archivePath.sha256" -Value "$digest  $archiveName" -Encoding ascii

    $architecture = $Target.Split('-')[0]
    $latestArchiveName = "hydra-windows-$architecture.zip"
    $latestArchivePath = Join-Path $resolvedOutput $latestArchiveName
    Copy-Item -LiteralPath $archivePath -Destination $latestArchivePath -Force
    Set-Content `
        -LiteralPath "$latestArchivePath.sha256" `
        -Value "$digest  $latestArchiveName" `
        -Encoding ascii
}
finally {
    if (Test-Path -LiteralPath $stagingRoot) {
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    }
}
