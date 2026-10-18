use anyhow::{Context, Result};
use lopdf::Document;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CompressionReport {
    pub original_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub bytes_saved: i64,
    pub percentage_saved: f64,
}

/// Compresses a PDF file using stream compression and modern object stream packaging.
pub fn compress_pdf(input_path: &Path, output_path: &Path) -> Result<CompressionReport> {
    let original_size_bytes = std::fs::metadata(input_path)
        .with_context(|| format!("Gagal membaca metadata file {:?}", input_path))?
        .len();

    let mut doc = Document::load(input_path)
        .with_context(|| format!("Gagal memuat dokumen PDF {:?}", input_path))?;

    // 1. Compress all internal stream objects
    doc.compress();

    // 2. Remove orphaned and unreferenced objects
    doc.prune_objects();

    // 3. Renumber objects sequentially to remove fragmentation
    doc.renumber_objects();

    // 4. Save using modern object streams (PDF 1.5+ Object Streams)
    {
        let mut out_file = File::create(output_path)
            .with_context(|| format!("Gagal membuat file output {:?}", output_path))?;
        doc.save_modern(&mut out_file)
            .with_context(|| format!("Gagal menulis dokumen terkompresi ke {:?}", output_path))?;
    }

    let compressed_size_bytes = std::fs::metadata(output_path)
        .with_context(|| format!("Gagal membaca metadata hasil kompresi {:?}", output_path))?
        .len();

    let bytes_saved = original_size_bytes as i64 - compressed_size_bytes as i64;
    let percentage_saved = if original_size_bytes > 0 {
        (bytes_saved as f64 / original_size_bytes as f64) * 100.0
    } else {
        0.0
    };

    Ok(CompressionReport {
        original_size_bytes,
        compressed_size_bytes,
        bytes_saved,
        percentage_saved,
    })
}
