<#
.SYNOPSIS
    rank.ps1 - assembles the tournament ranking report from all pair
    results: Davidson tie-aware BT + Holm, common-prefix refit, cyclic
    triples, bootstrap CIs, stopping provenance.

.DESCRIPTION
    Runs `koi-bench tournament-rank` over every pair-*-out/report.json
    found in the results directory. Produces:
      - docs/benchmarks/tournament/tournament-results.json  (machine-readable)
      - docs/benchmarks/tournament/tournament-report.md    (human-readable)

    Requires every pair to have produced a report.json - incomplete
    pairs are reported as inconclusive, not silently dropped.

.PARAMETER TournamentDir
    Directory containing results/pair-*-out/report.json and the
    tournament manifest. Default: docs/benchmarks/tournament/

.PARAMETER TournamentManifest
    Path to the tournament manifest (entrant roster + panel spec).

.PARAMETER OutDir
    Where tournament-results.json + tournament-report.md land.
    Default: docs/benchmarks/tournament/

.EXAMPLE
    .\scripts\tournament\rank.ps1 `
        -TournamentManifest docs/benchmarks/tournament/manifest.json
#>

[CmdletBinding()]
param(
    [string]$TournamentDir = "docs/benchmarks/tournament",
    [Parameter(Mandatory)][string]$TournamentManifest,
    [string]$OutDir = "docs/benchmarks/tournament"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$resultsDir = Join-Path $TournamentDir "results"
$reports = Get-ChildItem "$resultsDir/*-out/report.json" -ErrorAction SilentlyContinue
if ($reports.Count -eq 0) { throw "no report.json files in $resultsDir" }

$binary = "target/release/tournament_rank.exe"
if (-not (Test-Path $binary)) { $binary = "target/release/tournament_rank" }
if (-not (Test-Path $binary)) { throw "tournament_rank binary not found - build release first" }

Write-Host "ranking $($reports.Count) pair results -> $OutDir"
& $binary `
    --tournament-manifest $TournamentManifest `
    --results-dir $resultsDir `
    --output-dir $OutDir
if ($LASTEXITCODE -ne 0) { throw "tournament_rank failed (exit $LASTEXITCODE)" }

$jsonPath = Join-Path $OutDir "tournament-results.json"
$mdPath = Join-Path $OutDir "tournament-report.md"
if (Test-Path $jsonPath) { Write-Host "wrote $jsonPath" }
if (Test-Path $mdPath) { Write-Host "wrote $mdPath" }
