# Build + publish the Windows quick_stats binary (the one the LinkMesh
# desktop bundles for win). Run on a Windows dev machine.
#
# What it does:
#   1. downloads the pinned windows libtorch (2.2.2 CPU — the exact
#      artifact LinkMesh's `yarn download:deps` bundles for win) into
#      .\libtorch-win (gitignored)
#   2. builds quick_stats with cargo for the default windows target
#   3. stages it as builds\quick_stats-<version>-win-x64.exe (version =
#      the Cargo.toml version — that is the registry key)
#   4. uploads it to this project's generic package registry, the pinned
#      location LinkMesh's `yarn download:deps` fetches:
#        https://gitlab.byte.lu/api/v4/projects/14/packages/generic/quick_stats/<version>/
#   5. copies it into the LinkMesh repo's libs\win\ so the DEV flow works
#      immediately (QuickStatsController's non-packaged path resolves the
#      pinned artifact from libs\win without waiting on the registry)
#
# Note: a tch-based binary cannot be cross-compiled to windows from linux
# (the doc_processor .exe artifacts are manual uploads for this same
# reason), so windows stays a manual build — exactly like mac.
#
# Usage (PowerShell):
#   $env:LINKMESH_DIR = "$HOME\git_repos\linkmesh"
#   $env:GITLAB_TOKEN = "glpat-..."   # Developer+ on this project (14);
#                                     # without it the build + copy still
#                                     # happen, the upload is skipped with
#                                     # a warning (linkmesh win PACKAGING
#                                     # would fail at staging until then)
#   .\publish_windows.ps1
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $MyInvocation.MyCommand.Path)   # repo root

$Version = (Select-String -Path "Cargo.toml" -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
if (-not $Version) { throw "could not read version from Cargo.toml" }

$LinkMeshDir = if ($env:LINKMESH_DIR) { $env:LINKMESH_DIR } else { "$HOME\git_repos\linkmesh" }
$LibTorchVersion = "2.2.2"
$GitLabUrl = if ($env:GITLAB_URL) { $env:GITLAB_URL } else { "https://gitlab.byte.lu" }
$ProjectId = if ($env:PROJECT_ID) { $env:PROJECT_ID } else { "14" }   # olivier/quick_stats

# --- 1) pinned windows libtorch -----------------------------------------
$LibTorchDir = ".\libtorch-win\libtorch"
if (-not (Test-Path $LibTorchDir)) {
    $zipName = "libtorch-win-shared-with-deps-$LibTorchVersion+cpu.zip"
    Write-Host "Downloading $zipName ..."
    Invoke-WebRequest "https://download.pytorch.org/libtorch/cpu/$zipName" -OutFile ".\$zipName"
    Expand-Archive ".\$zipName" -DestinationPath ".\libtorch-win"
    Remove-Item ".\$zipName"
}

# --- 2) build -----------------------------------------------------------
# The doc_processor precedent (linkmesh _dev_ops/build_doc_processor_windows.ps1)
# builds with the default windows toolchain: MSVC if Visual Studio Build
# Tools are installed, mingw otherwise (Cargo.toml carries the mingw
# linker config). Both work against the windows libtorch zip.
$Env:LIBTORCH = (Resolve-Path $LibTorchDir).Path
$Env:Path += ";$Env:LIBTORCH\lib"
$Env:LIBTORCH_BYPASS_VERSION_CHECK = "1"
Write-Host "Building quick_stats with cargo (release, LIBTORCH=$Env:LIBTORCH)..."
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# Respect a custom CARGO_TARGET_DIR (dev machines may share one rust target
# dir); default is cargo's own target dir.
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { ".\target" }

# --- 3) stage ------------------------------------------------------------
New-Item -ItemType Directory -Force -Path ".\builds" | Out-Null
$ArtifactName = "quick_stats-$Version-win-x64.exe"
Copy-Item "$TargetDir\release\quick_stats.exe" ".\builds\$ArtifactName"
Write-Host "Staged builds\$ArtifactName"

# --- 4) registry upload (skippable, loud if skipped without a token) -----
if ($env:GITLAB_TOKEN) {
    $url = "$GitLabUrl/api/v4/projects/$ProjectId/packages/generic/quick_stats/$Version/$ArtifactName"
    Write-Host "Uploading $ArtifactName -> $url"
    & curl.exe -sfS -H "PRIVATE-TOKEN: $env:GITLAB_TOKEN" --upload-file ".\builds\$ArtifactName" $url
    if ($LASTEXITCODE -ne 0) { throw "registry upload failed for $ArtifactName" }
} else {
    Write-Warning "GITLAB_TOKEN not set — the win binary was NOT uploaded to the registry. The LinkMesh dev flow (libs\win) IS updated, but win PACKAGING will fail at staging until this script runs with a token."
}

# --- 5) dev-flow copy into the LinkMesh libs\ cache ----------------------
$dest = Join-Path $LinkMeshDir "libs\win"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item ".\builds\$ArtifactName" (Join-Path $dest $ArtifactName)
Write-Host "Copied $ArtifactName -> $dest"

Write-Host "done — quick_stats $Version windows artifact built."
