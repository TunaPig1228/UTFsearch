$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path

function Get-FeatureDir {
    $featureJson = Join-Path $RepoRoot ".specify\feature.json"
    if (-not (Test-Path $featureJson)) {
        throw "Missing .specify/feature.json — run /speckit-specify first"
    }
    $obj = Get-Content -Raw $featureJson | ConvertFrom-Json
    return (Join-Path $RepoRoot $obj.feature_directory)
}
