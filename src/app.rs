use std::path::PathBuf;
use iced::widget::{
    button, column, container, image, progress_bar, row, rule, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Padding, Task, Theme};

use crate::pdf_ops::{
    compress_pdf, export_pdf_page_to_image, extract_pages_to_file, get_pdf_page_count,
    images_to_pdf, merge_pdfs, render_page_to_rgba, CompressionReport, ExportImageFormat,
    PageViewData,
};

const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    View,
    ImagesToPdf,
    Merge,
    Split,
    Compress,
}

#[derive(Debug, Clone)]
pub enum OperationStatus {
    Idle,
    Processing(String),
    Success(String),
    Error(String),
}

pub struct App {
    pub active_tab: ActiveTab,
    pub status: OperationStatus,

    // Viewer Tab State (Ultra-Lightweight Pure-Rust Reader)
    pub viewer_file: Option<PathBuf>,
    pub viewer_current_page: usize,
    pub viewer_total_pages: usize,
    pub viewer_scale: f32,
    pub viewer_image_handle: Option<image::Handle>,

    // Images to PDF Tab State
    pub image_files: Vec<PathBuf>,

    // Merge Tab State
    pub merge_files: Vec<PathBuf>,

    // Split Tab State
    pub split_file: Option<PathBuf>,
    pub split_total_pages: usize,
    pub split_range_input: String,

    // Compress Tab State
    pub compress_file: Option<PathBuf>,
    pub compress_report: Option<CompressionReport>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(ActiveTab),

    // Viewer Actions
    PickViewerFile,
    ViewerFileSelected(Option<PathBuf>),
    ViewerNextPage,
    ViewerPrevPage,
    ViewerZoomIn,
    ViewerZoomOut,
    ViewerZoomReset,
    ViewerPageRendered(Result<PageViewData, String>),
    ViewerExportPagePng,
    ViewerExportPageJpg,
    ViewerExportCompleted(Result<PathBuf, String>),

    // Images to PDF Actions
    PickImageFiles,
    ImageFilesSelected(Option<Vec<PathBuf>>),
    RemoveImageFile(usize),
    MoveImageFileUp(usize),
    MoveImageFileDown(usize),
    ClearImageFiles,
    RunImagesToPdf,
    ImagesToPdfCompleted(Result<usize, String>),

    // Merge Actions
    PickMergeFiles,
    MergeFilesSelected(Option<Vec<PathBuf>>),
    RemoveMergeFile(usize),
    MoveMergeFileUp(usize),
    MoveMergeFileDown(usize),
    ClearMergeFiles,
    RunMerge,
    MergeCompleted(Result<usize, String>),

    // Split Actions
    PickSplitFile,
    SplitFileSelected(Option<PathBuf>),
    SplitRangeChanged(String),
    RunSplit,
    SplitCompleted(Result<usize, String>),

    // Compress Actions
    PickCompressFile,
    CompressFileSelected(Option<PathBuf>),
    RunCompress,
    CompressCompleted(Result<CompressionReport, String>),

