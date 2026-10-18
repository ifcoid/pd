#!/usr/bin/env bash
#
# Installer script for PD - Desktop PDF Utility on Linux & macOS
#
set -euo pipefail

REPO="ifcoid/pd"
INSTALL_DIR="${HOME}/.local/bin"

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}${BOLD}=========================================================${NC}"
echo -e "${CYAN}${BOLD}      PD - Desktop PDF Utility Installer (Linux/macOS)   ${NC}"
echo -e "${CYAN}${BOLD}=========================================================${NC}"

# 1. Detect OS & Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64)
                ASSET_PATTERN="linux-x86_64.tar.gz"
                ;;
            *)
                echo -e "${RED}[✗] Arsitektur $ARCH belum didukung secara resmi untuk Linux.${NC}"
                exit 1
                ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64)
                ASSET_PATTERN="macos-x86_64.tar.gz"
                ;;
            arm64|aarch64)
                ASSET_PATTERN="macos-aarch64.tar.gz"
                ;;
            *)
                echo -e "${RED}[✗] Arsitektur $ARCH belum didukung untuk macOS.${NC}"
                exit 1
                ;;
        esac
        ;;
    *)
        echo -e "${RED}[✗] Sistem operasi $OS tidak didukung oleh installer ini.${NC}"
        exit 1
        ;;
esac

mkdir -p "$INSTALL_DIR"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

IS_UPDATE=false
if [ -f "${INSTALL_DIR}/pd" ]; then
    IS_UPDATE=true
    echo -e "${CYAN}[PD] Aplikasi pd sudah terpasang. Memulai proses update ke versi terbaru...${NC}"
    if pgrep -x "pd" >/dev/null 2>&1; then
        echo -e "${YELLOW}[PD] Menutup instans pd yang sedang berjalan agar dapat diperbarui...${NC}"
        pkill -x "pd" || true
        sleep 0.5
    fi
else
    echo -e "${CYAN}[PD] Memulai instalasi baru pd...${NC}"
fi

DOWNLOAD_URL=""
LATEST_TAG=""

echo -e "${CYAN}[PD] Memeriksa rilis terbaru dari GitHub (${REPO})...${NC}"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

if command -v curl >/dev/null 2>&1; then
    RELEASE_DATA="$(curl -fsSL "$API_URL" || true)"
elif command -v wget >/dev/null 2>&1; then
    RELEASE_DATA="$(wget -qO- "$API_URL" || true)"
else
    RELEASE_DATA=""
fi

if [ -n "$RELEASE_DATA" ]; then
    LATEST_TAG="$(echo "$RELEASE_DATA" | grep '"tag_name":' | head -1 | cut -d '"' -f 4 || true)"
    DOWNLOAD_URL="$(echo "$RELEASE_DATA" | grep 'browser_download_url' | grep "$ASSET_PATTERN" | head -1 | cut -d '"' -f 4 || true)"
fi

if [ -n "$DOWNLOAD_URL" ]; then
    echo -e "${CYAN}[PD] Mengunduh versi ${LATEST_TAG} ($ASSET_PATTERN)...${NC}"
    ARCHIVE_PATH="${TEMP_DIR}/pd.tar.gz"
    
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH"
    else
        wget -qO "$ARCHIVE_PATH" "$DOWNLOAD_URL"
    fi

    echo -e "${CYAN}[PD] Mengekstrak binary ke ${INSTALL_DIR}...${NC}"
    tar -xzf "$ARCHIVE_PATH" -C "$TEMP_DIR"
    cp "${TEMP_DIR}/pd" "${INSTALL_DIR}/pd"
    chmod +x "${INSTALL_DIR}/pd"
