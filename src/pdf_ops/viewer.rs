use anyhow::{Context, Result};
use hayro::{
    hayro_interpret::InterpreterSettings,
    hayro_syntax::Pdf,
    render, PixmapSettings, RenderCache, RenderSettings,
    vello_cpu::color::palette::css::WHITE,
};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PageViewData {
    pub page_number: usize, // 1-based
    pub total_pages: usize,
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Vec<u8>,
}

/// Renders a single PDF page to RGBA pixels using the ultra-lightweight pure-Rust engine.
pub fn render_page_to_rgba(
    pdf_path: &Path,
    page_number: usize, // 1-based
    scale: f32,
) -> Result<PageViewData> {
    let file_bytes = std::fs::read(pdf_path)
        .with_context(|| format!("Gagal membaca file {:?}", pdf_path))?;

    let pdf = Pdf::new(Arc::new(file_bytes))
        .map_err(|e| anyhow::anyhow!("Gagal memuat struktur PDF: {:?}", e))?;

    let pages = pdf.pages();
    let total_pages = pages.len();
    if total_pages == 0 {
        anyhow::bail!("Dokumen PDF tidak memiliki halaman.");
    }

    let zero_based_idx = if page_number == 0 {
        0
    } else {
        (page_number - 1).min(total_pages - 1)
    };

    let page = &pages[zero_based_idx];
    let cache = RenderCache::new();

    let pixmap = render(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &PixmapSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
        },
    );

    let width = pixmap.width() as u32;
    let height = pixmap.height() as u32;
    let rgba_bytes = pixmap.data_as_u8_slice().to_vec();

    Ok(PageViewData {
        page_number: zero_based_idx + 1,
        total_pages,
        width,
        height,
        rgba_bytes,
    })
}
