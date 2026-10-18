use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::Path;

/// Parses a page range string such as "1-3, 5, 7-10" or "1, 3, 5" into a deduplicated list of 1-based page numbers.
pub fn parse_page_ranges(range_str: &str, total_pages: usize) -> Result<Vec<u32>> {
    let clean = range_str.trim();
    if clean.is_empty() {
        anyhow::bail!("Rentang halaman tidak boleh kosong.");
    }

    let mut pages = Vec::new();
    let parts = clean.split(&[',', ';'][..]);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((start_str, end_str)) = part.split_once('-') {
            let start: u32 = start_str
                .trim()
                .parse()
                .with_context(|| format!("Nomor halaman awal tidak valid: '{}'", start_str))?;
            let end: u32 = end_str
                .trim()
                .parse()
                .with_context(|| format!("Nomor halaman akhir tidak valid: '{}'", end_str))?;

            if start == 0 || end == 0 {
                anyhow::bail!("Nomor halaman harus dimulai dari 1.");
            }
            if start > end {
                anyhow::bail!(
                    "Rentang halaman terbalik: {}-{} (halaman awal harus <= akhir)",
                    start,
                    end
                );
            }

            for p in start..=end {
                if p as usize > total_pages {
                    anyhow::bail!(
                        "Halaman {} melebihi total halaman dokumen ({} halaman).",
                        p,
                        total_pages
                    );
                }
                pages.push(p);
            }
        } else {
            let p: u32 = part
                .parse()
                .with_context(|| format!("Nomor halaman tidak valid: '{}'", part))?;
            if p == 0 {
                anyhow::bail!("Nomor halaman harus dimulai dari 1.");
            }
            if p as usize > total_pages {
                anyhow::bail!(
                    "Halaman {} melebihi total halaman dokumen ({} halaman).",
                    p,
                    total_pages
                );
            }
            pages.push(p);
        }
    }

    if pages.is_empty() {
        anyhow::bail!("Tidak ada nomor halaman yang valid dalam input.");
    }

    // Keep original ordering requested by user or remove exact duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    let mut unique_pages = Vec::new();
    for p in pages {
        if seen.insert(p) {
            unique_pages.push(p);
        }
    }

    Ok(unique_pages)
}

/// Retrieves the total number of pages in a PDF file without modifying it.
pub fn get_pdf_page_count(pdf_path: &Path) -> Result<usize> {
    let doc = Document::load(pdf_path)
        .with_context(|| format!("Gagal membuka file PDF {:?}", pdf_path))?;
    Ok(doc.get_pages().len())
}

/// Extracts the specified 1-based page numbers from `input_path` and saves them to `output_path`.
pub fn extract_pages_to_file(
    input_path: &Path,
    pages_to_keep: &[u32],
    output_path: &Path,
) -> Result<usize> {
    if pages_to_keep.is_empty() {
        anyhow::bail!("Tidak ada halaman yang dipilih untuk diekstrak.");
    }

    let mut doc = Document::load(input_path)
        .with_context(|| format!("Gagal memuat dokumen {:?}", input_path))?;

    let pages_map: BTreeMap<u32, ObjectId> = doc.get_pages();
    let total_pages = pages_map.len();

    let mut target_object_ids: Vec<ObjectId> = Vec::new();
    for &page_num in pages_to_keep {
        if let Some(&obj_id) = pages_map.get(&page_num) {
            target_object_ids.push(obj_id);
        } else {
            anyhow::bail!(
                "Halaman {} tidak ditemukan dalam dokumen (total {} halaman).",
                page_num,
                total_pages
            );
        }
    }

    // Determine which pages are to be excluded
    let to_delete: Vec<u32> = (1..=(total_pages as u32))
        .filter(|p| !pages_to_keep.contains(p))
        .collect();

    // Delete unused pages
    doc.delete_pages(&to_delete);

    // Update Pages root dictionary with kept Kids and new count
    if let Ok(catalog) = doc.catalog() {
        if let Ok(pages_ref) = catalog.get(b"Pages").and_then(Object::as_reference) {
            if let Ok(pages_dict) = doc.get_dictionary_mut(pages_ref) {
                pages_dict.set("Count", target_object_ids.len() as u32);
                pages_dict.set(
                    "Kids",
                    target_object_ids
                        .iter()
                        .copied()
                        .map(Object::Reference)
                        .collect::<Vec<_>>(),
                );
            }
        }
    }

    // Clean up orphaned objects and renumber remaining objects
    doc.prune_objects();
    doc.renumber_objects();

    doc.save(output_path)
        .with_context(|| format!("Gagal menyimpan PDF hasil split ke {:?}", output_path))?;

    Ok(target_object_ids.len())
}
