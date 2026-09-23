$ErrorActionPreference = 'Stop'
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = (Resolve-Path (Join-Path $ScriptDir '..')).Path
Push-Location $ProjectRoot

# Native crates that must stay warning-free on every supported target.
# koi-pyo3 is intentionally excluded: its build script requires a Python
# interpreter for the *target* triple, so it is only validated natively.
$NativeCrateArgs = @('-p', 'koi-maestro-core', '-p', 'koi-maestro-solver', '-p', 'koi-maestro-bench')
$CrossTargets = @('aarch64-unknown-linux-gnu', 'x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc')

function Invoke-CargoStep {
    param([string]$Label, [scriptblock]$Command)
    Write-Host "=== $Label ===" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Label failed with exit code $LASTEXITCODE" }
}

function Get-RustHostTriple {
    $line = rustc -vV | Select-String -Pattern 'host:' | Select-Object -First 1
    if (-not $line) { return $null }
    return ($line.Line -replace '.*host:\s*', '').Trim()
}

function Test-CrossTarget {
    param([string]$Target, [string]$HostTriple)
    if ($Target -eq $HostTriple) {
        Write-Host "=== cargo check ($Target) === skipped (host target, covered by clippy)" -ForegroundColor DarkGray
        return
    }
    $installed = rustup target list --installed
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "rustup target list failed; skipping $Target"
        return
    }
    if ($installed -notcontains $Target) {
        Write-Host "=== installing Rust target $Target ===" -ForegroundColor Cyan
        rustup target add $Target | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "could not install target $Target (offline or unavailable); skipping cross-check"
            return
        }
    }
    Invoke-CargoStep "cargo check ($Target)" { cargo check --all-targets @NativeCrateArgs --target $Target }
}

try {
    # PyO3 test binaries load the base interpreter's DLL at runtime — when a
    # project `.venv` exists, put its base prefix on PATH so `cargo test`
    # covers koi-pyo3 instead of aborting at test-list time (0xc0000135).
    $venvPython = Join-Path $ProjectRoot '.venv\Scripts\python.exe'
    if (Test-Path $venvPython) {
        $basePrefix = (& $venvPython -c 'import sys; print(sys.base_prefix)').Trim()
        if ($basePrefix -and (Test-Path $basePrefix)) {
            $env:PATH = "$basePrefix;$env:PATH"
        }
    }

    Invoke-CargoStep 'cargo fmt --check' { cargo fmt --check }
    Invoke-CargoStep 'cargo clippy' { cargo clippy --workspace --all-targets -- -D warnings }

    # Gate profile: release-ci (thin LTO, multi-CGU) keeps gate wall-clock low
    # while preserving release semantics (panic=abort, overflow-checks off).
    # KOI_PROFILE=release re-runs the certification profile — that is also the
    # only profile whose binaries may produce official performance numbers.
    $Profile = if ($env:KOI_PROFILE) { $env:KOI_PROFILE } else { 'release-ci' }

    if (Get-Command 'cargo-nextest' -ErrorAction SilentlyContinue) {
        Invoke-CargoStep "cargo nextest run ($Profile)" { cargo nextest run --cargo-profile $Profile --workspace }
    } else {
        Invoke-CargoStep "cargo test ($Profile)" { cargo test --profile $Profile --workspace }
    }
    Invoke-CargoStep "cargo test --doc ($Profile)" { cargo test --doc --profile $Profile --workspace }

    # Cross-target compile gate: `cargo check` needs no linker, so it verifies
    # that the native crates build for every supported OS/ISA from any host.
    # Toolchain setup failures (no rustup, offline) are skipped, but a failed
    # check on an installed target IS a portability regression and fails.
    # --all-targets also cross-compiles tests/examples: dev-deps must stay
    # pure-Rust (the criterion C toolchain was the blocker removed in P6).
    if (-not [string]::IsNullOrEmpty($env:KOI_SKIP_CROSS) -and $env:KOI_SKIP_CROSS -ne '0') {
        Write-Host "`n=== cross-target checks skipped (KOI_SKIP_CROSS) ===" -ForegroundColor Yellow
    } elseif (-not (Get-Command 'rustup' -ErrorAction SilentlyContinue)) {
        Write-Host "`n=== cross-target checks skipped (rustup not found) ===" -ForegroundColor Yellow
    } else {
        $hostTriple = Get-RustHostTriple
        foreach ($target in $CrossTargets) { Test-CrossTarget -Target $target -HostTriple $hostTriple }
        Write-Host "`nNOTE: koi-pyo3 excluded from cross-checks (needs a target Python); build with maturin on the host." -ForegroundColor DarkGray
    }

    Write-Host "`n=== ALL CHECKS PASSED ===" -ForegroundColor Green
}
finally {
    Pop-Location
}
