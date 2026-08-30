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
$resolvedBinary = (Resolve-Path -LiteralPath $Binary).Path

$repositoryRoot = Split-Path -Parent $PSScriptRoot
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$resolvedOutput = (Resolve-Path -LiteralPath $OutputDirectory).Path
$stagingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hydra-release-" + [guid]::NewGuid().ToString("N"))
$packageRoot = Join-Path $stagingRoot "package"

try {
    New-Item -ItemType Directory -Path (Join-Path $packageRoot "skills\hydra\agents") -Force | Out-Null
    $completionDirectory = Join-Path $packageRoot "completions"
    New-Item -ItemType Directory -Path $completionDirectory -Force | Out-Null

    $completionStartInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $completionStartInfo.FileName = $resolvedBinary
    $completionStartInfo.UseShellExecute = $false
    $completionStartInfo.RedirectStandardOutput = $true
    $completionStartInfo.RedirectStandardError = $true
    $completionStartInfo.Environment["COMPLETE"] = "bash"
    $completionProcess = [System.Diagnostics.Process]::new()
    $completionProcess.StartInfo = $completionStartInfo
    if (-not $completionProcess.Start()) {
        throw "could not start release binary to generate Git Bash completion"
    }
    $completionOutput = $completionProcess.StandardOutput.ReadToEndAsync()
    $completionError = $completionProcess.StandardError.ReadToEndAsync()
    $completionProcess.WaitForExit()
    $completionText = $completionOutput.Result
    $completionErrorText = $completionError.Result
    if ($completionProcess.ExitCode -ne 0) {
        throw "Git Bash completion generation failed: $completionErrorText"
    }
    if ([string]::IsNullOrWhiteSpace($completionText)) {
        throw "Git Bash completion generation produced empty output"
    }
    $completionProcess.Dispose()
    $completionPath = Join-Path $completionDirectory "hydra.bash"
    [System.IO.File]::WriteAllText(
        $completionPath,
        $completionText,
        [System.Text.UTF8Encoding]::new($false)
    )

    Copy-Item -LiteralPath $resolvedBinary -Destination (Join-Path $packageRoot "hydra.exe")
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
