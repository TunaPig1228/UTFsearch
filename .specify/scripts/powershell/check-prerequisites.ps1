param(
    [switch]$Json,
    [switch]$PathsOnly
)
. (Join-Path $PSScriptRoot "common.ps1")
$featureDir = Get-FeatureDir
$payload = [ordered]@{
    FEATURE_DIR  = $featureDir
    FEATURE_SPEC = (Join-Path $featureDir "spec.md")
    IMPL_PLAN    = (Join-Path $featureDir "plan.md")
    TASKS        = (Join-Path $featureDir "tasks.md")
}
$payload | ConvertTo-Json -Compress
