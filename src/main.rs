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

    iced::application(App::boot, App::update, App::view)
        .title("pd")
        .theme(App::theme)
        .window_size([880.0, 700.0])
        .centered()
        .run()
}

