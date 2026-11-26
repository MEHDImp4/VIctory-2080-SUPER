# Starts the title with the settings this fork is played with (README.md, "How to run it").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -Profile      Default or Rtx2080Super, which limits speculative uploads near the VRAM budget
# -Prepare      translate shaders and warm known pipelines, then exit
# -RecordPipelines  record the pipelines requested by draws for the next warm-up
# -LogPath      save a diagnostic log in addition to the console output
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Build        target\diag (the default, the build all measurements are of) or target\release
# -More         other settings: -More @{VICTORY_PANEL='1'}
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$RayTracing,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{},
    [ValidateSet('Default', 'Rtx2080Super')][string]$Profile = 'Default',
    [switch]$Prepare,
    [switch]$RecordPipelines,
    [string]$LogPath
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'
}
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
}
if ($Profile -eq 'Rtx2080Super') {
    $settings.VICTORY_UPLOAD_AHEAD_VRAM_PERCENT = '85'
    $settings.VICTORY_DISPLAY_MODE = '1920x1080'
    $settings.VICTORY_RAM_COMMIT_AWARE = '1'
    $settings.VICTORY_POOL_HEADROOM = '1'
    $settings.VICTORY_PIPELINE_WAIT_MS = '120'
}
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
if ($RecordPipelines) { $settings.VICTORY_PIPELINE_BOOK = 'used' }
if ($LogPath) {
    $settings.VICTORY_LOG_FILE = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($LogPath)
    $settings.VICTORY_LOG = 'info'
}
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (README.md)."; return }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; return }
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR'
Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$Exe = (Resolve-Path $Exe).Path
Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
try {
    if ($Prepare) {
        $gameDir = Split-Path -LiteralPath $Exe
        $shaderSources = @($Exe)
        foreach ($name in 'shaders.rpf', 'Bink2Scarlett.dll') {
            $source = Join-Path $gameDir $name
            if (Test-Path -LiteralPath $source -PathType Leaf) { $shaderSources += $source }
        }
        & $victory shaders @shaderSources --jobs 4
        if ($LASTEXITCODE -ne 0) { throw "Shader preparation failed (exit $LASTEXITCODE)." }
        & $victory warm --used --jobs 2
        if ($LASTEXITCODE -ne 0) { throw "Pipeline preparation failed (exit $LASTEXITCODE)." }
    } else {
        & $victory run $Exe --execute
    }
} finally { Pop-Location }
