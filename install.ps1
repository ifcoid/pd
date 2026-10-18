# Installer script for PD - Desktop PDF Utility on Windows

[CmdletBinding()]
param (
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\pd",
    [string]$Repo = "ifcoid/pd"
)

$ErrorActionPreference = 'Stop'

function Write-Step {
    param([string]$Message)
    Write-Host "[PD] " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Write-SuccessMsg {
    param([string]$Message)
    Write-Host "[OK] " -ForegroundColor Green -NoNewline
    Write-Host $Message -ForegroundColor Green
}

function Write-WarningMsg {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message -ForegroundColor Yellow
}

function Write-ErrorMsg {
    param([string]$Message)
    Write-Host "[ERR] " -ForegroundColor Red -NoNewline
    Write-Host $Message -ForegroundColor Red
}

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "      PD - Desktop PDF Utility Installer for Windows     " -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

# 1. Check Architecture
if ([Environment]::Is64BitOperatingSystem -ne $true) {
    Write-ErrorMsg "Hanya sistem 64-bit (x86_64) yang didukung saat ini."
    exit 1
}

# 2. Prepare Install Directory
if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ZipPath = Join-Path $env:TEMP "pd-windows-x86_64.zip"
$ExePath = Join-Path $InstallDir "pd.exe"
$IsUpdate = Test-Path -Path $ExePath

if ($IsUpdate) {
    Write-Step "Aplikasi pd sudah terpasang. Memulai proses update ke versi terbaru..."
    $RunningPd = Get-Process -Name "pd" -ErrorAction SilentlyContinue
    if ($RunningPd) {
        Write-Step "Menutup aplikasi pd yang sedang berjalan agar binary dapat diperbarui..."
        $RunningPd | Stop-Process -Force
        Start-Sleep -Milliseconds 500
    }
} else {
    Write-Step "Memulai instalasi baru pd..."
}

# 3. Query GitHub Releases API
$ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"
Write-Step "Memeriksa rilis terbaru dari GitHub ($Repo)..."

$DownloadUrl = $null
$TagName = "latest"

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $Headers = @{ "User-Agent" = "PD-Installer-PowerShell" }
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
    Write-Step "Mengunduh versi $TagName ($DownloadUrl)..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing

    Write-Step "Mengekstrak binary ke $InstallDir..."
    Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force
    Remove-Item -Path $ZipPath -Force -ErrorAction SilentlyContinue
} else {
    Write-WarningMsg "Asset rilis binary belum tersedia di GitHub Releases."
    
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

if (-not (Test-Path $ExePath)) {
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
    Write-SuccessMsg "User PATH berhasil diperbarui."
} else {
    Write-Step "InstallDir sudah terdaftar di PATH."
}

# 6. Create Start Menu Shortcut
try {
    $WshShell = New-Object -ComObject WScript.Shell
    $StartMenuDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::StartMenu)
    $ProgramsDir = Join-Path $StartMenuDir "Programs"
    $ShortcutPath = Join-Path $ProgramsDir "pd.lnk"
    
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $ExePath
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.Description = "pd - Ultra-Lightweight PDF Reader & Utility"
    $Shortcut.Save()
    Write-SuccessMsg "Pintasan Start Menu berhasil dibuat: $ShortcutPath"
} catch {
    Write-WarningMsg "Tidak dapat membuat pintasan Start Menu: $($_.Exception.Message)"
}

