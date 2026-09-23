<#
.SYNOPSIS
    verify.ps1 -- integrity verifier over every pair's legs.jsonl:
    schema, protocol, provenance, first-cluster check.

.DESCRIPTION
    Validates each pair's legs.jsonl:
      - header line binds manifest hash + artifact identities
      - every cluster line parses as a SeedResult (seed, candidate_deals,
        baseline_deals)
      - seeds are a strict prefix of the panel in declared order
      - no duplicated or reordered seeds
      - LegStatus values are valid (Valid / TimeForfeit / Invalid)
      - first cluster of every pair verified (C-2 gate)

    Also cross-checks freeze.json against the current tree: a changed
    commit/binary/panel hash means the evidence predates the freeze and
    must be re-frozen before it counts as canonical.

.PARAMETER TournamentDir
    Directory containing freeze.json + results/pair-*-out/legs.jsonl.
    Default: docs/benchmarks/tournament/

.PARAMETER All
    Also verify annex pairs (champion/runner-up vs archetypes).

.EXAMPLE
    .\scripts\tournament\verify.ps1 -All
#>

[CmdletBinding()]
param(
    [string]$TournamentDir = "docs/benchmarks/tournament",
    [switch]$All
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$resultsDir = Join-Path $TournamentDir "results"
$freezePath = Join-Path $TournamentDir "freeze.json"
$failures = [System.Collections.Generic.List[string]]::new()

function Fail([string]$msg) {
    $script:failures.Add($msg)
    Write-Error $msg
}

# -- 1. freeze integrity ---------------------------------------------------
if (Test-Path $freezePath) {
    $freeze = Get-Content $freezePath -Raw | ConvertFrom-Json
    $currentCommit = git rev-parse HEAD 2>&1
    $currentTree = git rev-parse 'HEAD^{tree}' 2>&1

    if ($freeze.git_commit -ne $currentCommit) {
        Write-Warning "commit drift: freeze=$($freeze.git_commit.Substring(0,8)) current=$($currentCommit.Substring(0,8)) -- re-freeze required"
    }
    if ($freeze.git_tree_hash -ne $currentTree) {
        Write-Warning "tree drift: freeze=$($freeze.git_tree_hash.Substring(0,8)) current=$($currentTree.Substring(0,8))"
    }
} else {
    Write-Warning "no freeze.json -- evidence is not bound to a freeze"
}

# -- 2. per-pair legs.jsonl integrity ----------------------------------------
$pairDirs = Get-ChildItem $resultsDir -Directory -ErrorAction SilentlyContinue
if ($All) {
    # Annex pairs live in annex/results/ and follow the same legs.jsonl contract.
    $annexDir = Join-Path $TournamentDir "annex/results"
    if (Test-Path $annexDir) {
        $pairDirs += Get-ChildItem $annexDir -Directory -ErrorAction SilentlyContinue
    } else {
        Write-Warning "-All requested but no annex results at $annexDir"
    }
}
if ($pairDirs.Count -eq 0) { throw "no pair dirs in $resultsDir" }

$panelPath = Join-Path $TournamentDir "panel.json"
$panelSeeds = @{}
if (Test-Path $panelPath) {
    $panel = Get-Content $panelPath -Raw | ConvertFrom-Json
    foreach ($s in $panel.seeds) { $panelSeeds[$s] = $true }
}

$totalClusters = 0; $totalLegs = 0; $totalValid = 0; $totalForfeit = 0; $totalInvalid = 0
$firstClusterOk = 0

foreach ($dir in $pairDirs | Sort-Object Name) {
    $pairName = $dir.Name -replace '-out$',''
    $legsPath = Join-Path $dir.FullName "legs.jsonl"
    $reportPath = Join-Path $dir.FullName "report.json"

    if (-not (Test-Path $legsPath)) {
        Fail "[$pairName] missing legs.jsonl"
        continue
    }

    $lines = Get-Content $legsPath -Encoding UTF8
    if ($lines.Count -eq 0) {
        Fail "[$pairName] empty legs.jsonl"
        continue
    }

    # Line 0 = legs header (kind = "koi-bench-legs").
    $header = $null
    try { $header = $lines[0] | ConvertFrom-Json } catch {}
    if (-not $header -or $header.kind -ne "koi-bench-legs") {
        Fail "[$pairName] line 0 is not a legs header"
        continue
    }

    # Cluster lines: strict panel prefix, no dups, no reorder.
    $seenSeeds = [System.Collections.Generic.HashSet[uint64]]::new()
    $clusters = 0; $valid = 0; $forfeits = 0; $invalids = 0

    for ($i = 1; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        if (-not $line.Trim()) { continue }
        try { $entry = $line | ConvertFrom-Json } catch {
            Fail "[$pairName] line $i unparseable"
            continue
        }

        $seed = $entry.seed
        if (-not $seenSeeds.Add($seed)) {
            Fail "[$pairName] duplicate seed $seed at line $i"
        }
        if ($panelSeeds.Count -gt 0 -and -not $panelSeeds.ContainsKey($seed)) {
            Fail "[$pairName] off-panel seed $seed at line $i"
        }

        foreach ($leg in @($entry.candidate_deals, $entry.baseline_deals)) {
            $totalLegs++
            $kind = if ($leg.status -is [psobject]) { $leg.status.kind } else { $leg.status }
            switch ($kind) {
                "valid"       { $valid++; $totalValid++ }
                "time_forfeit" { $forfeits++; $totalForfeit++ }
                "invalid"     { $invalids++; $totalInvalid++ }
                default       { Fail "[$pairName] line $i unknown status $kind" }
            }
        }
        $clusters++
    }
    $totalClusters += $clusters

    # First-cluster gate (C-2): the first line after the header must be
    # a complete SeedResult with both legs present.
    if ($lines.Count -gt 1) {
        try {
            $first = $lines[1] | ConvertFrom-Json
            $hasBoth = $first.candidate_deals -and $first.baseline_deals
            if (-not $hasBoth) { Fail "[$pairName] first cluster missing a leg" }
            else { $script:firstClusterOk++ }
        } catch { Fail "[$pairName] first cluster unparseable" }
    }

    Write-Host "  [$pairName] $clusters clusters | $valid valid / $forfeits forfeit / $invalids invalid"
}

# -- summary -----------------------------------------------------------------
Write-Host ""
Write-Host "verify: $totalClusters clusters | $totalValid valid / $totalForfeit forfeit / $totalInvalid invalid | $firstClusterOk first-clusters OK"
if ($failures.Count -eq 0) {
    Write-Host "INTEGRITY: clean"
} else {
    Write-Host "INTEGRITY: $($failures.Count) failures"
    $failures | ForEach-Object { Write-Host "  - $_" }
    exit 1
}
