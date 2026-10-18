<#
.SYNOPSIS
    Installer script for PD - Desktop PDF Utility on Windows.
.DESCRIPTION
    Downloads and installs the latest binary release of PD from GitHub,
    configures user PATH, and optionally creates a Start Menu shortcut.
#>

[CmdletBinding()]
param (
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\pd",
    [string]$Repo = "ifcoid/pd"
)

$ErrorActionPreference = 'Stop'

function Write-Step ($message) {
    Write-Host "[PD] " -ForegroundColor Cyan -NoNewline
    Write-Host $message
}

function Write-Success ($message) {
    Write-Host "[✓] " -ForegroundColor Green -NoNewline
    Write-Host $message -ForegroundColor Green
}

function Write-WarningMsg ($message) {
    Write-Host "[!] " -ForegroundColor Yellow -NoNewline
    Write-Host $message -ForegroundColor Yellow
}

function Write-ErrorMsg ($message) {
    Write-Host "[✗] " -ForegroundColor Red -NoNewline
    Write-Host $message -ForegroundColor Red
}

Write-Host @"
=========================================================
      PD - Desktop PDF Utility Installer for Windows
=========================================================
"@ -ForegroundColor Cyan

# 1. Check Architecture
if ([Environment]::Is64BitOperatingSystem -ne $true) {
    Write-ErrorMsg "Hanya sistem 64-bit (x86_64) yang didukung saat ini."
    exit 1
}

# 2. Prepare Install Directory
if (!(Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ZipPath = Join-Path $env:TEMP "pd-windows-x86_64.zip"
$ExePath = Join-Path $InstallDir "pd.exe"

# 3. Query GitHub Releases API
$ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"
Write-Step "Memeriksa rilis terbaru dari GitHub ($Repo)..."

$DownloadUrl = $null
$TagName = "latest"

try {
    $Headers = @{ "User-Agent" = "PowerShell-PD-Installer" }
    $Release = Invoke-RestMethod -Uri $ApiUrl -Headers $Headers -Method Get
    $TagName = $Release.tag_name

    foreach ($asset in $Release.assets) {
        if ($asset.name -like "*windows*.zip") {
            $DownloadUrl = $asset.browser_download_url
            break
        }
    }
} catch {
    Write-WarningMsg "Gagal mengambil data dari GitHub Release API ($($_.Exception.Message))."
}

# 4. Download and Extract or Local Fallback
if ($DownloadUrl) {
    Write-Step "Mengunduh versi $TagName dari $DownloadUrl..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing

    Write-Step "Mengekstrak binary ke $InstallDir..."
    Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force
    Remove-Item -Path $ZipPath -Force -ErrorAction SilentlyContinue
} else {
    Write-WarningMsg "Asset rilis binary belum tersedia di GitHub Releases."
    
    # Check if local cargo build exists
    $LocalTarget = ".\target\release\pd.exe"
    $LocalDebug = ".\target\debug\pd.exe"
    
    if (Test-Path $LocalTarget) {
        Write-Step "Menggunakan build lokal dari $LocalTarget..."
        Copy-Item -Path $LocalTarget -Destination $ExePath -Force
    } elseif (Test-Path $LocalDebug) {
        Write-Step "Menggunakan build lokal dari $LocalDebug..."
        Copy-Item -Path $LocalDebug -Destination $ExePath -Force
    } elseif (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Step "Mencoba kompilasi langsung via cargo..."
        cargo build --release
        if (Test-Path $LocalTarget) {
            Copy-Item -Path $LocalTarget -Destination $ExePath -Force
        }
    } else {
        Write-ErrorMsg "Tidak dapat menemukan file binary maupun compiler Cargo."
        exit 1
    }
}

if (!(Test-Path $ExePath)) {
    Write-ErrorMsg "File binary pd.exe tidak ditemukan di $InstallDir."
    exit 1
}

# 5. Add to User PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallDir*") {
    Write-Step "Menambahkan $InstallDir ke User PATH..."
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$env:Path;$InstallDir"
    Write-Success "User PATH berhasil diperbarui."
} else {
    Write-Step "InstallDir sudah terdaftar di PATH."
}

# 6. Create Start Menu Shortcut
try {
    $WshShell = New-Object -ComObject WScript.Shell
    $StartMenuDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::StartMenu)
    $ProgramsDir = Join-Path $StartMenuDir "Programs"
    $ShortcutPath = Join-Path $ProgramsDir "PD - PDF Utility.lnk"
    
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $ExePath
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.Description = "PD - Desktop PDF Utility"
    $Shortcut.Save()
    Write-Success "Pintasan Start Menu berhasil dibuat: $ShortcutPath"
} catch {
    Write-WarningMsg "Tidak dapat membuat pintasan Start Menu: $($_.Exception.Message)"
}

Write-Host ""
Write-Success "Instalasi selesai! PD - Desktop PDF Utility siap digunakan."
Write-Host "Anda dapat menjalankan aplikasi dengan mengetik 'pd' di terminal atau melalui Start Menu." -ForegroundColor Gray
Write-Host ""
