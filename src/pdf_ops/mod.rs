pub mod compressor;
pub mod image_export;
pub mod image_to_pdf;
pub mod merger;
pub mod splitter;
pub mod viewer;

pub use compressor::{compress_pdf, CompressionReport};
pub use image_export::{
    export_all_pdf_pages_to_images, export_pdf_page_to_image, ExportImageFormat,
};
pub use image_to_pdf::images_to_pdf;
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

        // 6. Test Images to PDF
        let sample_png = temp_dir.join("sample_img.png");
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, 128])
        });
        img.save(&sample_png).unwrap();

        let img_pdf_path = temp_dir.join("from_images.pdf");
        let pages_created = images_to_pdf(&[sample_png.clone()], &img_pdf_path).unwrap();
        assert_eq!(pages_created, 1);
        assert!(img_pdf_path.exists());
        assert_eq!(get_pdf_page_count(&img_pdf_path).unwrap(), 1);

        // 7. Test Export PDF page to PNG & JPG
        let exported_png = temp_dir.join("exported_page.png");
        let exported_jpg = temp_dir.join("exported_page.jpg");
        export_pdf_page_to_image(&merged_path, 1, &exported_png, ExportImageFormat::Png, 1.0).unwrap();
        assert!(exported_png.exists());
        assert!(std::fs::metadata(&exported_png).unwrap().len() > 0);

        export_pdf_page_to_image(&merged_path, 1, &exported_jpg, ExportImageFormat::Jpeg, 1.0).unwrap();
        assert!(exported_jpg.exists());
        assert!(std::fs::metadata(&exported_jpg).unwrap().len() > 0);

        // 8. Test Export All Pages
        let all_dir = temp_dir.join("all_pages");
        let _ = std::fs::create_dir_all(&all_dir);
        let all_exported = export_all_pdf_pages_to_images(&merged_path, &all_dir, ExportImageFormat::Png, 1.0).unwrap();
        assert_eq!(all_exported.len(), 5);

        // Clean up temporary test files
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