    // Status
    DismissStatus,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            active_tab: ActiveTab::View,
            status: OperationStatus::Idle,
            viewer_file: None,
            viewer_current_page: 1,
            viewer_total_pages: 0,
            viewer_scale: 1.25,
            viewer_image_handle: None,
            image_files: Vec::new(),
            merge_files: Vec::new(),
            split_file: None,
            split_total_pages: 0,
            split_range_input: String::new(),
            compress_file: None,
            compress_report: None,
        }
    }

    pub fn boot() -> (Self, Task<Message>) {
        let mut app = Self::new();
        if let Some(arg) = std::env::args().nth(1) {
            let path = PathBuf::from(arg);
            if path.exists() && path.is_file() {
                if let Ok(total) = get_pdf_page_count(&path) {
                    app.viewer_file = Some(path.clone());
                    app.viewer_total_pages = total;
                    app.viewer_current_page = 1;
                    let scale = app.viewer_scale;
                    app.status = OperationStatus::Processing("Membuka dokumen PDF...".to_string());
                    let task = Task::future(async move {
                        match render_page_to_rgba(&path, 1, scale) {
                            Ok(data) => Message::ViewerPageRendered(Ok(data)),
                            Err(e) => Message::ViewerPageRendered(Err(e.to_string())),
                        }
                    });
                    return (app, task);
                }
            }
        }
        (app, Task::none())
    }

    pub fn theme(&self) -> Theme {
        Theme::TokyoNight
    }

    fn render_current_viewer_page(&mut self) -> Task<Message> {
        let file = match &self.viewer_file {
            Some(f) => f.clone(),
            None => return Task::none(),
        };

        let page = self.viewer_current_page;
        let scale = self.viewer_scale;
        self.status = OperationStatus::Processing(format!("Me-render halaman {}...", page));

        Task::future(async move {
            match render_page_to_rgba(&file, page, scale) {
                Ok(data) => Message::ViewerPageRendered(Ok(data)),
                Err(e) => Message::ViewerPageRendered(Err(e.to_string())),
            }
        })
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                self.status = OperationStatus::Idle;
                Task::none()
            }
            Message::DismissStatus => {
                self.status = OperationStatus::Idle;
                Task::none()
            }

            // --- VIEWER ACTIONS ---
            Message::PickViewerFile => {
                Task::future(async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_title("Buka file PDF untuk dibaca")
                        .pick_file()
                        .await;

                    Message::ViewerFileSelected(file.map(|h| h.path().to_path_buf()))
                })
            }
            Message::ViewerFileSelected(file_opt) => {
                if let Some(file) = file_opt {
                    self.viewer_file = Some(file);
                    self.viewer_current_page = 1;
                    return self.render_current_viewer_page();
                }
                Task::none()
            }
            Message::ViewerNextPage => {
                if self.viewer_current_page < self.viewer_total_pages {
                    self.viewer_current_page += 1;
                    return self.render_current_viewer_page();
                }
                Task::none()
            }
            Message::ViewerPrevPage => {
                if self.viewer_current_page > 1 {
                    self.viewer_current_page -= 1;
                    return self.render_current_viewer_page();
                }
                Task::none()
            }
            Message::ViewerZoomIn => {
                if self.viewer_scale < 3.0 {
                    self.viewer_scale = (self.viewer_scale + 0.25).min(3.0);
                    return self.render_current_viewer_page();
                }
                Task::none()
            }
            Message::ViewerZoomOut => {
                if self.viewer_scale > 0.5 {
                    self.viewer_scale = (self.viewer_scale - 0.25).max(0.5);
                    return self.render_current_viewer_page();
                }
                Task::none()
            }
            Message::ViewerZoomReset => {
                self.viewer_scale = 1.25;
                self.render_current_viewer_page()
            }
            Message::ViewerPageRendered(res) => {
                match res {
                    Ok(page_data) => {
                        self.viewer_current_page = page_data.page_number;
                        self.viewer_total_pages = page_data.total_pages;

                        let handle = image::Handle::from_rgba(
                            page_data.width,
                            page_data.height,
                            page_data.rgba_bytes,
                        );
                        self.viewer_image_handle = Some(handle);
                        self.status = OperationStatus::Idle;
                    }
                    Err(e) => {
                        self.status = OperationStatus::Error(format!("Gagal membaca PDF: {}", e));
                    }
                }
                Task::none()
            }
            Message::ViewerExportPagePng | Message::ViewerExportPageJpg => {
                let format = if matches!(message, Message::ViewerExportPagePng) {
                    ExportImageFormat::Png
                } else {
                    ExportImageFormat::Jpeg
                };
                let file = match &self.viewer_file {
                    Some(f) => f.clone(),
                    None => return Task::none(),
                };
                let page = self.viewer_current_page;
                let ext = format.extension();
                let default_name = format!(
                    "{}_page_{}.{}",
                    file.file_stem().and_then(|s| s.to_str()).unwrap_or("document"),
                    page,
                    ext
                );
                self.status = OperationStatus::Processing(format!(
                    "Mengekspor halaman {} ke {}...",
                    page,
                    ext.to_uppercase()
                ));

                Task::future(async move {
                    let filter_name = if format == ExportImageFormat::Png {
                        "PNG Image (*.png)"
                    } else {
                        "JPEG Image (*.jpg)"
                    };
                    let dest = rfd::AsyncFileDialog::new()
                        .add_filter(filter_name, &[ext])
                        .set_file_name(&default_name)
                        .set_title("Simpan Halaman Sebagai Gambar")
                        .save_file()
                        .await;

                    if let Some(handle) = dest {
                        let out_path = handle.path().to_path_buf();
                        match export_pdf_page_to_image(&file, page, &out_path, format, 2.0) {
                            Ok(path) => Message::ViewerExportCompleted(Ok(path)),
                            Err(e) => Message::ViewerExportCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::ViewerExportCompleted(Err("Ekspor gambar dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::ViewerExportCompleted(res) => {
                match res {
                    Ok(path) => {
                        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("image");
                        self.status = OperationStatus::Success(format!(
                            "Halaman berhasil diekspor ke: {}",
                            name
                        ));
                    }
                    Err(msg) => {
                        if msg.contains("dibatalkan") {
                            self.status = OperationStatus::Idle;
                        } else {
                            self.status = OperationStatus::Error(format!("Gagal mengekspor gambar: {}", msg));
                        }
                    }
                }
                Task::none()
            }

            // --- IMAGES TO PDF ACTIONS ---
            Message::PickImageFiles => {
                Task::future(async move {
                    let files = rfd::AsyncFileDialog::new()
                        .add_filter(
                            "Image files (*.png, *.jpg, *.jpeg, *.webp, *.bmp)",
                            &["png", "jpg", "jpeg", "webp", "bmp"],
                        )
                        .set_title("Pilih file gambar untuk dijadikan PDF")
                        .pick_files()
                        .await;

                    let paths = files.map(|handles| {
                        handles.into_iter().map(|h| h.path().to_path_buf()).collect()
                    });
                    Message::ImageFilesSelected(paths)
                })
            }
            Message::ImageFilesSelected(files_opt) => {
                if let Some(new_files) = files_opt {
                    for f in new_files {
                        if !self.image_files.contains(&f) {
                            self.image_files.push(f);
                        }
                    }
                    self.status = OperationStatus::Idle;
                }
                Task::none()
            }
            Message::RemoveImageFile(idx) => {
                if idx < self.image_files.len() {
                    self.image_files.remove(idx);
                }
                Task::none()
            }
            Message::MoveImageFileUp(idx) => {
                if idx > 0 && idx < self.image_files.len() {
                    self.image_files.swap(idx, idx - 1);
                }
                Task::none()
            }
            Message::MoveImageFileDown(idx) => {
                if idx + 1 < self.image_files.len() {
                    self.image_files.swap(idx, idx + 1);
                }
                Task::none()
            }
            Message::ClearImageFiles => {
                self.image_files.clear();
                self.status = OperationStatus::Idle;
                Task::none()
            }
            Message::RunImagesToPdf => {
                if self.image_files.is_empty() {
                    self.status = OperationStatus::Error(
                        "Pilih minimal satu file gambar untuk dibuat menjadi PDF.".to_string(),
                    );
                    return Task::none();
                }

                self.status =
                    OperationStatus::Processing("Mengonversi kumpulan gambar menjadi PDF...".to_string());
                let images = self.image_files.clone();

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_file_name("images.pdf")
                        .set_title("Simpan File PDF dari Gambar")
                        .save_file()
                        .await;

                    if let Some(handle) = dest {
                        let path = handle.path().to_path_buf();
                        match images_to_pdf(&images, &path) {
                            Ok(page_count) => Message::ImagesToPdfCompleted(Ok(page_count)),
                            Err(e) => Message::ImagesToPdfCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::ImagesToPdfCompleted(Err(
                            "Penyimpanan PDF dibatalkan oleh pengguna.".to_string(),
                        ))
                    }
                })
            }
            Message::ImagesToPdfCompleted(res) => {
                match res {
                    Ok(pages) => {
                        self.status = OperationStatus::Success(format!(
                            "Berhasil membuat PDF! {} halaman gambar berhasil digabungkan.",
                            pages
                        ));
                    }
                    Err(msg) => {
                        if msg.contains("dibatalkan") {
                            self.status = OperationStatus::Idle;
                        } else {
                            self.status = OperationStatus::Error(format!("Gagal: {}", msg));
                        }
                    }
                }
                Task::none()
            }

            // --- MERGE ACTIONS ---
            Message::PickMergeFiles => {
                Task::future(async move {
                    let files = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_title("Pilih file-file PDF untuk digabungkan")
                        .pick_files()
                        .await;

                    let paths = files.map(|handles| {
                        handles.into_iter().map(|h| h.path().to_path_buf()).collect()
                    });
                    Message::MergeFilesSelected(paths)
                })
            }
            Message::MergeFilesSelected(files_opt) => {
                if let Some(new_files) = files_opt {
                    for f in new_files {
                        if !self.merge_files.contains(&f) {
                            self.merge_files.push(f);
                        }
                    }
                    self.status = OperationStatus::Idle;
                }
                Task::none()
            }
            Message::RemoveMergeFile(idx) => {
                if idx < self.merge_files.len() {
                    self.merge_files.remove(idx);
                }
                Task::none()
            }
            Message::MoveMergeFileUp(idx) => {
                if idx > 0 && idx < self.merge_files.len() {
                    self.merge_files.swap(idx, idx - 1);
                }
                Task::none()
            }
            Message::MoveMergeFileDown(idx) => {
                if idx + 1 < self.merge_files.len() {
                    self.merge_files.swap(idx, idx + 1);
                }
                Task::none()
            }
            Message::ClearMergeFiles => {
                self.merge_files.clear();
                self.status = OperationStatus::Idle;
                Task::none()
            }
            Message::RunMerge => {
                if self.merge_files.is_empty() {
                    self.status = OperationStatus::Error(
                        "Pilih minimal satu file PDF untuk digabungkan.".to_string(),
                    );
                    return Task::none();
                }

                self.status = OperationStatus::Processing("Menggabungkan file PDF...".to_string());
                let files = self.merge_files.clone();

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_file_name("merged.pdf")
                        .set_title("Simpan File PDF Gabungan")
                        .save_file()
                        .await;

                    if let Some(handle) = dest {
                        let path = handle.path().to_path_buf();
                        match merge_pdfs(&files, &path) {
                            Ok(page_count) => Message::MergeCompleted(Ok(page_count)),
                            Err(e) => Message::MergeCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::MergeCompleted(Err("Penyimpanan dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::MergeCompleted(res) => {
                match res {
                    Ok(pages) => {
                        self.status = OperationStatus::Success(format!(
                            "Berhasil menggabungkan PDF! Total {} halaman berhasil disimpan.",
                            pages
                        ));
                    }
                    Err(msg) => {
                        if msg.contains("dibatalkan") {
                            self.status = OperationStatus::Idle;
                        } else {
                            self.status = OperationStatus::Error(format!("Gagal: {}", msg));
                        }
                    }
                }
                Task::none()
            }

            // --- SPLIT ACTIONS ---
            Message::PickSplitFile => {
                Task::future(async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_title("Pilih file PDF untuk dipecah / diekstrak")
                        .pick_file()
                        .await;

                    Message::SplitFileSelected(file.map(|h| h.path().to_path_buf()))
                })
            }
            Message::SplitFileSelected(file_opt) => {
                if let Some(file) = file_opt {
                    match get_pdf_page_count(&file) {
                        Ok(count) => {
                            self.split_total_pages = count;
                            self.split_file = Some(file);
                            self.split_range_input = format!("1-{}", count.min(1));
                            self.status = OperationStatus::Idle;
                        }
                        Err(e) => {
                            self.status = OperationStatus::Error(format!(
                                "Gagal membaca halaman PDF: {}",
                                e
                            ));
                        }
                    }
                }
                Task::none()
            }
            Message::SplitRangeChanged(val) => {
                self.split_range_input = val;
                Task::none()
            }
            Message::RunSplit => {
                let file = match &self.split_file {
                    Some(f) => f.clone(),
                    None => {
                        self.status = OperationStatus::Error(
                            "Pilih file PDF sumber terlebih dahulu.".to_string(),
                        );
                        return Task::none();
                    }
                };

                let range_str = self.split_range_input.clone();
                let total = self.split_total_pages;

                let pages_to_keep = match crate::pdf_ops::parse_page_ranges(&range_str, total) {
                    Ok(p) => p,
                    Err(e) => {
                        self.status = OperationStatus::Error(e.to_string());
                        return Task::none();
                    }
                };

                self.status = OperationStatus::Processing("Mengekstrak halaman PDF...".to_string());

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_file_name("split_extracted.pdf")
                        .set_title("Simpan Halaman PDF Hasil Pemisahan")
                        .save_file()
                        .await;

                    if let Some(handle) = dest {
                        let path = handle.path().to_path_buf();
                        match extract_pages_to_file(&file, &pages_to_keep, &path) {
                            Ok(cnt) => Message::SplitCompleted(Ok(cnt)),
                            Err(e) => Message::SplitCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::SplitCompleted(Err("Penyimpanan dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::SplitCompleted(res) => {
                match res {
                    Ok(cnt) => {
                        self.status = OperationStatus::Success(format!(
                            "Berhasil mengekstrak {} halaman ke file baru!",
                            cnt
                        ));
                    }
                    Err(msg) => {
                        if msg.contains("dibatalkan") {
                            self.status = OperationStatus::Idle;
                        } else {
                            self.status = OperationStatus::Error(format!("Gagal: {}", msg));
                        }
                    }
                }
                Task::none()
            }

            // --- COMPRESS ACTIONS ---
            Message::PickCompressFile => {
                Task::future(async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_title("Pilih file PDF untuk dikompresi")
                        .pick_file()
                        .await;

                    Message::CompressFileSelected(file.map(|h| h.path().to_path_buf()))
                })
            }
            Message::CompressFileSelected(file_opt) => {
                if let Some(file) = file_opt {
                    self.compress_file = Some(file);
                    self.compress_report = None;
                    self.status = OperationStatus::Idle;
                }
                Task::none()
            }
            Message::RunCompress => {
                let file = match &self.compress_file {
                    Some(f) => f.clone(),
                    None => {
                        self.status = OperationStatus::Error(
                            "Pilih file PDF yang akan dikompresi.".to_string(),
                        );
                        return Task::none();
                    }
                };

                self.status = OperationStatus::Processing("Mengompresi PDF...".to_string());

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_file_name("compressed.pdf")
                        .set_title("Simpan File PDF Terkompresi")
                        .save_file()
                        .await;

                    if let Some(handle) = dest {
                        let path = handle.path().to_path_buf();
                        match compress_pdf(&file, &path) {
                            Ok(report) => Message::CompressCompleted(Ok(report)),
                            Err(e) => Message::CompressCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::CompressCompleted(Err("Penyimpanan dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::CompressCompleted(res) => {
                match res {
                    Ok(report) => {
                        let pct_text = if report.bytes_saved > 0 {
                            format!("{:.1}% lebih kecil", report.percentage_saved)
                        } else {
                            "sudah teroptimasi".to_string()
                        };
                        self.status = OperationStatus::Success(format!(
                            "Kompresi selesai! Ukuran berkurang {} ({})",
                            format_file_size(report.bytes_saved.max(0) as u64),
                            pct_text
                        ));
                        self.compress_report = Some(report);
                    }
                    Err(msg) => {
                        if msg.contains("dibatalkan") {
                            self.status = OperationStatus::Idle;
                        } else {
                            self.status = OperationStatus::Error(format!("Gagal: {}", msg));
                        }
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = self.view_header();
        let tabs = self.view_tabs();
        let status_bar = self.view_status();

        let content: Element<'_, Message> = match self.active_tab {
            ActiveTab::View => self.view_viewer_tab(),
            ActiveTab::ImagesToPdf => self.view_images_to_pdf_tab(),
            ActiveTab::Merge => self.view_merge_tab(),
            ActiveTab::Split => self.view_split_tab(),
            ActiveTab::Compress => self.view_compress_tab(),
        };

        let main_layout = column![
            header,
            tabs,
            status_bar,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from(16))
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let app_badge = container(
            text("PURE RUST • ULTRA-LIGHT")
                .size(10)
                .style(text::primary)
        )
        .padding(Padding::from([3, 8]))
        .style(container::rounded_box);

        let title_col = column![
            row![
                text("pd").size(26).font(BOLD),
                text(" • Ultra-Lightweight PDF Reader & Utility").size(15).style(text::secondary),
                Space::new().width(10),
                app_badge
            ]
            .align_y(Alignment::Center),
            text("PDF reader teringan di muka bumi dan utilitas manipulasi dokumen 100% offline")
                .size(12)
                .style(text::secondary)
        ]
        .spacing(2);

        container(title_col)
            .width(Length::Fill)
            .padding(Padding::from([12, 20]))
            .into()
    }

    fn view_tabs(&self) -> Element<'_, Message> {
        let view_btn = button(
            row![
                text("👁️"),
                text("View PDF").size(13).font(BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::View {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::View))
        .padding(Padding::from([8, 16]));

        let img_to_pdf_btn = button(
            row![
                text("🖼️"),
                text("Gambar ke PDF").size(13).font(BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::ImagesToPdf {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::ImagesToPdf))
        .padding(Padding::from([8, 16]));

        let merge_btn = button(
            row![
                text("📑"),
                text("Merge PDF").size(13).font(BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::Merge {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::Merge))
        .padding(Padding::from([8, 16]));

        let split_btn = button(
            row![
                text("✂️"),
                text("Split PDF").size(13).font(BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::Split {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::Split))
        .padding(Padding::from([8, 16]));

        let compress_btn = button(
            row![
                text("🗜️"),
                text("Compress PDF").size(13).font(BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::Compress {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::Compress))
        .padding(Padding::from([8, 16]));

        let tabs_row = row![view_btn, img_to_pdf_btn, merge_btn, split_btn, compress_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        container(tabs_row)
            .width(Length::Fill)
            .padding(Padding::from([0, 20]))
            .into()
    }

    fn view_status(&self) -> Element<'_, Message> {
        match &self.status {
            OperationStatus::Idle => Space::new().height(0).into(),
            OperationStatus::Processing(msg) => {
                let bar = progress_bar(0.0..=100.0, 75.0);
                let content = column![
                    row![
                        text("⏳").size(15),
                        text(msg).size(13).font(BOLD)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    bar
                ]
                .spacing(6);

                container(content)
                    .width(Length::Fill)
                    .padding(Padding::from([8, 14]))
                    .style(container::rounded_box)
                    .into()
            }
            OperationStatus::Success(msg) => {
                let dismiss_btn = button(text("✕").size(11))
                    .style(button::text)
                    .on_press(Message::DismissStatus);

                let content = row![
                    text("✅").size(15),
                    text(msg).size(13).style(text::success),
                    Space::new().width(Length::Fill),
                    dismiss_btn
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                container(content)
                    .width(Length::Fill)
                    .padding(Padding::from([8, 14]))
                    .style(container::rounded_box)
                    .into()
            }
            OperationStatus::Error(msg) => {
                let dismiss_btn = button(text("✕").size(11))
                    .style(button::text)
                    .on_press(Message::DismissStatus);

                let content = row![
                    text("⚠️").size(15),
                    text(msg).size(13).style(text::danger),
                    Space::new().width(Length::Fill),
                    dismiss_btn
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                container(content)
                    .width(Length::Fill)
                    .padding(Padding::from([8, 14]))
                    .style(container::rounded_box)
                    .into()
            }
        }
    }

    // --- VIEWER TAB VIEW (PDF READER) ---
    fn view_viewer_tab(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let open_btn = button(
            row![
                text("📂"),
                text("Buka PDF").size(13)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding(Padding::from([7, 14]));

        let open_btn = if is_processing {
            open_btn
        } else {
            open_btn.on_press(Message::PickViewerFile)
        };

        let prev_btn = button(text("◀ Prev").size(12))
            .style(button::secondary)
            .padding(Padding::from([6, 12]));
        let prev_btn = if self.viewer_file.is_some() && self.viewer_current_page > 1 && !is_processing {
            prev_btn.on_press(Message::ViewerPrevPage)
        } else {
            prev_btn
        };

        let next_btn = button(text("Next ▶").size(12))
            .style(button::secondary)
            .padding(Padding::from([6, 12]));
        let next_btn = if self.viewer_file.is_some() && self.viewer_current_page < self.viewer_total_pages && !is_processing {
            next_btn.on_press(Message::ViewerNextPage)
        } else {
            next_btn
        };

        let zoom_in_btn = button(text("🔍 +").size(12))
            .style(button::secondary)
            .padding(Padding::from([6, 10]));
        let zoom_in_btn = if self.viewer_file.is_some() && !is_processing {
            zoom_in_btn.on_press(Message::ViewerZoomIn)
        } else {
            zoom_in_btn
        };

        let zoom_out_btn = button(text("🔍 -").size(12))
            .style(button::secondary)
            .padding(Padding::from([6, 10]));
        let zoom_out_btn = if self.viewer_file.is_some() && !is_processing {
            zoom_out_btn.on_press(Message::ViewerZoomOut)
        } else {
            zoom_out_btn
        };

        let zoom_reset_btn = button(text(format!("{:.0}%", self.viewer_scale * 100.0)).size(12))
            .style(button::secondary)
            .padding(Padding::from([6, 10]));
        let zoom_reset_btn = if self.viewer_file.is_some() && !is_processing {
            zoom_reset_btn.on_press(Message::ViewerZoomReset)
        } else {
            zoom_reset_btn
        };

        let export_png_btn = button(
            row![
                text("💾"),
                text("PNG").size(12).font(BOLD)
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .style(button::secondary)
        .padding(Padding::from([6, 10]));
        let export_png_btn = if self.viewer_file.is_some() && !is_processing {
            export_png_btn.on_press(Message::ViewerExportPagePng)
        } else {
            export_png_btn
        };

        let export_jpg_btn = button(
            row![
                text("💾"),
                text("JPG").size(12).font(BOLD)
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .style(button::secondary)
        .padding(Padding::from([6, 10]));
        let export_jpg_btn = if self.viewer_file.is_some() && !is_processing {
            export_jpg_btn.on_press(Message::ViewerExportPageJpg)
        } else {
            export_jpg_btn
        };

        let page_indicator = if self.viewer_total_pages > 0 {
            format!("Halaman {} / {}", self.viewer_current_page, self.viewer_total_pages)
        } else {
            "Tidak ada file".to_string()
        };

        let file_name_text = self.viewer_file.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Belum ada dokumen yang dibuka".to_string());

        let viewer_toolbar = row![
            open_btn,
            Space::new().width(10),
            text(file_name_text).size(13).font(BOLD),
            Space::new().width(Length::Fill),
            prev_btn,
            container(text(page_indicator).size(12).font(BOLD))
                .padding(Padding::from([4, 10]))
                .style(container::rounded_box),
            next_btn,
            Space::new().width(12),
            zoom_out_btn,
            zoom_reset_btn,
            zoom_in_btn,
            Space::new().width(12),
            export_png_btn,
            export_jpg_btn
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let canvas_view: Element<'_, Message> = if let Some(ref handle) = self.viewer_image_handle {
            let page_img = image(handle.clone());
            let paper_container = container(page_img)
                .padding(Padding::from(10))
                .style(container::rounded_box);

            scrollable(
                container(paper_container)
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .padding(Padding::from(16))
            )
            .height(Length::Fill)
            .into()
        } else {
            container(
                column![
                    text("📖").size(48),
                    text("PDF Reader Teringan di Muka Bumi").size(17).font(BOLD),
                    text("Buka file dokumen PDF apa saja untuk membaca secara instan.").size(13).style(text::secondary),
                    text("Ditenagai oleh rasterizer Pure-Rust (hayro) tanpa dependency C/C++ berat dan tanpa runtime Electron.")
                        .size(11)
                        .style(text::secondary),
                    Space::new().height(10),
                    button(
                        row![
                            text("📂"),
                            text("Pilih File PDF untuk Dibaca").size(14).font(BOLD)
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center)
                    )
                    .style(button::primary)
                    .padding(Padding::from([10, 20]))
                    .on_press(Message::PickViewerFile)
                ]
                .spacing(8)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(container::rounded_box)
            .into()
        };

        column![
            viewer_toolbar,
            rule::horizontal(1),
            canvas_view
        ]
        .spacing(12)
        .into()
    }

    // --- IMAGES TO PDF TAB VIEW ---
    fn view_images_to_pdf_tab(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let pick_btn = button(
            row![
                text("➕"),
                text("Tambah Gambar").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding(Padding::from([10, 16]));

        let pick_btn = if is_processing {
            pick_btn
        } else {
            pick_btn.on_press(Message::PickImageFiles)
        };

        let clear_btn = button(
            row![
                text("🗑️"),
                text("Kosongkan Daftar").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::danger)
        .padding(Padding::from([10, 16]));

        let clear_btn = if !self.image_files.is_empty() && !is_processing {
            clear_btn.on_press(Message::ClearImageFiles)
        } else {
            clear_btn
        };

        let convert_action_btn = button(
            row![
                text("⚡"),
                text(format!(
                    "Buat PDF ({} gambar)",
                    self.image_files.len()
                ))
                .size(15)
                .font(BOLD)
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        )
        .style(button::success)
        .padding(Padding::from([12, 24]));

        let convert_action_btn = if !self.image_files.is_empty() && !is_processing {
            convert_action_btn.on_press(Message::RunImagesToPdf)
        } else {
            convert_action_btn
        };

        let top_toolbar = row![
            pick_btn,
            clear_btn,
            Space::new().width(Length::Fill),
            convert_action_btn
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let images_list: Element<'_, Message> = if self.image_files.is_empty() {
            container(
                column![
                    text("🖼️").size(48),
                    text("Belum ada file gambar yang dipilih").size(16).font(BOLD),
                    text("Klik tombol 'Tambah Gambar' di atas untuk memilih file PNG, JPG, JPEG, WEBP, atau BMP.")
                        .size(13)
                        .style(text::secondary),
                    text("Setiap gambar akan dikonversi menjadi halaman PDF dengan resolusi dan kualitas asli.")
                        .size(12)
                        .style(text::secondary)
                ]
                .spacing(10)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(container::rounded_box)
            .into()
        } else {
            let mut list_col = column![].spacing(8);

            for (idx, path) in self.image_files.iter().enumerate() {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Image".to_string());

                let file_size_text = std::fs::metadata(path)
                    .map(|m| format_file_size(m.len()))
                    .unwrap_or_else(|_| "-".to_string());

                let up_btn = button(text("▲").size(12))
                    .style(button::secondary)
                    .padding(Padding::from([4, 8]));
                let up_btn = if idx > 0 && !is_processing {
                    up_btn.on_press(Message::MoveImageFileUp(idx))
                } else {
                    up_btn
                };

                let down_btn = button(text("▼").size(12))
                    .style(button::secondary)
                    .padding(Padding::from([4, 8]));
                let down_btn = if idx + 1 < self.image_files.len() && !is_processing {
                    down_btn.on_press(Message::MoveImageFileDown(idx))
                } else {
                    down_btn
                };

                let remove_btn = button(text("✕").size(12))
                    .style(button::danger)
                    .padding(Padding::from([4, 8]));
                let remove_btn = if !is_processing {
                    remove_btn.on_press(Message::RemoveImageFile(idx))
                } else {
                    remove_btn
                };

                let row_item = container(
                    row![
                        container(text(format!("#{}", idx + 1)).size(13).font(BOLD))
                            .padding(Padding::from([3, 8]))
                            .style(container::rounded_box),
                        column![
                            text(file_name).size(14).font(BOLD),
                            text(file_size_text).size(11).style(text::secondary)
                        ]
                        .spacing(2),
                        Space::new().width(Length::Fill),
                        up_btn,
                        down_btn,
                        remove_btn
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                )
                .width(Length::Fill)
                .padding(Padding::from([10, 16]))
                .style(container::rounded_box);

                list_col = list_col.push(row_item);
            }

            scrollable(list_col)
                .height(Length::Fill)
                .into()
        };

        column![
            top_toolbar,
            rule::horizontal(1),
            images_list
        ]
        .spacing(12)
        .into()
    }

    fn view_merge_tab(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let pick_btn = button(
            row![
                text("➕"),
                text("Tambah File PDF").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding(Padding::from([10, 16]));

        let pick_btn = if is_processing {
            pick_btn
        } else {
            pick_btn.on_press(Message::PickMergeFiles)
        };

        let clear_btn = button(
            row![
                text("🗑️"),
                text("Kosongkan Daftar").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::danger)
        .padding(Padding::from([10, 16]));

        let clear_btn = if !self.merge_files.is_empty() && !is_processing {
            clear_btn.on_press(Message::ClearMergeFiles)
        } else {
            clear_btn
        };

        let merge_action_btn = button(
            row![
                text("⚡"),
                text(format!(
                    "Gabungkan Sekarang ({} file)",
                    self.merge_files.len()
                ))
                .size(15)
                .font(BOLD)
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        )
        .style(button::success)
        .padding(Padding::from([12, 24]));

        let merge_action_btn = if self.merge_files.len() >= 2 && !is_processing {
            merge_action_btn.on_press(Message::RunMerge)
        } else {
            merge_action_btn
        };

        let top_toolbar = row![
            pick_btn,
            clear_btn,
            Space::new().width(Length::Fill),
            merge_action_btn
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let files_list: Element<'_, Message> = if self.merge_files.is_empty() {
            container(
                column![
                    text("📄").size(48),
                    text("Belum ada file PDF yang dipilih").size(16).font(BOLD),
                    text("Klik tombol 'Tambah File PDF' di atas untuk memilih file yang ingin digabungkan.")
                        .size(13)
                        .style(text::secondary)
                ]
                .spacing(10)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(container::rounded_box)
            .into()
        } else {
            let mut list_col = column![].spacing(8);

            for (idx, path) in self.merge_files.iter().enumerate() {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "File".to_string());

                let file_size_text = std::fs::metadata(path)
                    .map(|m| format_file_size(m.len()))
                    .unwrap_or_else(|_| "-".to_string());

                let up_btn = button(text("▲").size(12))
                    .style(button::secondary)
                    .padding(Padding::from([4, 8]));
                let up_btn = if idx > 0 && !is_processing {
                    up_btn.on_press(Message::MoveMergeFileUp(idx))
                } else {
                    up_btn
                };

                let down_btn = button(text("▼").size(12))
                    .style(button::secondary)
                    .padding(Padding::from([4, 8]));
                let down_btn = if idx + 1 < self.merge_files.len() && !is_processing {
                    down_btn.on_press(Message::MoveMergeFileDown(idx))
                } else {
                    down_btn
                };

                let del_btn = button(text("✕").size(12))
                    .style(button::danger)
                    .padding(Padding::from([4, 8]));
                let del_btn = if !is_processing {
                    del_btn.on_press(Message::RemoveMergeFile(idx))
                } else {
                    del_btn
                };

                let item_card = container(
                    row![
                        container(text(format!("#{}", idx + 1)).font(BOLD).size(13))
                            .padding(Padding::from([4, 8]))
                            .style(container::rounded_box),
                        column![
                            text(file_name).size(14).font(BOLD),
                            text(format!("{} • {}", file_size_text, path.display()))
                                .size(11)
                                .style(text::secondary)
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        row![up_btn, down_btn, del_btn].spacing(6)
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                )
                .width(Length::Fill)
                .padding(Padding::from([8, 14]))
                .style(container::rounded_box);

                list_col = list_col.push(item_card);
            }

            scrollable(list_col).height(Length::Fill).into()
        };

        column![
            top_toolbar,
            rule::horizontal(1),
            files_list
        ]
        .spacing(14)
        .into()
    }

    fn view_split_tab(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let pick_btn = button(
            row![
                text("📂"),
                text("Pilih File PDF Sumber").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding(Padding::from([10, 18]));

        let pick_btn = if is_processing {
            pick_btn
        } else {
            pick_btn.on_press(Message::PickSplitFile)
        };

        let file_info_view: Element<'_, Message> = if let Some(ref path) = self.split_file {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "PDF".to_string());

            let size_text = std::fs::metadata(path)
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|_| "-".to_string());

            container(
                row![
                    text("📄").size(32),
                    column![
                        text(file_name).size(15).font(BOLD),
                        text(format!("Ukuran: {} • Path: {}", size_text, path.display()))
                            .size(11)
                            .style(text::secondary),
                        row![
                            text(format!("Total Halaman: {}", self.split_total_pages))
                                .font(BOLD)
                                .size(13)
                                .style(text::primary)
                        ]
                        .spacing(8)
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    pick_btn
                ]
                .spacing(16)
                .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(Padding::from([14, 18]))
            .style(container::rounded_box)
            .into()
        } else {
            container(
                column![
                    text("✂️").size(42),
                    text("Belum ada file PDF yang dipilih untuk dipecah").size(15).font(BOLD),
                    text("Pilih dokumen PDF yang ingin dipisahkan atau diekstrak halamannya.").size(12).style(text::secondary),
                    Space::new().height(8),
                    pick_btn
                ]
                .spacing(8)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(Padding::from([28, 20]))
            .center_x(Length::Fill)
            .style(container::rounded_box)
            .into()
        };

        // Range input configuration card
        let range_input = text_input(
            "Contoh: 1-3, 5, 8-10",
            &self.split_range_input,
        )
        .on_input(Message::SplitRangeChanged)
        .padding(10)
        .size(14);

        let split_action_btn = button(
            row![
                text("⚡"),
                text("Ekstrak & Simpan Dokumen").size(15).font(BOLD)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::success)
        .padding(Padding::from([12, 24]));

        let split_action_btn = if self.split_file.is_some() && !self.split_range_input.is_empty() && !is_processing {
            split_action_btn.on_press(Message::RunSplit)
        } else {
            split_action_btn
        };

        let config_card = container(
            column![
                text("Konfigurasi Pemisahan Halaman").size(16).font(BOLD),
                text("Tentukan halaman yang ingin diambil dari dokumen. Anda dapat menggunakan tanda hubung (-) untuk rentang dan koma (,) untuk halaman terpisah.")
                    .size(12)
                    .style(text::secondary),
                Space::new().height(6),
                row![
                    text("Rentang Halaman:").size(14).font(BOLD),
                    range_input
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                text("Format yang didukung: '1-5' (halaman 1 s/d 5), '1, 3, 5' (halaman tertentu), atau kombinasi '1-3, 5, 7-9'")
                    .size(11)
                    .style(text::secondary),
                Space::new().height(8),
                split_action_btn
            ]
            .spacing(10)
        )
        .width(Length::Fill)
        .padding(Padding::from([16, 20]))
        .style(container::rounded_box);

        column![
            file_info_view,
            config_card
        ]
        .spacing(16)
        .into()
    }

    fn view_compress_tab(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let pick_btn = button(
            row![
                text("📂"),
                text("Pilih File PDF").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding(Padding::from([10, 18]));

        let pick_btn = if is_processing {
            pick_btn
        } else {
            pick_btn.on_press(Message::PickCompressFile)
        };

        let file_view: Element<'_, Message> = if let Some(ref path) = self.compress_file {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "PDF".to_string());

            let size_text = std::fs::metadata(path)
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|_| "-".to_string());

            container(
                row![
                    text("📄").size(32),
                    column![
                        text(file_name).size(15).font(BOLD),
                        text(format!("Ukuran Saat Ini: {}", size_text))
                            .size(13)
                            .style(text::primary),
                        text(path.display().to_string()).size(11).style(text::secondary)
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    pick_btn
                ]
                .spacing(16)
                .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(Padding::from([14, 18]))
            .style(container::rounded_box)
            .into()
        } else {
            container(
                column![
                    text("🗜️").size(42),
                    text("Belum ada file PDF yang dipilih untuk dikompresi").size(15).font(BOLD),
                    text("Pilih dokumen PDF yang ingin dioptimasi dan dikurangi ukurannya.").size(12).style(text::secondary),
                    Space::new().height(8),
                    pick_btn
                ]
                .spacing(8)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(Padding::from([28, 20]))
            .center_x(Length::Fill)
            .style(container::rounded_box)
            .into()
        };

        let compress_action_btn = button(
            row![
                text("⚡"),
                text("Kompres PDF Sekarang").size(15).font(BOLD)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::success)
        .padding(Padding::from([12, 24]));

        let compress_action_btn = if self.compress_file.is_some() && !is_processing {
            compress_action_btn.on_press(Message::RunCompress)
        } else {
            compress_action_btn
        };

        let report_view: Element<'_, Message> = if let Some(ref report) = self.compress_report {
            let orig = format_file_size(report.original_size_bytes);
            let comp = format_file_size(report.compressed_size_bytes);
            let saved = format_file_size(report.bytes_saved.max(0) as u64);
            let pct = format!("{:.1}%", report.percentage_saved.max(0.0));

            container(
                column![
                    row![
                        text("📊 Hasil Optimasi PDF").size(16).font(BOLD),
                        Space::new().width(Length::Fill),
                        container(text(format!("Hemat {}", pct)).font(BOLD).size(13))
                            .padding(Padding::from([4, 10]))
                            .style(container::rounded_box)
                    ]
                    .align_y(Alignment::Center),
                    rule::horizontal(1),
                    row![
                        column![
                            text("Ukuran Asli").size(12).style(text::secondary),
                            text(orig).size(16).font(BOLD)
                        ],
                        Space::new().width(24),
                        column![
                            text("Ukuran Baru").size(12).style(text::secondary),
                            text(comp).size(16).font(BOLD).style(text::success)
                        ],
                        Space::new().width(24),
                        column![
                            text("Ruang Dihemat").size(12).style(text::secondary),
                            text(saved).size(16).font(BOLD).style(text::primary)
                        ]
                    ]
                ]
                .spacing(12)
            )
            .width(Length::Fill)
            .padding(Padding::from([16, 20]))
            .style(container::rounded_box)
            .into()
        } else {
            Space::new().height(0).into()
        };

        let optimize_card = container(
            column![
                text("Metode Optimasi").size(16).font(BOLD),
                text("Aplikasi menggunakan kompresi stream Flate internal dan pengemasan modern PDF 1.5 Object Streams (/ObjStm) serta pemangkasan objek yang tidak lagi direferensikan.")
                    .size(12)
                    .style(text::secondary),
                Space::new().height(6),
                compress_action_btn
            ]
            .spacing(10)
        )
        .width(Length::Fill)
        .padding(Padding::from([16, 20]))
        .style(container::rounded_box);

        column![
            file_view,
            report_view,
            optimize_card
        ]
        .spacing(16)
        .into()
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
