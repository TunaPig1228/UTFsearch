param([switch]$Json)
. (Join-Path $PSScriptRoot "common.ps1")
$featureDir = Get-FeatureDir
$payload = [ordered]@{
    FEATURE_SPEC = (Join-Path $featureDir "spec.md")
    IMPL_PLAN    = (Join-Path $featureDir "plan.md")
    SPECS_DIR    = $featureDir
    BRANCH       = Split-Path $featureDir -Leaf
    HAS_GIT      = [bool](Test-Path (Join-Path $RepoRoot ".git"))
}
$payload | ConvertTo-Json -Compress