else
    echo -e "${YELLOW}[!] Asset rilis binary belum tersedia di GitHub Releases.${NC}"
    if [ -f "./target/release/pd" ]; then
        echo -e "${CYAN}[PD] Menggunakan binary lokal ./target/release/pd...${NC}"
        cp "./target/release/pd" "${INSTALL_DIR}/pd"
        chmod +x "${INSTALL_DIR}/pd"
    elif command -v cargo >/dev/null 2>&1; then
        echo -e "${CYAN}[PD] Melakukan build langsung menggunakan Cargo...${NC}"
        cargo build --release
        cp "./target/release/pd" "${INSTALL_DIR}/pd"
        chmod +x "${INSTALL_DIR}/pd"
    else
        echo -e "${RED}[✗] Tidak dapat menemukan binary rilis maupun Rust/Cargo.${NC}"
        exit 1
    fi
fi

# 2. Add to User PATH automatically
echo ""
echo -e "${GREEN}[✓] Berhasil menginstal PD di: ${INSTALL_DIR}/pd${NC}"

PATH_ADDED=false
for RC_FILE in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$RC_FILE" ]; then
        if ! grep -q "$INSTALL_DIR" "$RC_FILE"; then
            echo -e "\n# PD Desktop PDF Utility" >> "$RC_FILE"
            echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$RC_FILE"
            PATH_ADDED=true
            echo -e "${GREEN}[✓] Menambahkan ${INSTALL_DIR} ke ${RC_FILE}${NC}"
        fi
    fi
done

if [ "$PATH_ADDED" = false ] && [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}[!] Catatan: Direktori ${INSTALL_DIR} belum ada di PATH Anda saat ini.${NC}"
    echo "Silakan tambahkan perintah berikut ke shell config Anda: export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# 3. Automatic .pdf file association
echo -e "${CYAN}[PD] Mengonfigurasi asosiasi file .pdf untuk dibuka otomatis dengan PD...${NC}"

if [ "$OS" = "Linux" ]; then
    APP_DIR="$HOME/.local/share/applications"
    mkdir -p "$APP_DIR"
    
    cat <<EOF > "$APP_DIR/pd.desktop"
[Desktop Entry]
Name=pd
Comment=pd - Ultra-lightweight Pure-Rust PDF Reader & Desktop Utility
Exec=${INSTALL_DIR}/pd %U
Terminal=false
Type=Application
Icon=application-pdf
MimeType=application/pdf;
Categories=Office;Viewer;Graphics;
StartupNotify=true
EOF
    chmod +x "$APP_DIR/pd.desktop"

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    fi

    # Set as default for application/pdf using xdg-mime
    if command -v xdg-mime >/dev/null 2>&1; then
        xdg-mime default pd.desktop application/pdf 2>/dev/null || true
    fi

    # Also register in mimeapps.list
    MIME_LIST="$HOME/.config/mimeapps.list"
    if [ -f "$MIME_LIST" ]; then
        if grep -q "\[Default Applications\]" "$MIME_LIST"; then
            sed -i '/^application\/pdf=/d' "$MIME_LIST" 2>/dev/null || true
            sed -i '/\[Default Applications\]/a application/pdf=pd.desktop;' "$MIME_LIST" 2>/dev/null || true
        else
            echo -e "\n[Default Applications]\napplication/pdf=pd.desktop;" >> "$MIME_LIST"
        fi
    else
        mkdir -p "$HOME/.config"
        cat <<EOF > "$MIME_LIST"
[Default Applications]
application/pdf=pd.desktop;

[Added Associations]
application/pdf=pd.desktop;
EOF
    fi

    # Nautilus right-click scripts
    NAUTILUS_SCRIPTS="$HOME/.local/share/nautilus/scripts"
    mkdir -p "$NAUTILUS_SCRIPTS"
    cat <<EOF > "$NAUTILUS_SCRIPTS/Konversi ke PDF (PD)"
#!/usr/bin/env bash
${INSTALL_DIR}/pd --to-pdf "\$@"
EOF
    chmod +x "$NAUTILUS_SCRIPTS/Konversi ke PDF (PD)"

    # KDE Dolphin ServiceMenu
    KDE_SERVICES="$HOME/.local/share/kio/servicemenus"
    mkdir -p "$KDE_SERVICES"
    cat <<EOF > "$KDE_SERVICES/pd_convert.desktop"
