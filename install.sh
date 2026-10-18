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

# Check PATH
echo ""
echo -e "${GREEN}[✓] Berhasil menginstal PD di: ${INSTALL_DIR}/pd${NC}"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}[!] Catatan: Direktori ${INSTALL_DIR} belum ada di PATH Anda.${NC}"
    echo "Silakan tambahkan perintah berikut ke file konfigurasi shell Anda (~/.bashrc atau ~/.zshrc):"
    echo -e "${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
fi

echo ""
echo -e "${GREEN}${BOLD}Instalasi selesai! Jalankan aplikasi dengan mengetik:${NC} pd"
