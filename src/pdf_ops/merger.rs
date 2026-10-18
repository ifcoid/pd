use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Merges multiple PDF files in the specified order into a single output PDF file.
pub fn merge_pdfs(input_files: &[PathBuf], output_path: &Path) -> Result<usize> {
    if input_files.is_empty() {
        anyhow::bail!("Tidak ada file PDF yang dipilih untuk digabungkan.");
    }

    if input_files.len() == 1 {
        let mut doc = Document::load(&input_files[0])
            .with_context(|| format!("Gagal memuat file {:?}", input_files[0]))?;
        let page_count = doc.get_pages().len();
        doc.save(output_path)
            .with_context(|| format!("Gagal menyimpan PDF gabungan ke {:?}", output_path))?;
        return Ok(page_count);
    }

    let mut max_id = 1u32;
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();
    let mut merged_document = Document::with_version("1.5");

    let mut total_page_count = 0;

    for file_path in input_files {
        let mut doc = Document::load(file_path)
            .with_context(|| format!("Gagal membaca file PDF {:?}", file_path))?;

        // Renumber objects in this document so IDs don't collide
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        total_page_count += pages.len();

        for (_page_num, object_id) in pages {
            if let Ok(page_obj) = doc.get_object(object_id) {
                documents_pages.insert(object_id, page_obj.clone());
            }
        }

        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Separate Pages, Catalog, and other objects
    for (object_id, object) in documents_objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        object_id
                    },
                    object,
                ));
            }
            b"Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref old_pages_obj)) = pages_object {
                        if let Ok(old_dict) = old_pages_obj.as_dict() {
                            dictionary.extend(old_dict);
                        }
                    }

                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            b"Page" => {
                // Page objects will be inserted with updated Parent reference
            }
            b"Outlines" | b"Outline" => {
                // Skip outlines to avoid cross-document broken links
            }
            _ => {
                merged_document.objects.insert(object_id, object);
            }
        }
    }

    let (pages_id, page_object) = pages_object
        .ok_or_else(|| anyhow::anyhow!("Struktur 'Pages' root tidak ditemukan dalam dokumen."))?;

    let (catalog_id, catalog_object) = catalog_object
        .ok_or_else(|| anyhow::anyhow!("Struktur 'Catalog' root tidak ditemukan dalam dokumen."))?;

    // Update parent reference for each Page object and insert into document
    for (object_id, page_val) in documents_pages.iter() {
        if let Ok(mut page_dict) = page_val.as_dict().cloned() {
            page_dict.set("Parent", pages_id);
            merged_document.objects.insert(*object_id, Object::Dictionary(page_dict));
        }
    }

    // Update Pages root dictionary with total count and new Kids list
    if let Ok(mut pages_dict) = page_object.as_dict().cloned() {
        pages_dict.set("Count", documents_pages.len() as u32);
        pages_dict.set(
            "Kids",
            documents_pages.keys().copied().map(Object::Reference).collect::<Vec<_>>(),
        );
        merged_document.objects.insert(pages_id, Object::Dictionary(pages_dict));
    }

    // Update Catalog root dictionary
    if let Ok(mut cat_dict) = catalog_object.as_dict().cloned() {
        cat_dict.set("Pages", pages_id);
        cat_dict.remove(b"Outlines");
        merged_document.objects.insert(catalog_id, Object::Dictionary(cat_dict));
    }

    merged_document.trailer.set("Root", catalog_id);
    merged_document.max_id = merged_document.objects.len() as u32;
    merged_document.renumber_objects();

    merged_document.save(output_path)
        .with_context(|| format!("Gagal menyimpan file hasil merge ke {:?}", output_path))?;

    Ok(total_page_count)
}
