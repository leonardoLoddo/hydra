$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$hydraBinary = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot "target\debug\hydra.exe")).Path
$gitBash = Get-Command bash.exe -All | Where-Object { $_.Source -match '[\\/]Git[\\/]' } | Select-Object -First 1
if (-not $gitBash) {
    throw "Git Bash is not installed"
}

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hydra-gitbash-e2e-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testRoot | Out-Null
$env:HYDRA_E2E_ROOT = $testRoot
$env:HYDRA_E2E_BIN = $hydraBinary

try {
    $script = @'
set -euo pipefail
root=$(cygpath -u "$HYDRA_E2E_ROOT")
hydra_bin=$(cygpath -u "$HYDRA_E2E_BIN")
if [[ -n ${HYDRA_E2E_RUNTIME_BIN:-} ]]; then
    runtime_bin=$(cygpath -u "$HYDRA_E2E_RUNTIME_BIN")
    export PATH="$runtime_bin:$PATH"
fi
cd "$root"
git init -q -b main project
cd project
git config user.name "Hydra Windows E2E"
git config user.email "hydra-windows@example.invalid"
git config core.autocrlf false
printf "windows git bash\n" > tracked.txt
git add tracked.txt
git commit -q -m "initial"
"$hydra_bin" init
"$hydra_bin" head create payment --from main --target main
head_path=$("$hydra_bin" head path payment)
cd "$head_path"
"$hydra_bin" doctor storage
"$hydra_bin" head remove payment --force
cd "$root/project"
"$hydra_bin" status
"$hydra_bin" --help >/dev/null
'@
    $scriptPath = Join-Path $testRoot "workflow.sh"
    [System.IO.File]::WriteAllText(
        $scriptPath,
        $script.Replace("`r`n", "`n") + "`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    $normalizedScriptPath = $scriptPath.Replace('\', '/')
    if ($normalizedScriptPath -notmatch '^([A-Za-z]):/(.*)$') {
        throw "Git Bash smoke-test script is not on a drive-letter path"
    }
    $gitBashScriptPath = "/$($Matches[1].ToLowerInvariant())/$($Matches[2])"
    $output = & $gitBash.Source -lc "bash '$gitBashScriptPath'" 2>&1 | Tee-Object -Variable capturedOutput
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "Git Bash workflow failed with exit code $exitCode at $gitBashScriptPath`n$($capturedOutput -join "`n")"
    }
    $transcript = $capturedOutput -join "`n"
    foreach ($expected in @(
        "Initialized Hydra in",
        "New Head successfully created at",
        "Storage backend:",
        "Removed Head payment",
        "Heads: 0"
    )) {
        if (-not $transcript.Contains($expected)) {
            throw "Git Bash workflow output is missing '$expected'`n$transcript"
        }
    }
    Write-Output $transcript
}
finally {
    $resolvedTestRoot = (Resolve-Path -LiteralPath $testRoot).Path
    $temporaryRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if (-not $resolvedTestRoot.StartsWith($temporaryRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "unsafe Git Bash test cleanup target: $resolvedTestRoot"
    }
    Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
}
