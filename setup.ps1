<#
.SYNOPSIS
    Builds Mic Switch from source and installs it for the current user.

.DESCRIPTION
    1. Installs Rust (rustup) if it is missing.
    2. Checks for the MSVC C++ build tools and offers to install them with winget.
    3. Builds the release binary.
    4. Copies MicSwitch.exe to the install folder and creates Desktop / Start menu shortcuts.

    No administrator rights are needed.

.PARAMETER InstallDir
    Where MicSwitch.exe is installed. Default: %LOCALAPPDATA%\Programs\MicSwitch
    If this folder is writable, settings are stored next to the exe in a "data" folder.

.PARAMETER RustDir
    Install Rust under this folder instead of %USERPROFILE% (useful when C: is short on space).
    Sets RUSTUP_HOME and CARGO_HOME for the current user.

.PARAMETER NoShortcut
    Do not create Desktop / Start menu shortcuts.

.PARAMETER Uninstall
    Remove the installed app, its shortcuts, its settings, and the "Start with Windows" entry.

.EXAMPLE
    .\setup.ps1
.EXAMPLE
    .\setup.ps1 -InstallDir D:\Apps\MicSwitch -RustDir D:\dev
.EXAMPLE
    .\setup.ps1 -Uninstall
#>
[CmdletBinding()]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\MicSwitch'),
    [string]$RustDir,
    [switch]$NoShortcut,
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$AppName = 'Mic Switch'
$ExeName = 'MicSwitch.exe'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$RunValue = 'MicSwitch'
$RepoRoot = $PSScriptRoot
$Shortcuts = @(
    (Join-Path ([Environment]::GetFolderPath('Desktop')) "$AppName.lnk"),
    (Join-Path ([Environment]::GetFolderPath('Programs')) "$AppName.lnk")
)

function Write-Step([string]$Message) { Write-Host "==> $Message" -ForegroundColor Cyan }
function Write-Ok([string]$Message) { Write-Host "    $Message" -ForegroundColor Green }
function Write-Note([string]$Message) { Write-Host "    $Message" -ForegroundColor Yellow }

function Stop-App {
    Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($ExeName)) -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}

# ---------------------------------------------------------------- uninstall
if ($Uninstall) {
    Write-Step "Uninstalling $AppName"
    Stop-App
    Remove-ItemProperty -Path $RunKey -Name $RunValue -ErrorAction SilentlyContinue
    $Shortcuts | Where-Object { Test-Path $_ } | Remove-Item -Force
    foreach ($dir in @($InstallDir, (Join-Path $env:LOCALAPPDATA 'MicSwitch'))) {
        if (Test-Path $dir) { Remove-Item $dir -Recurse -Force; Write-Ok "Removed $dir" }
    }
    Write-Ok 'Done. Rust and the C++ build tools were left installed.'
    return
}

# ---------------------------------------------------------------- checks
if ([Environment]::OSVersion.Version.Build -lt 17763) {
    throw "$AppName needs Windows 10 version 1809 or later."
}

$webView2Keys = @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
)
if (-not ($webView2Keys | Where-Object { Test-Path $_ })) {
    Write-Note 'Microsoft Edge WebView2 Runtime was not found. Windows 11 ships with it; on Windows 10 install it from:'
    Write-Note 'https://developer.microsoft.com/microsoft-edge/webview2/'
}

# ---------------------------------------------------------------- Rust
Write-Step 'Checking Rust'
if ($RustDir) {
    $env:RUSTUP_HOME = Join-Path $RustDir 'rustup'
    $env:CARGO_HOME = Join-Path $RustDir 'cargo'
    [Environment]::SetEnvironmentVariable('RUSTUP_HOME', $env:RUSTUP_HOME, 'User')
    [Environment]::SetEnvironmentVariable('CARGO_HOME', $env:CARGO_HOME, 'User')
}
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$env:PATH = "$(Join-Path $cargoHome 'bin');$env:PATH"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Note 'Rust was not found. Installing it with rustup (minimal profile)...'
    $downloadDir = if ($RustDir) { $RustDir } else { $env:TEMP }
    New-Item -ItemType Directory -Force $downloadDir | Out-Null
    $rustupInit = Join-Path $downloadDir 'rustup-init.exe'
    Invoke-WebRequest 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $rustupInit
    & $rustupInit -y --default-toolchain stable --profile minimal
    if ($LASTEXITCODE -ne 0) { throw 'rustup installation failed.' }
    Remove-Item $rustupInit -ErrorAction SilentlyContinue
}
Write-Ok (& cargo --version)

# ---------------------------------------------------------------- MSVC build tools
Write-Step 'Checking the C++ build tools'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$hasMsvc = (Test-Path $vswhere) -and
    (& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if (-not $hasMsvc) {
    Write-Note 'The MSVC C++ build tools are required to compile Rust on Windows.'
    $answer = Read-Host '    Install "Visual Studio Build Tools" (C++ workload, several GB) with winget now? [y/N]'
    if ($answer -notmatch '^[yY]') {
        throw 'Install "Desktop development with C++" from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and run setup again.'
    }
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw 'winget was not found. Install the build tools manually from https://visualstudio.microsoft.com/visual-cpp-build-tools/'
    }
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements `
        --override '--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
    if ($LASTEXITCODE -ne 0) { throw 'Build tools installation failed.' }
}
Write-Ok 'Found'

# ---------------------------------------------------------------- build
Write-Step 'Building (the first build downloads dependencies and takes a few minutes)'
Push-Location $RepoRoot
try {
    & cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
} finally {
    Pop-Location
}

# ---------------------------------------------------------------- install
Write-Step "Installing to $InstallDir"
Stop-App
New-Item -ItemType Directory -Force $InstallDir | Out-Null
$exe = Join-Path $InstallDir $ExeName
Copy-Item (Join-Path $RepoRoot 'target\release\mic-switch.exe') $exe -Force
Write-Ok $exe

# Keep "Start with Windows" pointing at the new location
if (Get-ItemProperty -Path $RunKey -Name $RunValue -ErrorAction SilentlyContinue) {
    Set-ItemProperty -Path $RunKey -Name $RunValue -Value "`"$exe`" --hidden"
}

if (-not $NoShortcut) {
    $shell = New-Object -ComObject WScript.Shell
    foreach ($path in $Shortcuts) {
        $link = $shell.CreateShortcut($path)
        $link.TargetPath = $exe
        $link.WorkingDirectory = $InstallDir
        $link.IconLocation = "$exe,0"
        $link.Description = 'Switch the default microphone and speaker'
        $link.Save()
        Write-Ok "Shortcut: $path"
    }
}

Start-Process $exe
Write-Step "Done. $AppName is running (look for its icon in the system tray)."
