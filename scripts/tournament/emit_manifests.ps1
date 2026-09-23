<#
.SYNOPSIS
    emit_manifests.ps1 — expands one tournament manifest into the shared
    anomaly-free panel + one pairwise manifest per unordered entrant pair.

.DESCRIPTION
    Thin wrapper over `koi-bench tournament-emit`. The emitter validates
    every pairing, expands the shared panel (Generated → deterministic
    anomaly-free seeds), and writes:
      - panel.json           (seed list + FNV-1a/64 hash → freeze binding)
      - manifests/pair-*.json (one per unordered pair, identical protocol)

.PARAMETER TournamentManifest
    Path to the tournament manifest (roster + shared protocol block).

.PARAMETER OutDir
    Destination for panel.json and manifests/. Default:
    docs/benchmarks/tournament/

.PARAMETER Release
    Use the release-profile binary (default true for canonical work).

.EXAMPLE
    .\scripts\tournament\emit_manifests.ps1 `
        -TournamentManifest docs/benchmarks/tournament/manifest.json
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$TournamentManifest,
    [string]$OutDir = "docs/benchmarks/tournament",
    [bool]$Release = $true
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$profile = if ($Release) { "release" } else { "debug" }
$binary = "target/$profile/tournament_emit.exe"
if (-not (Test-Path $binary)) { $binary = "target/$profile/tournament_emit" }
if (-not (Test-Path $binary)) { throw "tournament_emit binary not found — build $profile first" }

Write-Host "emitting tournament manifests → $OutDir"
& $binary --tournament-manifest $TournamentManifest --out-dir $OutDir
if ($LASTEXITCODE -ne 0) { throw "tournament_emit failed (exit $LASTEXITCODE)" }

$pairCount = (Get-ChildItem "$OutDir/manifests/pair-*.json" -ErrorAction SilentlyContinue).Count
$panelJson = Get-Content "$OutDir/panel.json" -Raw | ConvertFrom-Json
Write-Host "done: $pairCount pair manifests + panel ($($panelJson.cluster_count) clusters)"