[Desktop Entry]
Type=Service
ServiceTypes=KonqPopupMenu/Plugin
MimeType=image/png;image/jpeg;image/webp;image/bmp;
Actions=pdConvert;pdOpen;
X-KDE-Submenu=PD - PDF Utility

[Desktop Action pdConvert]
Name=Konversi ke PDF dengan PD
Icon=application-pdf
Exec=${INSTALL_DIR}/pd --to-pdf %F

[Desktop Action pdOpen]
Name=Buka di PD (Gambar ke PDF)...
Icon=application-pdf
Exec=${INSTALL_DIR}/pd %F
EOF

    # Nautilus PDF to Image script
    cat <<EOF > "$NAUTILUS_SCRIPTS/Konversi PDF ke Gambar (PD)"
#!/usr/bin/env bash
${INSTALL_DIR}/pd --to-png "\$@"
EOF
    chmod +x "$NAUTILUS_SCRIPTS/Konversi PDF ke Gambar (PD)"

    # KDE Dolphin PDF to Image ServiceMenu
    cat <<EOF > "$KDE_SERVICES/pd_pdf_to_image.desktop"
[Desktop Entry]
Type=Service
ServiceTypes=KonqPopupMenu/Plugin
MimeType=application/pdf;
Actions=pdToPng;pdToJpg;pdOpenImages;
X-KDE-Submenu=PD - PDF Utility

[Desktop Action pdToPng]
Name=Konversi PDF ke Gambar (PNG)
Icon=image-png
Exec=${INSTALL_DIR}/pd --to-png %F

[Desktop Action pdToJpg]
Name=Konversi PDF ke Gambar (JPG)
Icon=image-jpeg
Exec=${INSTALL_DIR}/pd --to-jpg %F

[Desktop Action pdOpenImages]
Name=Buka di PD (PDF ke Gambar)...
Icon=application-pdf
Exec=${INSTALL_DIR}/pd --gui-pdf-to-images %F
EOF

    echo -e "${GREEN}[✓] File .pdf dan menu klik kanan (Gambar & PDF) berhasil didaftarkan di Linux!${NC}"

elif [ "$OS" = "Darwin" ]; then
    MAC_APP_DIR="$HOME/Applications/pd.app"
    mkdir -p "$MAC_APP_DIR/Contents/MacOS" "$MAC_APP_DIR/Contents/Resources"
    cat <<'EOF' > "$MAC_APP_DIR/Contents/MacOS/pd_launcher"
#!/bin/bash
exec "$HOME/.local/bin/pd" "$@"
EOF
    chmod +x "$MAC_APP_DIR/Contents/MacOS/pd_launcher"

    cat <<EOF > "$MAC_APP_DIR/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>pd</string>
    <key>CFBundleDisplayName</key>
    <string>pd</string>
    <key>CFBundleIdentifier</key>
    <string>id.co.if.pd</string>
    <key>CFBundleVersion</key>
    <string>0.1.2</string>
    <key>CFBundleExecutable</key>
    <string>pd_launcher</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>PDF Document</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Owner</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>pdf</string>
            </array>
            <key>CFBundleTypeMIMETypes</key>
            <array>
                <string>application/pdf</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF
    if [ -x "/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister" ]; then
        /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -f "$MAC_APP_DIR" >/dev/null 2>&1 || true
    fi
    if command -v duti >/dev/null 2>&1; then
        duti -s id.co.if.pd pdf all >/dev/null 2>&1 || true
    fi
    echo -e "${GREEN}[✓] File .pdf berhasil diasosiasikan dengan PD di macOS (~/Applications/PD.app)!${NC}"
fi

echo ""
if [ "$IS_UPDATE" = true ]; then
    echo -e "${GREEN}${BOLD}Update selesai! PD berhasil diperbarui ke versi ${LATEST_TAG}.${NC}"
else
    echo -e "${GREEN}${BOLD}Instalasi selesai! PD - Desktop PDF Utility siap digunakan.${NC}"
fi
echo "Jalankan 'pd' di terminal atau langsung double-click file PDF apapun untuk membukanya!"
echo ""


