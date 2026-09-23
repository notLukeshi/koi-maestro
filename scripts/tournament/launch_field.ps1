<#
.SYNOPSIS
    launch_field.ps1 -- dispatches the field phase: every pair manifest
    gets a lane slot; runs are sequential within each lane and parallel
    across lanes.

.DESCRIPTION
    LAW-3 lane admission: the lane count comes from pilot M-04, not from
    the sibling codebase. Each lane runs pairs sequentially via `koi-bench run`; pairs
    resume from checkpoint automatically (never replay completed clusters).

    Requires freeze.json to exist -- canonical field evidence is bound to
    the frozen build/config/panel. Run freeze.ps1 first.

.PARAMETER TournamentDir
    Directory containing freeze.json, panel.json, and manifests/pair-*.json.
    Default: docs/benchmarks/tournament/

.PARAMETER LaneCount
    Must match the lane_count recorded in freeze.json.

.PARAMETER Pairs
    Optional subset of pair manifests to run (e.g. "pair-random-heuristic").
    Default runs every pair-*.json found.

.EXAMPLE
    .\scripts\tournament\launch_field.ps1 -LaneCount 4
#>

[CmdletBinding()]
param(
    [string]$TournamentDir = "docs/benchmarks/tournament",
    [Parameter(Mandatory)][int]$LaneCount,
    [string[]]$Pairs
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# -- freeze gate ---------------------------------------------------------
$freezePath = Join-Path $TournamentDir "freeze.json"
if (-not (Test-Path $freezePath)) {
    throw "freeze.json not found at $freezePath -- run freeze.ps1 first"
}
$freeze = Get-Content $freezePath -Raw | ConvertFrom-Json
if ($freeze.lane_count -ne $LaneCount) {
    throw "lane mismatch: freeze recorded $($freeze.lane_count), requested $LaneCount"
}
Write-Host "freeze verified: commit $($freeze.git_commit.Substring(0,8)), lanes $LaneCount"

# -- collect pair manifests ------------------------------------------------
$manifestDir = Join-Path $TournamentDir "manifests"
$allPairs = Get-ChildItem "$manifestDir/pair-*.json" | Sort-Object Name
if ($Pairs) {
    $allPairs = $allPairs | Where-Object { $Pairs -contains $_.BaseName }
}
if ($allPairs.Count -eq 0) { throw "no pair manifests found in $manifestDir" }

$binary = "target/release/koi-bench.exe"
if (-not (Test-Path $binary)) { $binary = "target/release/koi-bench" }
if (-not (Test-Path $binary)) { throw "koi-bench release binary not found" }
$binary = (Resolve-Path $binary).Path

# -- sequential lane dispatch -------------------------------------------------
$resultsDir = Join-Path $TournamentDir "results"
New-Item -ItemType Directory -Force -Path $resultsDir | Out-Null
$resultsDir = (Resolve-Path $resultsDir).Path

# Pair paths as strings — FileInfo objects don't survive the job boundary.
$pairPaths = $allPairs | ForEach-Object { $_.FullName }
$pairNames = $allPairs | ForEach-Object { $_.BaseName }

$laneJobs = @()
for ($lane = 0; $lane -lt $LaneCount; $lane++) {
    $laneJobs += Start-Job -Name "lane-$lane" -ScriptBlock {
        param($pairPaths, $pairNames, $binary, $resultsDir, $laneIndex, $laneCount)
        for ($i = $laneIndex; $i -lt $pairPaths.Count; $i += $laneCount) {
            $pairPath = $pairPaths[$i]
            $pairName = $pairNames[$i]
            $outDir = Join-Path $resultsDir "$pairName-out"
            $reportJson = Join-Path $outDir "report.json"
            if (Test-Path $reportJson) { continue }
            Write-Host "  [$pairName] starting -> $outDir"
            & $binary run --manifest $pairPath --out $outDir 2>&1 | ForEach-Object {
                Write-Host "  [$pairName] $_"
            }
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "  [$pairName] exited $LASTEXITCODE -- check $outDir"
            } else {
                Write-Host "  [$pairName] complete"
            }
        }
    } -ArgumentList $pairPaths, $pairNames, $binary, $resultsDir, $lane, $LaneCount
}

Write-Host "$($pairPaths.Count) pairs queued across $LaneCount lanes (stride dispatch)"
Write-Host "waiting for lanes..."
$laneJobs | Wait-Job | Out-Null
$laneJobs | Receive-Job
$laneJobs | Remove-Job -Force

# -- post-launch integrity ---------------------------------------------------
$completed = (Get-ChildItem "$resultsDir/*-out/report.json" -ErrorAction SilentlyContinue).Count
$total = $pairPaths.Count
Write-Host ""
Write-Host "field dispatch finished: $completed/$total pairs produced report.json"
if ($completed -lt $total) {
    Write-Warning "incomplete pairs remain -- re-run launch_field.ps1 to resume"
}
