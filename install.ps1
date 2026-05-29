# Continuum installer for Windows (PowerShell)
# Usage:
#   irm https://raw.githubusercontent.com/NiteshYadavPataudi/continuum/main/install.ps1 | iex
#
# Environment variables:
#   CONTINUUM_VERSION   Version to install, or "latest" (default: latest)
#   CONTINUUM_INSTALL_DIR / INSTALL_DIR  Cargo install root (default: $HOME\.local\continuum)

$ErrorActionPreference = "Stop"

function Write-Info($msg) { Write-Host $msg -ForegroundColor Cyan }
function Write-Warn($msg) { Write-Host $msg -ForegroundColor Yellow }
function Write-Ok($msg)   { Write-Host $msg -ForegroundColor Green }
function Write-Err($msg)  { Write-Host $msg -ForegroundColor Red }

function Add-ToCurrentPath([string]$PathToAdd) {
    $segments = @($env:PATH -split ';' | Where-Object { $_ -and $_.Trim() })
    if ($segments -notcontains $PathToAdd) {
        $env:PATH = ($segments + $PathToAdd) -join ';'
    }
}

function Add-ToUserPath([string]$PathToAdd) {
    $currentUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $parts = @()
    if ($currentUserPath) {
        $parts = @($currentUserPath -split ';' | Where-Object { $_ -and $_.Trim() })
    }
    if ($parts -notcontains $PathToAdd) {
        $newUserPath = (@($parts + $PathToAdd) | Where-Object { $_ -and $_.Trim() }) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
        return $true
    }
    return $false
}

$InstallDir = if ($env:CONTINUUM_INSTALL_DIR) { $env:CONTINUUM_INSTALL_DIR } elseif ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $HOME ".local\continuum" }
$Version = if ($env:CONTINUUM_VERSION) { $env:CONTINUUM_VERSION } else { "latest" }

Write-Info "Installing Continuum..."
Write-Host ""

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Err "Rust/Cargo was not found."
    Write-Host "Install Rust from: https://rustup.rs"
    exit 1
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

Write-Info "Install root: $InstallDir"
Write-Info "Version: $Version"
Write-Host ""

$cargoArgs = @("install", "--locked", "--root", $InstallDir, "continuum-cli")
if ($Version -ne "latest") {
    $cargoArgs = @("install", "--locked", "--root", $InstallDir, "continuum-cli", "--version", $Version)
}

Write-Info "Building continuum-cli..."
& cargo @cargoArgs

$binDir = $InstallDir
if ($env:PATH -notlike "*$binDir*") {
    Write-Host ""
    Write-Warn "Adding $binDir to PATH..."
    Add-ToCurrentPath $binDir
    $added = Add-ToUserPath $binDir
    if ($added) {
        Write-Ok "Updated your user PATH."
        Write-Host "Open a new terminal to pick up the permanent PATH change."
    } else {
        Write-Ok "PATH already included $binDir."
    }
}

Write-Host ""
Write-Ok "Installation complete!"
Write-Host ""
Write-Host "Verify installation:"
Write-Host "  continuum --version"
Write-Host "  continuum doctor"
Write-Host ""
