<#
.SYNOPSIS
    launch_annex.ps1 - Phase D: champion + runner-up vs all 7 archetypes
    (14 pairs), same protocol as the field.

.DESCRIPTION
    Reads the ranking report to identify champion and runner-up, then
    emits annex pair manifests against every archetype entrant and runs
    them under the same GSPRT+EB-CS protocol as the field.

    Archetypes are annex opponents - not field entrants. The plan's
    descriptive names map to the engine's actual enum/config names:
      heuristic, materialist, yaku_chaser, banker, gambler, timid, random

.PARAMETER TournamentDir
    Field tournament directory (contains tournament-results.json + panel.json).
    Default: docs/benchmarks/tournament/

.PARAMETER RankingJson
    Path to the ranking report (tournament-results.json). Default:
    docs/benchmarks/tournament/tournament-results.json

.PARAMETER AnnexDir
    Where annex pair dirs + results land.
    Default: docs/benchmarks/tournament/annex/

.PARAMETER LaneCount
    Lane count for annex dispatch (same as field or lower).

.EXAMPLE
    .\scripts\tournament\launch_annex.ps1 -LaneCount 4
#>

[CmdletBinding()]
param(
    [string]$TournamentDir = "docs/benchmarks/tournament",
    [string]$RankingJson = "docs/benchmarks/tournament/tournament-results.json",
    [string]$AnnexDir = "docs/benchmarks/tournament/annex",
    [Parameter(Mandatory)][int]$LaneCount
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# -- identify champion + runner-up from ranking ---------------------------
if (-not (Test-Path $RankingJson)) { throw "ranking not found: $RankingJson" }
$ranking = Get-Content $RankingJson -Raw | ConvertFrom-Json

# The ranking's entrants array is indexed by entrant number, not sorted by Elo.
# Sort by Elo descending to get champion and runner-up.
$provenance = $ranking.entrants
$ranked = $ranking.ranking.entrants | Sort-Object -Property elo -Descending
$champion = $ranked[0]
$runnerUp = $ranked[1]
$championLabel = $provenance[$champion.entrant].label
$runnerUpLabel = $provenance[$runnerUp.entrant].label
Write-Host "annex: champion=$championLabel ($($champion.elo) Elo) runner-up=$runnerUpLabel ($($runnerUp.elo) Elo)"

# -- archetype roster -------------------------------------------------------
$archetypes = @(
    "heuristic", "materialist", "yaku_chaser", "banker",
    "gambler", "timid", "random"
)

# -- emit annex pair manifests ------------------------------------------------
New-Item -ItemType Directory -Force -Path "$AnnexDir/manifests" | Out-Null
New-Item -ItemType Directory -Force -Path "$AnnexDir/results" | Out-Null

$panelJson = Get-Content (Join-Path $TournamentDir "panel.json") -Raw | ConvertFrom-Json
$freezePath = Join-Path $TournamentDir "freeze.json"
if (-not (Test-Path $freezePath)) {
    throw "freeze.json missing — run scripts/tournament/freeze.ps1 to bind the field first"
}
$freeze = Get-Content $freezePath -Raw | ConvertFrom-Json
$manifest = Get-Content (Join-Path $TournamentDir "manifest.json") -Raw | ConvertFrom-Json

$annexPairs = @()
foreach ($top in @($champion, $runnerUp)) {
    $topLabel = $provenance[$top.entrant].label
    # Find a manifest containing this entrant to extract its config
    $sourceManifest = Get-ChildItem "$TournamentDir/manifests/pair-*$topLabel*.json" | Select-Object -First 1
    if (-not $sourceManifest) { throw "no manifest found for entrant $topLabel" }
    $sm = Get-Content $sourceManifest.FullName -Raw | ConvertFrom-Json
    $topConfig = if ($sm.baseline.label -eq $topLabel) { $sm.baseline.config } else { $sm.candidate.config }

    foreach ($arch in $archetypes) {
        $pairName = "pair-$topLabel-$arch"
        $manifestPath = "$($PWD.Path)/$AnnexDir/manifests/$pairName.json"

        # Build a pairwise manifest: top entrant as baseline, archetype as candidate.
        $manifest = [ordered]@{
            schema_version = 1
            run_label = "annex: $topLabel vs $arch"
            ruleset = $freeze.ruleset
            seeds = $panelJson.seeds
            stopping = $freeze.stopping
            worker_timeout_seconds = $manifest.worker_timeout_seconds
            time_hard_cap_ms = $manifest.time_hard_cap_ms
            invalid_forfeit_margin = $manifest.invalid_forfeit_margin
            bootstrap = $manifest.bootstrap
            baseline = [ordered]@{
                label = $topLabel
                artifact = @{ kind = "self_binary" }
                config = $topConfig
            }
            candidate = [ordered]@{
                label = $arch
                artifact = @{ kind = "self_binary" }
                config = @{
                    solver = "archetype"
                    archetype = $arch
                    seed = 1
                }
            }
        }

        $json = $manifest | ConvertTo-Json -Depth 8
        [System.IO.File]::WriteAllText($manifestPath, $json, [System.Text.UTF8Encoding]::new($false))
        $annexPairs += $manifestPath
    }
}

Write-Host "emitted $($annexPairs.Count) annex pair manifests"

# -- sequential lane dispatch -------------------------------------------------
$binary = "target/release/koi-bench.exe"
if (-not (Test-Path $binary)) { $binary = "target/release/koi-bench" }
if (-not (Test-Path $binary)) { throw "koi-bench release binary not found" }
$binary = (Resolve-Path $binary).Path

$resultsDir = (Resolve-Path "$AnnexDir/results").Path

# Pair paths as strings - annexPairs are already string paths.
$pairPaths = $annexPairs
$pairNames = $annexPairs | ForEach-Object { [System.IO.Path]::GetFileNameWithoutExtension($_) }

$laneJobs = @()
for ($lane = 0; $lane -lt $LaneCount; $lane++) {
    $laneJobs += Start-Job -Name "annex-lane-$lane" -ScriptBlock {
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
                Write-Warning "  [$pairName] exited $LASTEXITCODE - check $outDir"
            } else {
                Write-Host "  [$pairName] complete"
            }
        }
    } -ArgumentList $pairPaths, $pairNames, $binary, $resultsDir, $lane, $LaneCount
}

Write-Host "$($pairPaths.Count) annex pairs queued across $LaneCount lanes (stride dispatch)"
Write-Host "waiting for lanes..."
$laneJobs | Wait-Job | Out-Null
$laneJobs | Receive-Job
$laneJobs | Remove-Job -Force

# -- post-launch integrity ---------------------------------------------------
$completed = (Get-ChildItem "$resultsDir/*-out/report.json" -ErrorAction SilentlyContinue).Count
$total = $pairPaths.Count
Write-Host ""
Write-Host "annex dispatch finished: $completed/$total pairs produced report.json"
if ($completed -lt $total) {
    Write-Warning "incomplete pairs remain - re-run launch_annex.ps1 to resume"
}
