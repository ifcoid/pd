use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::pdf_ops::viewer::render_page_to_rgba;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportImageFormat {
    Png,
    Jpeg,
}

impl ExportImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportImageFormat::Png => "png",
            ExportImageFormat::Jpeg => "jpg",
        }
    }
}

/// Exports a single PDF page to an image file (PNG or JPEG).
pub fn export_pdf_page_to_image(
    pdf_path: &Path,
    page_num: usize,
    output_path: &Path,
    format: ExportImageFormat,
    scale: f32,
) -> Result<PathBuf> {
    let page_data = render_page_to_rgba(pdf_path, page_num, scale)?;

    match format {
        ExportImageFormat::Png => {
            image::save_buffer_with_format(
                output_path,
                &page_data.rgba_bytes,
                page_data.width,
                page_data.height,
                image::ExtendedColorType::Rgba8,
                image::ImageFormat::Png,
            )
            .with_context(|| format!("Gagal menyimpan PNG ke {:?}", output_path))?;
        }
        ExportImageFormat::Jpeg => {
            // Convert RGBA to RGB for JPEG
            let mut rgb_bytes =
                Vec::with_capacity((page_data.width * page_data.height * 3) as usize);
            for chunk in page_data.rgba_bytes.chunks_exact(4) {
                rgb_bytes.extend_from_slice(&chunk[0..3]);
            }
            image::save_buffer_with_format(
                output_path,
                &rgb_bytes,
                page_data.width,
                page_data.height,
                image::ExtendedColorType::Rgb8,
                image::ImageFormat::Jpeg,
            )
            .with_context(|| format!("Gagal menyimpan JPEG ke {:?}", output_path))?;
        }
    }

    Ok(output_path.to_path_buf())
}

/// Exports specific pages of a PDF to images inside an output directory.
pub fn export_pdf_pages_to_dir(
    pdf_path: &Path,
    pages: &[usize],
    output_dir: &Path,
    format: ExportImageFormat,
    scale: f32,
) -> Result<Vec<PathBuf>> {
    let base_name = pdf_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    let ext = format.extension();
    let mut exported_files = Vec::with_capacity(pages.len());

    for &page_idx in pages {
        let file_name = format!("{}_page_{:03}.{}", base_name, page_idx, ext);
        let out_path = output_dir.join(file_name);
        export_pdf_page_to_image(pdf_path, page_idx, &out_path, format, scale)?;
        exported_files.push(out_path);
    }

    Ok(exported_files)
}

/// Exports all pages of a PDF to images inside an output directory.
pub fn export_all_pdf_pages_to_images(
    pdf_path: &Path,
    output_dir: &Path,
    format: ExportImageFormat,
    scale: f32,
) -> Result<Vec<PathBuf>> {
    let first_page = render_page_to_rgba(pdf_path, 1, scale)?;
    let total_pages = first_page.total_pages;
    let all_pages: Vec<usize> = (1..=total_pages).collect();
    export_pdf_pages_to_dir(pdf_path, &all_pages, output_dir, format, scale)
}

