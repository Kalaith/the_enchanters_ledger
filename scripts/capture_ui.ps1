<#
.SYNOPSIS
    Headless screenshot harness for The Enchanter's Ledger.

.DESCRIPTION
    Thin wrapper around the shared macroquad-toolkit capture script. Builds the
    debug exe and drives it through the env-var capture hook
    (THE_ENCHANTERS_LEDGER_CAPTURE_*) provided by macroquad_toolkit::capture in
    src/main.rs. Scenes seed a GamePhase via Game::begin_capture_scene in
    src/game.rs: "title" (main menu), "naming" (name-entry screen), and the
    default "gameplay" (jumps straight into the workshop).

.EXAMPLE
    ./scripts/capture_ui.ps1
    ./scripts/capture_ui.ps1 -Frames 60 -SkipBuild
#>
param(
    [string[]]$Scenes = @("title", "naming", "gameplay"),
    [int]$Frames = 150,
    [string]$OutputDir = "docs\verification",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$gameDir = Split-Path -Parent $PSScriptRoot
$shared = Join-Path (Split-Path -Parent $gameDir) "macroquad-toolkit\scripts\capture_ui.ps1"

& $shared -GameDir $gameDir -Scenes $Scenes -Frames $Frames -OutputDir $OutputDir -SkipBuild:$SkipBuild
