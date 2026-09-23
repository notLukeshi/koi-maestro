<#
.SYNOPSIS
    watchdog.ps1 — monitors field-phase lanes: scans legs.jsonl
    heartbeats, detects stalled pairs, marks poisoned clusters.

.DESCRIPTION
    Periodically scans every pair's legs.jsonl for:
      - heartbeat stall: no new cluster appended within the stall window
      - worker timeout clusters (LegStatus::TimeForfeit)
      - invalid legs (LegStatus::Invalid)
      - torn tail lines (crash mid-write — checkpoint handles these on
        resume, but the watchdog reports them)

    Stalled pairs are NOT restarted here — the watchdog only reports
    them; launch_field.ps1 resumes pairs automatically via checkpoint.
    Poisoned clusters (≥ 2 consecutive forfeits on one seed) are marked
    in a sidecar file so verify.ps1 can report them separately.

.PARAMETER TournamentDir
    Directory containing results/pair-*-out/legs.jsonl.
    Default: docs/benchmarks/tournament/

.PARAMETER StallMinutes
    Minutes without a new cluster before a pair is flagged stalled.
    Default: 10 (legs are ~10 s at production lanes; a 10-min stall
    means something is wrong).

.PARAMETER IntervalSeconds
    Poll interval between scans. Default: 60.

.PARAMETER Once
    Run one scan and exit (for interactive checks). Default: continuous.

.EXAMPLE
    .\scripts\tournament\watchdog.ps1 -StallMinutes 10 -Once
#>

[CmdletBinding()]
param(
    [string]$TournamentDir = "docs/benchmarks/tournament",
    [int]$StallMinutes = 10,
    [int]$IntervalSeconds = 60,
    [switch]$Once
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$resultsDir = Join-Path $TournamentDir "results"
if (-not (Test-Path $resultsDir)) { throw "no results dir: $resultsDir" }

$stallThreshold = [TimeSpan]::FromMinutes($StallMinutes)
$scan = {
    $now = Get-Date
    $report = @()
    $stalled = @()
    $poisoned = @()

    foreach ($pairDir in Get-ChildItem $resultsDir -Directory) {
        $legsPath = Join-Path $pairDir.FullName "legs.jsonl"
        $pairName = $pairDir.Name -replace '-out$',''
        if (-not (Test-Path $legsPath)) {
            $report += [pscustomobject]@{
                pair = $pairName; status = "no-legs"; clusters = 0
                lastWrite = $null; stall = $null
            }
            continue
        }

        $lines = Get-Content $legsPath -Encoding UTF8
        $lastWrite = (Get-Item $legsPath).LastWriteTime
        $stall = $now - $lastWrite
        $isStalled = $stall -gt $stallThreshold -and $lines.Count -gt 0

        # Parse cluster statuses: count forfeits + invalids per line.
        # LegStatus serializes as {"kind": "valid"|"time_forfeit"|"invalid"}.
        $forfeits = 0; $invalids = 0; $poisonedSeeds = @{}
        foreach ($line in $lines) {
            if (-not $line.Trim()) { continue }
            try {
                $entry = $line | ConvertFrom-Json
            } catch { continue }  # torn tail — checkpoint truncates it
            foreach ($leg in @($entry.candidate_deals, $entry.baseline_deals)) {
                $kind = if ($leg.status -is [psobject]) { $leg.status.kind } else { $leg.status }
                if ($kind -eq "time_forfeit") { $forfeits++ }
                if ($kind -eq "invalid") { $invalids++ }
            }
            # Both legs forfeiting the same seed = poisoned.
            $seedKey = $entry.seed
            $cd = $entry.candidate_deals.status; $bd = $entry.baseline_deals.status
            $cdk = if ($cd -is [psobject]) { $cd.kind } else { $cd }
            $bdk = if ($bd -is [psobject]) { $bd.kind } else { $bd }
            if ($cdk -eq "time_forfeit" -and $bdk -eq "time_forfeit") {
                $poisonedSeeds[$seedKey] = $true
            }
        }

        $status = if ($isStalled) { "STALLED" } else { "running" }
        if ($poisonedSeeds.Count -gt 0) { $poisoned += $pairName }
        if ($isStalled) { $stalled += $pairName }

        $report += [pscustomobject]@{
            pair = $pairName; status = $status
            clusters = $lines.Count; forfeits = $forfeits; invalids = $invalids
            lastWrite = $lastWrite; stall = if ($isStalled) { $stall } else { $null }
        }
    }

    # Emit poisoned-cluster sidecar for verify.ps1.
    if ($poisoned.Count -gt 0) {
        $poisonPath = Join-Path $TournamentDir "poisoned-clusters.txt"
        $poisoned | Set-Content $poisonPath -Encoding UTF8
    }

    [pscustomobject]@{
        timestamp = $now
        pairs = $report
        stalled = $stalled
        poisoned = $poisoned
    }
}

Write-Host "watchdog: scanning $resultsDir (stall > ${StallMinutes}m, poll ${IntervalSeconds}s)"
do {
    $result = & $scan
    $time = $result.timestamp.ToString("HH:mm:ss")

    $running = ($result.pairs | Where-Object status -eq "running").Count
    $stalled = ($result.pairs | Where-Object status -eq "STALLED").Count
    $idle = ($result.pairs | Where-Object status -eq "no-legs").Count
    $totalClusters = ($result.pairs | Measure-Object clusters -Sum).Sum

    Write-Host "[$time] pairs: $running running / $stalled stalled / $idle idle | clusters: $totalClusters"

    foreach ($pair in $result.pairs | Where-Object { $_.status -ne "no-legs" }) {
        $flag = switch ($pair.status) {
            "STALLED" { "  !! STALLED" }
            default   { "" }
        }
        Write-Host "    $($pair.pair): $($pair.clusters) clusters ($($pair.forfeits)F $($pair.invalids)I)$flag"
    }

    if ($result.poisoned.Count -gt 0) {
        Write-Warning "poisoned clusters: $($result.poisoned -join ', ')"
    }
    if ($result.stalled.Count -gt 0) {
        Write-Warning "stalled pairs: $($result.stalled -join ', ') — relaunch launch_field.ps1 to resume"
    }

    if (-not $Once) { Start-Sleep -Seconds $IntervalSeconds }
} while (-not $Once)
