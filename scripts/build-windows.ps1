param(
    [switch]$SkipPreflight
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root
$OutputDir = Join-Path $Root 'INSTALLER_OUTPUT'
$BuildLogDir = Join-Path $Root 'build-logs'
New-Item -ItemType Directory -Path $OutputDir, $BuildLogDir -Force | Out-Null
$LogFile = Join-Path $BuildLogDir ('windows-installer-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.log')
$transcriptStarted = $false

function Assert-Command([string]$Name, [string]$Url) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing build prerequisite: $Name . Install from $Url and reopen PowerShell."
    }
}

function Invoke-Checked([string]$Label, [string]$Command, [string[]]$Arguments) {
    Write-Host "`n== $Label ==" -ForegroundColor Cyan
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE. See $LogFile"
    }
}

try {
    Start-Transcript -Path $LogFile -Force | Out-Null
    $transcriptStarted = $true
    Write-Host 'AI Research PDF Reader - Windows NSIS installer builder' -ForegroundColor Cyan
    Write-Host "Project: $Root"

    Assert-Command 'node' 'https://nodejs.org/'
    Assert-Command 'npm' 'https://nodejs.org/'
    Assert-Command 'cargo' 'https://rustup.rs/'
    Assert-Command 'rustc' 'https://rustup.rs/'

    $nodeVersion = (& node --version).TrimStart('v')
    $nodeMajor = [int]($nodeVersion.Split('.')[0])
    if ($nodeMajor -lt 22) {
        throw "Node.js 22 or newer is required. Current: $nodeVersion"
    }

    if (-not $SkipPreflight) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path $vswhere)) {
            throw 'Visual Studio 2022 C++ Build Tools not detected. Install Desktop development with C++: https://visualstudio.microsoft.com/visual-cpp-build-tools/'
        }
        $vsInstallation = & $vswhere -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if (-not $vsInstallation) {
            throw 'MSVC C++ tools missing. Use Visual Studio Installer -> Modify -> Desktop development with C++.'
        }
        Write-Host "MSVC: $vsInstallation" -ForegroundColor Green
    }

    Invoke-Checked 'Install JavaScript dependencies' 'npm' @('install', '--no-audit', '--no-fund')
    Invoke-Checked 'Compile React/TypeScript frontend' 'npm' @('run', 'build')
    Invoke-Checked 'Compile Rust/Tauri and package Windows NSIS EXE' 'npm' @('run', 'tauri:build')

    $bundleDir = Join-Path $Root 'src-tauri\target\release\bundle\nsis'
    if (-not (Test-Path $bundleDir)) {
        throw "Build completed but NSIS output directory not found: $bundleDir"
    }
    $installers = @(Get-ChildItem -Path $bundleDir -Filter '*.exe' -File)
    if ($installers.Count -eq 0) {
        throw "Build completed but no NSIS .exe found in $bundleDir"
    }
    foreach ($installer in $installers) {
        $target = Join-Path $OutputDir $installer.Name
        Copy-Item $installer.FullName $target -Force
        Write-Host "Installer: $target" -ForegroundColor Green
    }
    Write-Host "`nBuild succeeded. Installers are in $OutputDir" -ForegroundColor Green
    Write-Host "Build log: $LogFile"
    exit 0
}
catch {
    Write-Host "`nBuild failed: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Build log: $LogFile" -ForegroundColor Yellow
    exit 1
}
finally {
    if ($transcriptStarted) { Stop-Transcript | Out-Null }
}