# 7. Asosiasikan ekstensi .pdf dengan PD secara otomatis (User Level)
Write-Step "Mengonfigurasi asosiasi file .pdf untuk otomatis dibuka menggunakan PD..."
try {
    $ProgId = "pd.pdf"
    $DocDesc = "PDF Document (PD - Ultra-Lightweight Reader)"
    $Command = "`"$ExePath`" `"%1`""

    # Daftarkan ProgID di HKCU:\Software\Classes
    New-Item -Path "HKCU:\Software\Classes\$ProgId" -Value $DocDesc -Force | Out-Null
    New-Item -Path "HKCU:\Software\Classes\$ProgId\DefaultIcon" -Value "$ExePath,0" -Force | Out-Null
    New-Item -Path "HKCU:\Software\Classes\$ProgId\shell\open\command" -Value $Command -Force | Out-Null

    # Daftarkan ekstensi .pdf di HKCU:\Software\Classes
    New-Item -Path "HKCU:\Software\Classes\.pdf" -Value $ProgId -Force | Out-Null
    New-Item -Path "HKCU:\Software\Classes\.pdf\OpenWithProgids" -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Classes\.pdf\OpenWithProgids" -Name $ProgId -Value "" -Force

    # Daftarkan di App Paths (memungkinkan 'pd' dan 'pd.exe' dijalankan dari Run dialog / CLI)
    New-Item -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\pd.exe" -Value $ExePath -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\pd.exe" -Name "Path" -Value $InstallDir -Force

    # Daftarkan di Applications
    New-Item -Path "HKCU:\Software\Classes\Applications\pd.exe\shell\open\command" -Value $Command -Force | Out-Null
    New-Item -Path "HKCU:\Software\Classes\Applications\pd.exe\SupportedTypes" -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Classes\Applications\pd.exe\SupportedTypes" -Name ".pdf" -Value "" -Force

    # Daftarkan preferensi di Explorer FileExts
    $FileExtsPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf"
    if (Test-Path $FileExtsPath) {
        New-Item -Path "$FileExtsPath\OpenWithList" -Force | Out-Null
        Set-ItemProperty -Path "$FileExtsPath\OpenWithList" -Name "a" -Value "pd.exe" -Force
        Set-ItemProperty -Path "$FileExtsPath\OpenWithList" -Name "MRUList" -Value "a" -Force

        New-Item -Path "$FileExtsPath\OpenWithProgids" -Force | Out-Null
        Set-ItemProperty -Path "$FileExtsPath\OpenWithProgids" -Name $ProgId -Value "" -Force
    }

    # Beritahu Windows Shell bahwa asosiasi file telah diperbarui
    $code = @'
    [System.Runtime.InteropServices.DllImport("shell32.dll")]
    public static extern void SHChangeNotify(int wEventId, int uFlags, int dwItem1, int dwItem2);
'@
    try {
        $type = Add-Type -MemberDefinition $code -Name "ShellNotification" -Namespace "PD" -PassThru -ErrorAction SilentlyContinue
        if ($type) {
            $type::SHChangeNotify(0x08000000, 0, 0, 0) # SHCNE_ASSOCCHANGED
        }
    } catch {}

    Write-SuccessMsg "Ekstensi file .pdf berhasil diasosiasikan untuk dibuka otomatis dengan PD!"
} catch {
    Write-WarningMsg "Tidak dapat mengatur asosiasi otomatis file .pdf: $($_.Exception.Message)"
}

# 8. Daftarkan Menu Klik Kanan (Context Menu ala WinRAR) untuk File Gambar
Write-Step "Mendaftarkan menu klik kanan Windows Explorer untuk file gambar..."
try {
    $ImgExtensions = @(".png", ".jpg", ".jpeg", ".webp", ".bmp")
    $MenuNameConvert = "Konversi ke PDF dengan PD"
    $MenuNameOpen = "Buka di PD (Gambar ke PDF)..."
    $CmdConvert = "`"$ExePath`" --to-pdf `"%1`""
    $CmdOpen = "`"$ExePath`" `"%1`""

    # Daftarkan pada SystemFileAssociations\image
    $SysImgPath = "HKCU:\Software\Classes\SystemFileAssociations\image\shell"
    New-Item -Path "$SysImgPath\PD.Convert" -Value $MenuNameConvert -Force | Out-Null
    Set-ItemProperty -Path "$SysImgPath\PD.Convert" -Name "Icon" -Value "$ExePath,0" -Force
    New-Item -Path "$SysImgPath\PD.Convert\command" -Value $CmdConvert -Force | Out-Null

    New-Item -Path "$SysImgPath\PD.Open" -Value $MenuNameOpen -Force | Out-Null
    Set-ItemProperty -Path "$SysImgPath\PD.Open" -Name "Icon" -Value "$ExePath,0" -Force
    New-Item -Path "$SysImgPath\PD.Open\command" -Value $CmdOpen -Force | Out-Null

    # Daftarkan per ekstensi gambar untuk kompatibilitas maksimal
    foreach ($ext in $ImgExtensions) {
        $ExtShellPath = "HKCU:\Software\Classes\SystemFileAssociations\$ext\shell"
        New-Item -Path "$ExtShellPath\PD.Convert" -Value $MenuNameConvert -Force | Out-Null
        Set-ItemProperty -Path "$ExtShellPath\PD.Convert" -Name "Icon" -Value "$ExePath,0" -Force
        New-Item -Path "$ExtShellPath\PD.Convert\command" -Value $CmdConvert -Force | Out-Null

        New-Item -Path "$ExtShellPath\PD.Open" -Value $MenuNameOpen -Force | Out-Null
        Set-ItemProperty -Path "$ExtShellPath\PD.Open" -Name "Icon" -Value "$ExePath,0" -Force
        New-Item -Path "$ExtShellPath\PD.Open\command" -Value $CmdOpen -Force | Out-Null
    }

    Write-SuccessMsg "Menu klik kanan untuk file gambar (PNG, JPG, WebP, BMP) berhasil didaftarkan!"
} catch {
    Write-WarningMsg "Tidak dapat mendaftarkan menu klik kanan: $($_.Exception.Message)"
}

Write-Host ""
if ($IsUpdate) {
    Write-SuccessMsg "Update selesai! PD berhasil diperbarui ke versi $TagName."
} else {
    Write-SuccessMsg "Instalasi selesai! PD - Desktop PDF Utility siap digunakan."
}
Write-Host "Anda dapat menjalankan aplikasi dengan mengetik 'pd' di terminal, melalui Start Menu, atau langsung double-click file PDF apapun!" -ForegroundColor Gray
Write-Host ""

