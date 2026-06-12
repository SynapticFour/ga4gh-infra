#Requires -Version 5.1
<#
.SYNOPSIS
  Install the ga4gh-infra all-in-one binary and default native configuration.

.EXAMPLE
  irm https://raw.githubusercontent.com/<org>/ga4gh-infra/main/scripts/install.ps1 | iex

.NOTES
  Environment variables:
    GA4GH_INFRA_REPO          GitHub repo (default: SynapticFour/ga4gh-infra)
    GA4GH_INFRA_VERSION       Release version without v prefix (default: latest)
    GA4GH_INFRA_INSTALL_DIR   Binary directory (default: %LOCALAPPDATA%\Programs\ga4gh-infra)
    GA4GH_INFRA_CONFIG_DIR    Config directory (default: %USERPROFILE%\.config\ga4gh-infra)
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Repo = if ($env:GA4GH_INFRA_REPO) { $env:GA4GH_INFRA_REPO } else { "SynapticFour/ga4gh-infra" }
$Version = $env:GA4GH_INFRA_VERSION
$InstallDir = if ($env:GA4GH_INFRA_INSTALL_DIR) {
    $env:GA4GH_INFRA_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "Programs\ga4gh-infra"
}
$ConfigDir = if ($env:GA4GH_INFRA_CONFIG_DIR) {
    $env:GA4GH_INFRA_CONFIG_DIR
} else {
    Join-Path $env:USERPROFILE ".config\ga4gh-infra"
}
$RawBase = "https://raw.githubusercontent.com/$Repo/main"

function Write-Step($Message) {
    Write-Host "==> $Message"
}

function Get-RustTarget {
    if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64") {
        return "x86_64-pc-windows-msvc"
    }
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
        return "aarch64-pc-windows-msvc"
    }
    throw "Unsupported Windows architecture: $($env:PROCESSOR_ARCHITECTURE)"
}

function Get-ReleaseTag {
    if ($Version) {
        return "ga4gh-infra-v$Version"
    }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    return $release.tag_name
}

function Install-Binary {
    param(
        [string]$Tag,
        [string]$Target
    )

    $asset = "ga4gh-infra-$Target.zip"
    $url = "https://github.com/$Repo/releases/download/$Tag/$asset"
    $tmp = Join-Path $env:TEMP ("ga4gh-infra-" + [guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $tmp | Out-Null

    Write-Step "Downloading $asset from $Tag"
    $zipPath = Join-Path $tmp $asset
    Invoke-WebRequest -Uri $url -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath $tmp -Force

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $exeName = "ga4gh-infra-$Target.exe"
    Copy-Item -Path (Join-Path $tmp $exeName) -Destination (Join-Path $InstallDir "ga4gh-infra.exe") -Force
}

function Install-Config {
    $secretsDir = Join-Path $ConfigDir "secrets"
    New-Item -ItemType Directory -Path $secretsDir -Force | Out-Null

    $configPath = Join-Path $ConfigDir "all-in-one.toml"
    if (-not (Test-Path $configPath)) {
        Write-Step "Writing $configPath"
        $template = (Invoke-WebRequest -Uri "$RawBase/config/all-in-one.native.toml.example").Content
        $template.Replace("{{CONFIG_DIR}}", $ConfigDir.Replace("\", "/")) | Set-Content -Path $configPath -Encoding UTF8
    }

    $envPath = Join-Path $ConfigDir "env"
    if (-not (Test-Path $envPath)) {
        Write-Step "Writing $envPath"
        Invoke-WebRequest -Uri "$RawBase/config/env.native.example" -OutFile $envPath
    }
}

function Generate-Keys {
    Write-Step "Generating signing keys (when missing)"
    $binary = Join-Path $InstallDir "ga4gh-infra.exe"
    & $binary keygen --output-dir (Join-Path $ConfigDir "secrets")
}

$tag = Get-ReleaseTag
$target = Get-RustTarget
Install-Binary -Tag $tag -Target $target
Install-Config
Generate-Keys

Write-Host @"

ga4gh-infra installed to $InstallDir\ga4gh-infra.exe
Configuration directory: $ConfigDir

Next steps:
  1. Add $InstallDir to your PATH if needed.
  2. Edit $ConfigDir\env (secrets and SERVICE_REGISTRY_DATABASE_URL).
  3. ga4gh-infra all-in-one --config $ConfigDir\all-in-one.toml

For a PostgreSQL-free demo stack, use Docker instead (see justfile / docker/README.md).

"@
