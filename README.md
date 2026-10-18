# PD - Desktop PDF Utility Application (Rust + Iced)

[![Build & Release](https://github.com/ifcoid/pd/actions/workflows/release.yml/badge.svg)](https://github.com/ifcoid/pd/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/ifcoid/pd?color=blue&label=Release)](https://github.com/ifcoid/pd/releases/latest)
[![Website](https://img.shields.io/badge/Website-if.co.id%2Fpd-10b981)](https://if.co.id/pd/)
[![License](https://img.shields.io/badge/License-MIT-purple)](https://github.com/ifcoid/pd/blob/main/LICENSE)

🌐 **Official Website:** [https://if.co.id/pd/](https://if.co.id/pd/)  
🐙 **GitHub Repository:** [https://github.com/ifcoid/pd](https://github.com/ifcoid/pd)

---

## 1. Project Overview
Membangun aplikasi desktop *cross-platform* berperforma tinggi menggunakan **Rust** dan pustaka GUI **Iced**. Aplikasi ini dirancang untuk bekerja secara lokal (offline-first) guna memproses dokumen PDF, terinspirasi dari fungsionalitas inti [iLovePDF](https://www.ilovepdf.com/).

## 2. Target Platforms & Tech Stack
* **Language:** Rust (Stable)
* **GUI Framework:** `iced` (menggunakan arsitektur bergaya *The Elm Architecture*)
* **File Dialog & OS Integration:** `rfd` (Native File Dialogs)
* **PDF Processing Crates:** 
  * `lopdf` (untuk manipulasi struktur, *Merge*, *Split*)
  * `pdfium-render` (untuk *Rendering* halaman, konversi PDF ke Gambar, ekstraksi teks)
* **Target OS:** Windows, Linux, macOS

---

## 3. Core Features (Minimum Viable Product / MVP)
Fokus awal pengembangan dibagi menjadi tiga fitur dasar yang paling sering digunakan:

1. **Merge PDF:**
   * Menggabungkan beberapa file PDF terpilih menjadi satu dokumen utuh.
   * Mendukung pengaturan urutan file (*reorder*).
2. **Split PDF:**
   * Memecah file PDF berdasarkan rentang halaman (*page ranges*) tertentu menjadi dokumen terpisah.
3. **Compress PDF:**
   * Mengurangi ukuran file PDF dengan optimasi objek internal (*object streams*).

---

## 4. Architectural Design & State Management (Iced)
Aplikasi harus mengikuti pola fungsional Iced (`State`, `Message`, `Update`, `View`):

* **`State` (Model):**
  * Menyimpan daftar file PDF yang sedang dipilih (`Vec<PathBuf>`).
  * Menyimpan status proses saat ini (`Idle`, `Processing`, `Success`, `Error`).
  * Menyimpan opsi konfigurasi fitur (misalnya: rentang halaman untuk *split*).
* **`Message` (Actions):**
  * Tombol interaksi pengguna (`SelectFiles`, `RemoveFile`, `RunMerge`, `RunCompress`, dll.).
  * *Async Tasks callback* untuk penanganan proses I/O file agar UI tidak *freeze*.
* **`View` (UI Layout):**
  * Desain bersih ala *dashboard* dengan area *drag-and-drop* file, panel tombol aksi di sebelah kanan/bawah, serta *progress bar* saat proses berjalan.

---

## 5. Project Directory Structure
```text
pdf-desktop-app/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs         # Titik masuk aplikasi & Inisialisasi Iced Loop
    ├── app.rs          # Definisi State, Message, dan logika Update/View
    ├── pdf_ops/        # Modul logika pengolahan PDF (Rust backend)
    │   ├── mod.rs
    │   ├── merger.rs   # Integrasi lopdf untuk merge
    │   ├── splitter.rs # Logika split halaman
    │   └── compressor.rs # Logika kompresi ukuran file
    └── components/     # Kustom widget Iced jika diperlukan
```

---

## 6. Instalasi & Cara Menjalankan

### Cara Cepat (One-Line Installer)
* **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/ifcoid/pd/main/install.ps1 | iex
  ```
* **Linux & macOS (Shell):**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/ifcoid/pd/main/install.sh | bash
  ```

### Menjalankan dari Kode Sumber (Development)
Pastikan compiler Rust (`rustc` & `cargo`) telah terinstal di perangkat Anda:
```bash
# Menjalankan unit & integration tests
cargo test

# Menjalankan aplikasi secara langsung
cargo run

# Membangun binary rilis optimal
cargo build --release
```