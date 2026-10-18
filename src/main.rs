#![windows_subsystem = "windows"]

mod app;
mod pdf_ops;

use app::App;
use std::path::{Path, PathBuf};

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--to-pdf" {
        let image_paths: Vec<PathBuf> = args[2..]
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists() && p.is_file())
            .collect();

        if !image_paths.is_empty() {
            let parent_dir = image_paths[0].parent().unwrap_or_else(|| Path::new("."));
            let stem = image_paths[0]
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let dest_name = if image_paths.len() == 1 {
                format!("{}.pdf", stem)
            } else {
                format!("{}_bundle.pdf", stem)
            };
            let dest_path = parent_dir.join(dest_name);

            if let Err(e) = pdf_ops::images_to_pdf(&image_paths, &dest_path) {
                eprintln!("Error converting to PDF: {}", e);
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }

    if args.len() > 1 && (args[1] == "--to-image" || args[1] == "--to-png" || args[1] == "--to-jpg") {
        let is_jpg = args[1] == "--to-jpg";
        let format = if is_jpg {
            pdf_ops::ExportImageFormat::Jpeg
        } else {
            pdf_ops::ExportImageFormat::Png
        };
        let ext = format.extension();

        let pdf_paths: Vec<PathBuf> = args[2..]
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists() && p.is_file())
            .collect();

        for pdf_path in pdf_paths {
            if let Ok(total_pages) = pdf_ops::get_pdf_page_count(&pdf_path) {
                let parent_dir = pdf_path.parent().unwrap_or_else(|| Path::new("."));
                let stem = pdf_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("document");

                if total_pages <= 1 {
                    let dest_path = parent_dir.join(format!("{}.{}", stem, ext));
                    let _ = pdf_ops::export_pdf_page_to_image(&pdf_path, 1, &dest_path, format, 2.0);
                } else {
                    let out_dir = parent_dir.join(format!("{}_images", stem));
                    let _ = std::fs::create_dir_all(&out_dir);
                    let _ = pdf_ops::export_all_pdf_pages_to_images(&pdf_path, &out_dir, format, 2.0);
                }
            }
        }
        std::process::exit(0);
    }

    if args.len() > 1 && args[1] == "--compress" {
        let pdf_paths: Vec<PathBuf> = args[2..]
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists() && p.is_file())
            .collect();

        for pdf_path in pdf_paths {
            let parent_dir = pdf_path.parent().unwrap_or_else(|| Path::new("."));
            let stem = pdf_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");

            let dest_path = parent_dir.join(format!("{}_compressed.pdf", stem));
            let _ = pdf_ops::compress_pdf(&pdf_path, &dest_path);
        }
        std::process::exit(0);
    }

    if args.len() > 1 && args[1] == "--merge" {
        let pdf_paths: Vec<PathBuf> = args[2..]
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists() && p.is_file())
            .collect();

        if pdf_paths.len() >= 2 {
            let parent_dir = pdf_paths[0].parent().unwrap_or_else(|| Path::new("."));
            let stem = pdf_paths[0]
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");

            let dest_path = parent_dir.join(format!("{}_merged.pdf", stem));
            if let Err(e) = pdf_ops::merge_pdfs(&pdf_paths, &dest_path) {
                eprintln!("Error merging PDFs: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    let icon = {
        let png_bytes = include_bytes!("../assets/pd.png");
        if let Ok(img) = image::load_from_memory(png_bytes) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            iced::window::icon::from_rgba(rgba.into_raw(), width, height).ok()
        } else {
            None
        }
    };

    let mut window = iced::window::Settings {
        size: [880.0, 700.0].into(),
        position: iced::window::Position::Centered,
        ..Default::default()
    };
    if let Some(ic) = icon {
        window.icon = Some(ic);
    }

    iced::application(App::boot, App::update, App::view)
        .title("pd")
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window)
        .run()
}

