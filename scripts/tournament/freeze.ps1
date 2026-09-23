<#
.SYNOPSIS
    freeze.ps1 -- the canonical-evidence gate. Must pass before leg 1 of
    the field phase.

.DESCRIPTION
    Binds every provenance the section-0.7 contract requires into freeze.json:
    git commit + tree hash, release-binary SHA-256, per-entrant config
    digests, blueprint artifact hash, leaf-model hash, panel seed hash,
    ruleset, stopping spec, lane count.

    REFUSES on a dirty working tree -- canonical evidence from a dirty
    build is worthless by definition.

.PARAMETER TournamentManifest
    Path to the tournament manifest (roster + shared protocol block).

.PARAMETER Panel
    Path to panel.json emitted by tournament_emit (contains the expanded
    anomaly-free seed list + its FNV-1a/64 hash).

.PARAMETER LaneCount
    Production lane count fixed by pilot measurement (M-04).

.PARAMETER BlueprintArtifact
    Optional path to a certified blueprint gadget the resolving entrants
    consume; its SHA-256 is recorded.

.PARAMETER LeafModel
    Optional path to the trained ONNX leaf model; its SHA-256 is recorded.
    Absent = no leaf entrant in the field.

.PARAMETER Out
    Where freeze.json lands (default: docs/benchmarks/tournament/).

.EXAMPLE
    .\scripts\tournament\freeze.ps1 `
        -TournamentManifest docs/benchmarks/tournament/manifest.json `
        -Panel docs/benchmarks/tournament/panel.json `
        -LaneCount 4 `
        -BlueprintArtifact artifacts/blueprint.gadget `
        -LeafModel runs/leaf/model.onnx
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$TournamentManifest,
    [Parameter(Mandatory)][string]$Panel,
    [Parameter(Mandatory)][int]$LaneCount,
    [string]$BlueprintArtifact,
    [string]$LeafModel,
    [string]$Out = "docs/benchmarks/tournament"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Sha256([string]$Path) {
    if (-not (Test-Path $Path)) { throw "missing file: $Path" }
    (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLower()
}

# -- 1. Dirty-tree gate ----------------------------------------------------
$status = git status --porcelain 2>&1
if ($LASTEXITCODE -ne 0) { throw "git status failed: $status" }
if ($status) {
    Write-Error "DIRTY TREE -- canonical evidence requires a clean tree.`n$status"
    exit 1
}

$commit = git rev-parse HEAD
$treeHash = git rev-parse 'HEAD^{tree}'

# -- 2. Release binary hash --------------------------------------------------
$benchBinary = "target/release/koi-bench.exe"
if (-not (Test-Path $benchBinary)) {
    $benchBinary = "target/release/koi-bench"  # Linux fallback
}
$binarySha256 = Sha256 $benchBinary

# -- 3. Tournament manifest digest -------------------------------------------
$manifestSha256 = Sha256 $TournamentManifest
$manifestJson = Get-Content $TournamentManifest -Raw | ConvertFrom-Json

# -- 4. Panel hash ------------------------------------------------------------
$panelSha256 = Sha256 $Panel
$panelJson = Get-Content $Panel -Raw | ConvertFrom-Json
$panelHash = $panelJson.panel_hash_fx64

# -- 5. Optional artifact hashes ------------------------------------------------
$blueprintHash = if ($BlueprintArtifact) { Sha256 $BlueprintArtifact } else { $null }
$leafHash = if ($LeafModel) { Sha256 $LeafModel } else { $null }

# -- 6. Per-entrant config digests ----------------------------------------------
$entrantDigests = @{}
foreach ($entrant in $manifestJson.entrants) {
    $configJson = $entrant.config | ConvertTo-Json -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($configJson)
    $hash = [System.BitConverter]::ToString(
        [System.Security.Cryptography.SHA256]::Create().ComputeHash($bytes)
    ).Replace("-", "").ToLower()
    $entrantDigests[$entrant.label] = $hash
}

# -- 7. Emit freeze.json ---------------------------------------------------------
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$freeze = [ordered]@{
    frozen_at_utc       = (Get-Date).ToUniversalTime().ToString("o")
    git_commit          = $commit
    git_tree_hash       = $treeHash
    binary_sha256       = $binarySha256
    binary_path         = $benchBinary
    manifest_sha256     = $manifestSha256
    panel_sha256        = $panelSha256
    panel_hash_fx64     = $panelHash
    blueprint_sha256    = $blueprintHash
    leaf_model_sha256   = $leafHash
    ruleset             = $manifestJson.ruleset
    stopping            = $manifestJson.stopping
    lane_count          = $LaneCount
    entrant_config_sha256 = $entrantDigests
}

$freezePath = Join-Path $Out "freeze.json"
$freeze | ConvertTo-Json -Depth 8 | Set-Content $freezePath -Encoding UTF8

Write-Host "freeze.json written: $freezePath"
Write-Host "  commit: $($commit.Substring(0,8))  tree: $($treeHash.Substring(0,8))"
Write-Host "  binary: $binarySha256"
Write-Host "  lanes:  $LaneCount  entrants: $($manifestJson.entrants.Count)"
if ($leafHash) { Write-Host "  leaf:   $leafHash" }
Write-Host ""
Write-Host "Field phase is now unlocked. Any source/config/model change requires re-freeze."
