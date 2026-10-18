pub mod compressor;
pub mod merger;
pub mod splitter;
pub mod viewer;

pub use compressor::{compress_pdf, CompressionReport};
pub use merger::merge_pdfs;
pub use splitter::{extract_pages_to_file, get_pdf_page_count, parse_page_ranges};
pub use viewer::{render_page_to_rgba, PageViewData};

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Document, Object};
    use std::path::PathBuf;

    fn create_dummy_pdf(pages: usize) -> Document {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut page_ids = Vec::new();

        for _ in 0..pages {
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            page_ids.push(page_id);
        }

        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
            "Count" => pages as i64,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        doc
    }

    #[test]
    fn test_parse_page_ranges() {
        let res = parse_page_ranges("1-3, 5, 7", 10).unwrap();
        assert_eq!(res, vec![1, 2, 3, 5, 7]);

        let res2 = parse_page_ranges("2, 4-6", 6).unwrap();
        assert_eq!(res2, vec![2, 4, 5, 6]);

        let err = parse_page_ranges("5-2", 10);
        assert!(err.is_err());

        let out_of_bounds = parse_page_ranges("1-15", 10);
        assert!(out_of_bounds.is_err());
    }

    #[test]
    fn test_pdf_operations_end_to_end() {
        let temp_dir = std::env::temp_dir().join("pd_test_ops");
        let _ = std::fs::create_dir_all(&temp_dir);

        let pdf1_path = temp_dir.join("test_doc1.pdf");
        let pdf2_path = temp_dir.join("test_doc2.pdf");
        let merged_path = temp_dir.join("merged_output.pdf");
        let split_path = temp_dir.join("split_output.pdf");
        let compressed_path = temp_dir.join("compressed_output.pdf");

        // 1. Create two dummy PDFs (2 pages and 3 pages)
        let mut doc1 = create_dummy_pdf(2);
        doc1.save(&pdf1_path).unwrap();

        let mut doc2 = create_dummy_pdf(3);
        doc2.save(&pdf2_path).unwrap();

        assert_eq!(get_pdf_page_count(&pdf1_path).unwrap(), 2);
        assert_eq!(get_pdf_page_count(&pdf2_path).unwrap(), 3);

        // 2. Test Merge
        let files: Vec<PathBuf> = vec![pdf1_path.clone(), pdf2_path.clone()];
        let merged_pages = merge_pdfs(&files, &merged_path).unwrap();
        assert_eq!(merged_pages, 5);
        assert_eq!(get_pdf_page_count(&merged_path).unwrap(), 5);

        // 3. Test Split / Extract
        let pages_to_keep = vec![1, 3];
        let split_count = extract_pages_to_file(&merged_path, &pages_to_keep, &split_path).unwrap();
        assert_eq!(split_count, 2);
        assert_eq!(get_pdf_page_count(&split_path).unwrap(), 2);

        // 4. Test Compress
        let report = compress_pdf(&merged_path, &compressed_path).unwrap();
        assert!(compressed_path.exists());
        assert!(report.compressed_size_bytes > 0);

        // 5. Test Pure-Rust Ultra-Light Page Renderer
        let view_data = render_page_to_rgba(&merged_path, 1, 1.0).unwrap();
        assert_eq!(view_data.page_number, 1);
        assert_eq!(view_data.total_pages, 5);
        assert!(view_data.width > 0);
        assert!(view_data.height > 0);
        assert!(!view_data.rgba_bytes.is_empty());

        // Clean up temporary test files
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
