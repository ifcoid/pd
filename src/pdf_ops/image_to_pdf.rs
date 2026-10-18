use anyhow::{Context, Result};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lopdf::{Dictionary, Document, Object, Stream};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Creates a PDF document from a collection of image files.
/// Each image becomes a page sized precisely to the image's dimensions.
pub fn images_to_pdf(image_paths: &[PathBuf], output_path: &Path) -> Result<usize> {
    if image_paths.is_empty() {
        anyhow::bail!("Tidak ada file gambar yang dipilih untuk dibuat PDF.");
    }

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut page_ids: Vec<Object> = Vec::new();

    for img_path in image_paths {
        let img_bytes = std::fs::read(img_path)
            .with_context(|| format!("Gagal membaca file gambar {:?}", img_path))?;

        // Detect or open image using the image crate
        let reader = image::ImageReader::new(std::io::Cursor::new(&img_bytes))
            .with_guessed_format()
            .with_context(|| format!("Format gambar tidak dikenali: {:?}", img_path))?;

        let format = reader.format();
        let dyn_img = reader
            .decode()
            .with_context(|| format!("Gagal men-decode gambar {:?}", img_path))?;

        let width = dyn_img.width();
        let height = dyn_img.height();
        if width == 0 || height == 0 {
            continue;
        }

        // Determine if we can embed raw JPEG directly or encode with FlateDecode
        let (img_stream_data, filter_name) = match format {
            Some(image::ImageFormat::Jpeg) => {
                // Direct JPEG embed without recompression loss
                (img_bytes, "DCTDecode")
            }
            _ => {
                // Convert to RGB8 and compress with Zlib (FlateDecode)
                let rgb_img = dyn_img.to_rgb8();
                let raw_rgb = rgb_img.into_raw();

                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder
                    .write_all(&raw_rgb)
                    .context("Gagal mengompres stream gambar RGB dengan Zlib")?;
                let compressed = encoder.finish().context("Gagal menyelesaikan kompresi Zlib")?;
                (compressed, "FlateDecode")
            }
        };

        // 1. Create Image XObject
        let mut img_dict = Dictionary::new();
        img_dict.set("Type", Object::Name(b"XObject".to_vec()));
        img_dict.set("Subtype", Object::Name(b"Image".to_vec()));
        img_dict.set("Width", Object::Integer(width as i64));
        img_dict.set("Height", Object::Integer(height as i64));
        img_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
        img_dict.set("BitsPerComponent", Object::Integer(8));
        img_dict.set("Filter", Object::Name(filter_name.as_bytes().to_vec()));

        let img_xobject = Stream::new(img_dict, img_stream_data);
        let img_xobject_id = doc.add_object(Object::Stream(img_xobject));

        // 2. Create Content Stream (scale image to fill the page)
        // In PDF coordinate system: q [width] 0 0 [height] 0 0 cm /Im0 Do Q
        let content_str = format!("q {} 0 0 {} 0 0 cm /Im0 Do Q\n", width, height);
        let content_stream = Stream::new(Dictionary::new(), content_str.into_bytes());
        let content_id = doc.add_object(Object::Stream(content_stream));

        // 3. Create Resources Dictionary
        let mut xobject_dict = Dictionary::new();
        xobject_dict.set("Im0", Object::Reference(img_xobject_id));

        let mut resources_dict = Dictionary::new();
        resources_dict.set("XObject", Object::Dictionary(xobject_dict));

        // 4. Create Page Object
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(width as f32),
                Object::Real(height as f32),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        page_dict.set("Resources", Object::Dictionary(resources_dict));

        let page_id = doc.add_object(Object::Dictionary(page_dict));
        page_ids.push(Object::Reference(page_id));
    }

    if page_ids.is_empty() {
        anyhow::bail!("Tidak ada gambar valid yang berhasil diproses.");
    }

    let total_pages = page_ids.len();

    // 5. Create Pages Catalog Object
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(page_ids));
    pages_dict.set("Count", Object::Integer(total_pages as i64));

    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // 6. Create Document Catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));

    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // 7. Save document
    doc.save(output_path)
        .with_context(|| format!("Gagal menyimpan PDF hasil konversi ke {:?}", output_path))?;

    Ok(total_pages)
}
