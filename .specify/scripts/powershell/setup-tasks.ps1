param([switch]$Json)
. (Join-Path $PSScriptRoot "common.ps1")
$featureDir = Get-FeatureDir
$docs = @()
foreach ($name in @("research.md", "data-model.md", "quickstart.md", "plan.md", "spec.md")) {
    if (Test-Path (Join-Path $featureDir $name)) { $docs += $name }
}
if (Test-Path (Join-Path $featureDir "contracts")) { $docs += "contracts/" }
$payload = [ordered]@{
    FEATURE_DIR     = $featureDir
    TASKS_TEMPLATE  = (Join-Path $RepoRoot ".specify\templates\tasks-template.md")
    AVAILABLE_DOCS  = $docs
}
$payload | ConvertTo-Json -Compress
