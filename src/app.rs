use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use iced::event::{self, Event};
use iced::widget::{
    button, column, container, image, mouse_area, progress_bar, row, rule, scrollable, text,
    text_editor, text_input, Space,
};
use iced::{
    border::Border, keyboard, mouse, Alignment, Background, Color, Element, Font, Length, Padding,
    Shadow, Subscription, Task, Theme, Vector,
};

use crate::pdf_ops::{
    compress_pdf, export_all_pdf_pages_to_images, export_pdf_page_to_image,
    export_pdf_pages_to_dir, extract_page_text, extract_pages_to_file, get_pdf_page_count,
    images_to_pdf, merge_pdfs, parse_page_ranges, render_page_to_rgba,
    CompressionReport, ExportImageFormat, PageViewData,
};


const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    View,
    PdfToImages,
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

    // Viewer Tab State (Continuous Vertical Scroll Pure-Rust Reader)
    pub viewer_file: Option<PathBuf>,
    pub viewer_current_page: usize,
    pub viewer_total_pages: usize,
    pub viewer_scale: f32,
    pub viewer_rotation: usize,
    pub viewer_page_input: String,
    pub viewer_image_handle: Option<image::Handle>,
    pub viewer_current_data: Option<PageViewData>,
    pub viewer_pages: HashMap<usize, (image::Handle, PageViewData)>,
    pub viewer_page_dims: (u32, u32),
    pub viewer_loading_pages: HashSet<usize>,
    pub viewer_scroll_relative_y: f32,
    pub viewer_content_taller: bool,
    pub sidebar_open: bool,
    pub context_menu_open: bool,

    // Text Drawer (Option 1: Blok & Salin Teks)
    pub text_drawer_open: bool,
    pub text_drawer_content: text_editor::Content,
    pub text_drawer_page: usize,

    // Snip Tool (Option 2: Cuplik Area Gambar/Teks)
    pub snip_mode: bool,
    pub snip_start: Option<(f32, f32)>,
    pub snip_current: Option<(f32, f32)>,
    pub snip_rect: Option<(f32, f32, f32, f32)>,
    pub cursor_pos: (f32, f32),

    // PDF to Images Tab State
    pub pdf_to_img_file: Option<PathBuf>,
    pub pdf_to_img_total_pages: usize,
    pub pdf_to_img_range: String,
    pub pdf_to_img_all_pages: bool,
    pub pdf_to_img_format: ExportImageFormat,
    pub pdf_to_img_scale: f32,

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

    // Header / Hamburger Menu
    pub menu_open: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(ActiveTab),
    ToggleMenu,
    CloseMenu,
    ToggleSidebar,

    // Viewer Actions
    PickViewerFile,
    ViewerFileSelected(Option<PathBuf>),
    ViewerNextPage,
    ViewerPrevPage,
    ViewerGoToPage(usize),
    ViewerPageInputChanged(String),
    ViewerPageInputSubmitted,
    ViewerZoomIn,
    ViewerZoomOut,
    ViewerZoomReset,
    ViewerFitWidth,
    ViewerRotateClockwise,
    ViewerPrint,
    ViewerPageRendered(Result<PageViewData, String>),
    ViewerExportPagePng,
    ViewerExportPageJpg,
    ViewerExportAll,
    ViewerExportCompleted(Result<PathBuf, String>),
    ViewerExportAllCompleted(Result<usize, String>),
    ViewerCopyPageToClipboard,
    ViewerExportPagePdf,
    ViewerExportPdfCompleted(Result<PathBuf, String>),
    ViewerScrolled {
        relative_y: f32,
        content_taller: bool,
    },
    CloseContextMenu,
    EventOccurred(Event),

    // Text Drawer (Opsi 1: Blok & Salin Teks)
    ToggleTextDrawer,
    CloseTextDrawer,
    PageTextEditorAction(text_editor::Action),
    CopyPageText,

    // Snip Tool (Opsi 2: Cuplik Area)
    ToggleSnipMode,
    CancelSnip,
    SnipPresetFull,
    SnipPresetTop,
    SnipPresetBottom,
    SnipPresetCenter,
    CopySnipArea,
    SaveSnipArea,
    SnipAreaSaved(Result<PathBuf, String>),

    // PDF to Images Actions
    PickPdfToImgFile,
    PdfToImgFileSelected(Option<PathBuf>),
    PdfToImgRangeChanged(String),
    PdfToImgFormatChanged(ExportImageFormat),
    PdfToImgScaleChanged(f32),
    PdfToImgToggleAllPages(bool),
    RunPdfToImages,
    PdfToImagesCompleted(Result<usize, String>),

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
            viewer_rotation: 0,
            viewer_page_input: "1".to_string(),
            viewer_image_handle: None,
            viewer_pages: HashMap::new(),
            viewer_page_dims: (0, 0),
            viewer_loading_pages: HashSet::new(),
            viewer_scroll_relative_y: 0.0,
            viewer_content_taller: false,
            pdf_to_img_file: None,
            pdf_to_img_total_pages: 0,
            pdf_to_img_range: String::new(),
            pdf_to_img_all_pages: true,
            pdf_to_img_format: ExportImageFormat::Png,
            pdf_to_img_scale: 2.0,
            image_files: Vec::new(),
            merge_files: Vec::new(),
            split_file: None,
            split_total_pages: 0,
            split_range_input: String::new(),
            compress_file: None,
            compress_report: None,
            menu_open: false,
            sidebar_open: false,
            viewer_current_data: None,
            context_menu_open: false,
            text_drawer_open: false,
            text_drawer_content: text_editor::Content::new(),
            text_drawer_page: 1,
            snip_mode: false,
            snip_start: None,
            snip_current: None,
            snip_rect: None,
            cursor_pos: (0.0, 0.0),
        }
    }

    pub fn boot() -> (Self, Task<Message>) {
        let mut app = Self::new();
        let args: Vec<String> = std::env::args().skip(1).collect();

        if !args.is_empty() {
            if args[0] == "--gui-pdf-to-images" && args.len() > 1 {
                let path = PathBuf::from(&args[1]);
                if path.exists() && path.is_file() {
                    if let Ok(count) = get_pdf_page_count(&path) {
                        app.pdf_to_img_file = Some(path);
                        app.pdf_to_img_total_pages = count;
                        app.pdf_to_img_range = format!("1-{}", count);
                        app.active_tab = ActiveTab::PdfToImages;
                        app.status = OperationStatus::Idle;
                        return (app, Task::none());
                    }
                }
            }

            if args[0] == "--gui-compress" && args.len() > 1 {
                let path = PathBuf::from(&args[1]);
                if path.exists() && path.is_file() {
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ext == "pdf" {
                        app.compress_file = Some(path);
                        app.active_tab = ActiveTab::Compress;
                        app.status = OperationStatus::Idle;
                        return (app, Task::none());
                    }
                }
            }

            if (args[0] == "--gui-merge" || args[0] == "--merge") && args.len() > 1 {
                for arg in args.iter().skip(1) {
                    let p = PathBuf::from(arg);
                    if p.exists() && p.is_file() {
                        let ext = p
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if ext == "pdf" && !app.merge_files.contains(&p) {
                            app.merge_files.push(p);
                        }
                    }
                }
                if !app.merge_files.is_empty() {
                    app.active_tab = ActiveTab::Merge;
                    app.status = OperationStatus::Idle;
                    return (app, Task::none());
                }
            }

            let first_path = PathBuf::from(&args[0]);
            let ext = first_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if first_path.exists() && first_path.is_file() && ext == "pdf" {
                if let Ok(total) = get_pdf_page_count(&first_path) {
                    app.viewer_file = Some(first_path.clone());
                    app.viewer_total_pages = total;
                    app.viewer_current_page = 1;
                    let task = app.ensure_pages_rendered();
                    return (app, task);
                }
            } else {
                // Check if arguments contain image files
                let image_extensions = ["png", "jpg", "jpeg", "webp", "bmp"];
                for arg in &args {
                    if arg == "--gui-images" {
                        continue;
                    }
                    let p = PathBuf::from(arg);
                    if p.exists() && p.is_file() {
                        let e = p
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if image_extensions.contains(&e.as_str()) && !app.image_files.contains(&p) {
                            app.image_files.push(p);
                        }
                    }
                }

                if !app.image_files.is_empty() {
                    app.active_tab = ActiveTab::ImagesToPdf;
                    app.status = OperationStatus::Idle;
                }
            }
        }
        (app, Task::none())
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.active_tab == ActiveTab::View && self.viewer_file.is_some() {
            event::listen_with(|event, _status, _window| {
                Some(Message::EventOccurred(event))
            })
        } else {
            Subscription::none()
        }
    }

    fn spawn_render_page(&self, page: usize) -> Task<Message> {
        let file = match &self.viewer_file {
            Some(f) => f.clone(),
            None => return Task::none(),
        };

        let scale = self.viewer_scale;
        let rotation = self.viewer_rotation;
        Task::future(async move {
            match render_page_to_rgba(&file, page, scale) {
                Ok(mut data) => {
                    if rotation != 0 {
                        let (nw, nh, nbytes) = rotate_rgba_clockwise(data.width, data.height, &data.rgba_bytes, rotation);
                        data.width = nw;
                        data.height = nh;
                        data.rgba_bytes = nbytes;
                    }
                    Message::ViewerPageRendered(Ok(data))
                }
                Err(e) => Message::ViewerPageRendered(Err(e.to_string())),
            }
        })
    }

    fn ensure_pages_rendered(&mut self) -> Task<Message> {
        let curr = self.viewer_current_page;
        let total = self.viewer_total_pages;
        if total == 0 || self.viewer_file.is_none() {
            return Task::none();
        }

        let start_p = curr.saturating_sub(1).max(1);
        let end_p = (curr + 2).min(total);

        let mut tasks = Vec::new();
        for p in start_p..=end_p {
            if !self.viewer_pages.contains_key(&p) && !self.viewer_loading_pages.contains(&p) {
                self.viewer_loading_pages.insert(p);
                tasks.push(self.spawn_render_page(p));
            }
        }

        if self.viewer_pages.len() > 35 {
            self.viewer_pages.retain(|&p, _| (p as isize - curr as isize).abs() <= 12);
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn reload_text_drawer(&mut self) {
        if let Some(ref path) = self.viewer_file {
            let p = self.viewer_current_page;
            self.text_drawer_page = p;
            match extract_page_text(path, p) {
                Ok(txt) => {
                    let trimmed = txt.trim();
                    if trimmed.is_empty() {
                        self.text_drawer_content = text_editor::Content::with_text(
                            "(Halaman ini tidak mengandung teks digital, kemungkinan berupa gambar pindaian/scan)"
                        );
                    } else {
                        self.text_drawer_content = text_editor::Content::with_text(&txt);
                    }
                }
                Err(e) => {
                    self.text_drawer_content = text_editor::Content::with_text(
                        &format!("Gagal mengekstrak teks halaman: {}", e)
                    );
                }
            }
        }
    }

    fn get_snip_crop_coords(&self, page_w: u32, page_h: u32) -> (u32, u32, u32, u32) {
        if let Some((rx, ry, rw, rh)) = self.snip_rect {
            let x = (rx.max(0.0) as u32).min(page_w.saturating_sub(1));
            let y = (ry.max(0.0) as u32).min(page_h.saturating_sub(1));
            let w = (rw.max(10.0) as u32).min(page_w - x);
            let h = (rh.max(10.0) as u32).min(page_h - y);
            (x, y, w.max(1), h.max(1))
        } else {
            (0, 0, page_w, page_h)
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ViewerScrolled { relative_y, content_taller } => {
                self.viewer_scroll_relative_y = relative_y;
                self.viewer_content_taller = content_taller;
                if self.viewer_total_pages > 0 {
                    let approx_page = (((relative_y * (self.viewer_total_pages as f32)).floor() as usize) + 1)
                        .clamp(1, self.viewer_total_pages);
                    if approx_page != self.viewer_current_page {
                        self.viewer_current_page = approx_page;
                        self.viewer_page_input = approx_page.to_string();
                        if self.text_drawer_open {
                            self.reload_text_drawer();
                        }
                        if let Some((handle, data)) = self.viewer_pages.get(&approx_page) {
                            self.viewer_image_handle = Some(handle.clone());
                            self.viewer_current_data = Some(data.clone());
                        }
                        return self.ensure_pages_rendered();
                    }
                }
                Task::none()
            }
            Message::EventOccurred(event) => {
                if self.active_tab != ActiveTab::View || self.viewer_file.is_none() {
                    return Task::none();
                }

                match event {
                    Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        self.cursor_pos = (position.x, position.y);
                        if self.snip_mode {
                            if let Some(start) = self.snip_start {
                                self.snip_current = Some((position.x, position.y));
                                let pw = self.viewer_page_dims.0 as f32;
                                let ph = self.viewer_page_dims.1 as f32;
                                if pw > 0.0 && ph > 0.0 {
                                    let sidebar_w = if self.sidebar_open { 220.0 } else { 0.0 };
                                    let page_x = sidebar_w + 30.0;
                                    let page_y = 100.0;
                                    let rel_sx = (start.0 - page_x).clamp(0.0, pw);
                                    let rel_sy = (start.1 - page_y).clamp(0.0, ph);
                                    let rel_cx = (position.x - page_x).clamp(0.0, pw);
                                    let rel_cy = (position.y - page_y).clamp(0.0, ph);
                                    let x = rel_sx.min(rel_cx);
                                    let y = rel_sy.min(rel_cy);
                                    let w = (rel_cx - rel_sx).abs();
                                    let h = (rel_cy - rel_sy).abs();
                                    if w > 5.0 && h > 5.0 {
                                        self.snip_rect = Some((x, y, w, h));
                                    }
                                }
                            }
                        }
                    }
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        if self.snip_mode {
                            self.snip_start = Some(self.cursor_pos);
                            self.snip_current = Some(self.cursor_pos);
                        }
                    }
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        if self.snip_mode {
                            if let (Some(start), Some(curr)) = (self.snip_start, self.snip_current) {
                                let pw = self.viewer_page_dims.0 as f32;
                                let ph = self.viewer_page_dims.1 as f32;
                                if pw > 0.0 && ph > 0.0 {
                                    let sidebar_w = if self.sidebar_open { 220.0 } else { 0.0 };
                                    let page_x = sidebar_w + 30.0;
                                    let page_y = 100.0;
                                    let rel_sx = (start.0 - page_x).clamp(0.0, pw);
                                    let rel_sy = (start.1 - page_y).clamp(0.0, ph);
                                    let rel_cx = (curr.0 - page_x).clamp(0.0, pw);
                                    let rel_cy = (curr.1 - page_y).clamp(0.0, ph);
                                    let x = rel_sx.min(rel_cx);
                                    let y = rel_sy.min(rel_cy);
                                    let w = (rel_cx - rel_sx).abs();
                                    let h = (rel_cy - rel_sy).abs();
                                    if w > 10.0 && h > 10.0 {
                                        self.snip_rect = Some((x, y, w, h));
                                    }
                                }
                            }
                            self.snip_start = None;
                            self.snip_current = None;
                        }
                    }
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                        if self.snip_mode {
                            self.snip_mode = false;
                            self.snip_rect = None;
                            self.status = OperationStatus::Idle;
                        } else {
                            self.context_menu_open = !self.context_menu_open;
                        }
                    }
                    Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                        match key {
                            keyboard::Key::Named(keyboard::key::Named::PageDown) => {
                                if self.viewer_total_pages > 0 {
                                    let step = 1.0 / self.viewer_total_pages.max(1) as f32;
                                    let new_y = (self.viewer_scroll_relative_y + step).min(1.0);
                                    return iced::widget::operation::snap_to(
                                        "viewer_scroll",
                                        iced::widget::operation::RelativeOffset { y: Some(new_y), x: None },
                                    );
                                }
                            }
                            keyboard::Key::Named(keyboard::key::Named::PageUp) => {
                                if self.viewer_total_pages > 0 {
                                    let step = 1.0 / self.viewer_total_pages.max(1) as f32;
                                    let new_y = (self.viewer_scroll_relative_y - step).max(0.0);
                                    return iced::widget::operation::snap_to(
                                        "viewer_scroll",
                                        iced::widget::operation::RelativeOffset { y: Some(new_y), x: None },
                                    );
                                }
                            }
                            keyboard::Key::Named(keyboard::key::Named::Escape) => {
                                self.context_menu_open = false;
                                self.menu_open = false;
                                if self.snip_mode {
                                    self.snip_mode = false;
                                    self.snip_rect = None;
                                    self.status = OperationStatus::Idle;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::CloseContextMenu => {
                self.context_menu_open = false;
                Task::none()
            }
            Message::CloseMenu => {
                self.menu_open = false;
                Task::none()
            }
            Message::ToggleMenu => {
                self.menu_open = !self.menu_open;
                Task::none()
            }
            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                Task::none()
            }
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                self.menu_open = false;
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
                    if let Ok(total) = get_pdf_page_count(&file) {
                        self.viewer_file = Some(file);
                        self.viewer_total_pages = total;
                        self.viewer_current_page = 1;
                        self.viewer_page_input = "1".to_string();
                        self.viewer_rotation = 0;
                        self.viewer_pages.clear();
                        self.viewer_loading_pages.clear();
                        self.viewer_page_dims = (0, 0);
                        if self.text_drawer_open {
                            self.reload_text_drawer();
                        }
                        return self.ensure_pages_rendered();
                    }
                }
                Task::none()
            }
            Message::ViewerNextPage => {
                if self.viewer_current_page < self.viewer_total_pages {
                    self.viewer_current_page += 1;
                    self.viewer_page_input = self.viewer_current_page.to_string();
                    if self.text_drawer_open {
                        self.reload_text_drawer();
                    }
                    let target_ratio = (self.viewer_current_page - 1) as f32 / self.viewer_total_pages.max(1) as f32;
                    let snap_task = iced::widget::operation::snap_to(
                        "viewer_scroll",
                        iced::widget::operation::RelativeOffset { y: Some(target_ratio), x: None },
                    );
                    let render_task = self.ensure_pages_rendered();
                    return Task::batch(vec![snap_task, render_task]);
                }
                Task::none()
            }
            Message::ViewerPrevPage => {
                if self.viewer_current_page > 1 {
                    self.viewer_current_page -= 1;
                    self.viewer_page_input = self.viewer_current_page.to_string();
                    if self.text_drawer_open {
                        self.reload_text_drawer();
                    }
                    let target_ratio = (self.viewer_current_page - 1) as f32 / self.viewer_total_pages.max(1) as f32;
                    let snap_task = iced::widget::operation::snap_to(
                        "viewer_scroll",
                        iced::widget::operation::RelativeOffset { y: Some(target_ratio), x: None },
                    );
                    let render_task = self.ensure_pages_rendered();
                    return Task::batch(vec![snap_task, render_task]);
                }
                Task::none()
            }
            Message::ViewerGoToPage(page) => {
                if self.viewer_file.is_some()
                    && page >= 1
                    && page <= self.viewer_total_pages
                {
                    self.viewer_current_page = page;
                    self.viewer_page_input = page.to_string();
                    if self.text_drawer_open {
                        self.reload_text_drawer();
                    }
                    let target_ratio = (page - 1) as f32 / self.viewer_total_pages.max(1) as f32;
                    let snap_task = iced::widget::operation::snap_to(
                        "viewer_scroll",
                        iced::widget::operation::RelativeOffset { y: Some(target_ratio), x: None },
                    );
                    let render_task = self.ensure_pages_rendered();
                    return Task::batch(vec![snap_task, render_task]);
                }
                Task::none()
            }
            Message::ViewerPageInputChanged(val) => {
                self.viewer_page_input = val;
                Task::none()
            }
            Message::ViewerPageInputSubmitted => {
                if let Ok(p) = self.viewer_page_input.trim().parse::<usize>() {
                    let clamped = p.clamp(1, self.viewer_total_pages.max(1));
                    return self.update(Message::ViewerGoToPage(clamped));
                }
                self.viewer_page_input = self.viewer_current_page.to_string();
                Task::none()
            }
            Message::ViewerZoomIn => {
                if self.viewer_scale < 3.0 {
                    self.viewer_scale = (self.viewer_scale + 0.25).min(3.0);
                    self.viewer_pages.clear();
                    self.viewer_loading_pages.clear();
                    self.viewer_page_dims = (0, 0);
                    return self.ensure_pages_rendered();
                }
                Task::none()
            }
            Message::ViewerZoomOut => {
                if self.viewer_scale > 0.5 {
                    self.viewer_scale = (self.viewer_scale - 0.25).max(0.5);
                    self.viewer_pages.clear();
                    self.viewer_loading_pages.clear();
                    self.viewer_page_dims = (0, 0);
                    return self.ensure_pages_rendered();
                }
                Task::none()
            }
            Message::ViewerZoomReset => {
                self.viewer_scale = 1.25;
                self.viewer_pages.clear();
                self.viewer_loading_pages.clear();
                self.viewer_page_dims = (0, 0);
                return self.ensure_pages_rendered();
            }
            Message::ViewerFitWidth => {
                self.viewer_scale = match (self.viewer_scale * 100.0).round() as i32 {
                    125 => 1.5,
                    150 => 1.0,
                    100 => 1.25,
                    _ => 1.25,
                };
                self.viewer_pages.clear();
                self.viewer_loading_pages.clear();
                self.viewer_page_dims = (0, 0);
                return self.ensure_pages_rendered();
            }
            Message::ViewerRotateClockwise => {
                self.viewer_rotation = (self.viewer_rotation + 90) % 360;
                self.viewer_pages.clear();
                self.viewer_loading_pages.clear();
                self.viewer_page_dims = (0, 0);
                return self.ensure_pages_rendered();
            }
            Message::ViewerPrint => {
                if let Some(ref path) = self.viewer_file {
                    let p = path.clone();
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("powershell")
                            .args(&["-NoProfile", "-Command", &format!("Start-Process -FilePath '{}' -Verb Print", p.display())])
                            .spawn();
                    });
                }
                Task::none()
            }
            Message::ViewerPageRendered(res) => {
                match res {
                    Ok(page_data) => {
                        let p = page_data.page_number;
                        self.viewer_loading_pages.remove(&p);
                        if self.viewer_page_dims.0 == 0 {
                            self.viewer_page_dims = (page_data.width, page_data.height);
                        }
                        let handle = image::Handle::from_rgba(
                            page_data.width,
                            page_data.height,
                            page_data.rgba_bytes.clone(),
                        );
                        if p == self.viewer_current_page {
                            self.viewer_current_data = Some(page_data.clone());
                            self.viewer_image_handle = Some(handle.clone());
                        }
                        self.viewer_pages.insert(p, (handle, page_data));
                        if self.viewer_loading_pages.is_empty() {
                            self.status = OperationStatus::Idle;
                        }
                    }
                    Err(e) => {
                        self.status = OperationStatus::Error(format!("Gagal membaca PDF: {}", e));
                    }
                }
                Task::none()
            }
            Message::ViewerCopyPageToClipboard => {
                self.context_menu_open = false;
                let data_opt = self.viewer_pages.get(&self.viewer_current_page)
                    .map(|(_, d)| d.clone())
                    .or_else(|| self.viewer_current_data.clone());

                if let Some(ref data) = data_opt {
                    match arboard::Clipboard::new() {
                        Ok(mut cb) => {
                            let img = arboard::ImageData {
                                width: data.width as usize,
                                height: data.height as usize,
                                bytes: std::borrow::Cow::Borrowed(&data.rgba_bytes),
                            };
                            match cb.set_image(img) {
                                Ok(_) => {
                                    self.status = OperationStatus::Success(format!(
                                        "Halaman {} berhasil disalin ke clipboard! Siap dipaste (Ctrl+V) ke WA/Word.",
                                        self.viewer_current_page
                                    ));
                                }
                                Err(e) => {
                                    self.status = OperationStatus::Error(format!("Gagal menyalin gambar ke clipboard: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.status = OperationStatus::Error(format!("Gagal membuka clipboard: {}", e));
                        }
                    }
                } else {
                    self.status = OperationStatus::Error("Tidak ada halaman aktif untuk disalin.".to_string());
                }
                Task::none()
            }
            Message::ViewerExportPagePdf => {
                self.context_menu_open = false;
                let file = match &self.viewer_file {
                    Some(f) => f.clone(),
                    None => {
                        self.status = OperationStatus::Error("Tidak ada file PDF yang sedang dibuka.".to_string());
                        return Task::none();
                    }
                };

                let page = self.viewer_current_page as u32;
                let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("dokumen");
                let default_name = format!("{}_halaman_{}.pdf", stem, page);

                self.status = OperationStatus::Processing(format!("Mengekstrak halaman {} sebagai file PDF baru...", page));

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files (*.pdf)", &["pdf"])
                        .set_file_name(&default_name)
                        .set_title("Simpan Halaman ini sebagai File PDF Baru")
                        .save_file()
                        .await;

                    if let Some(handle) = dest {
                        let path = handle.path().to_path_buf();
                        match extract_pages_to_file(&file, &[page], &path) {
                            Ok(_) => Message::ViewerExportPdfCompleted(Ok(path)),
                            Err(e) => Message::ViewerExportPdfCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::ViewerExportPdfCompleted(Err("Penyimpanan PDF dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::ViewerExportPdfCompleted(res) => {
                match res {
                    Ok(path) => {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                        self.status = OperationStatus::Success(format!(
                            "Halaman PDF berhasil disimpan sebagai file baru: {}",
                            name
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
            Message::ViewerExportPagePng | Message::ViewerExportPageJpg => {
                self.context_menu_open = false;
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
            Message::ViewerExportAll => {
                let file = match &self.viewer_file {
                    Some(f) => f.clone(),
                    None => return Task::none(),
                };

                self.status = OperationStatus::Processing(
                    "Mengekspor seluruh halaman PDF ke gambar...".to_string(),
                );

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .set_title("Pilih Folder untuk Menyimpan Semua Halaman Gambar")
                        .pick_folder()
                        .await;

                    if let Some(folder) = dest {
                        let out_dir = folder.path().to_path_buf();
                        match export_all_pdf_pages_to_images(&file, &out_dir, ExportImageFormat::Png, 2.0) {
                            Ok(paths) => Message::ViewerExportAllCompleted(Ok(paths.len())),
                            Err(e) => Message::ViewerExportAllCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::ViewerExportAllCompleted(Err("Ekspor gambar dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::ViewerExportAllCompleted(res) => {
                match res {
                    Ok(count) => {
                        self.status = OperationStatus::Success(format!(
                            "Berhasil mengekspor seluruh {} halaman ke folder tujuan!",
                            count
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

            // --- TEXT DRAWER ACTIONS (Opsi 1: Blok & Salin Teks) ---
            Message::ToggleTextDrawer => {
                self.text_drawer_open = !self.text_drawer_open;
                if self.text_drawer_open {
                    self.reload_text_drawer();
                }
                Task::none()
            }
            Message::CloseTextDrawer => {
                self.text_drawer_open = false;
                Task::none()
            }
            Message::PageTextEditorAction(action) => {
                self.text_drawer_content.perform(action);
                Task::none()
            }
            Message::CopyPageText => {
                let full_text = self.text_drawer_content.text();
                if full_text.trim().is_empty() {
                    self.status = OperationStatus::Error("Tidak ada teks untuk disalin.".to_string());
                } else {
                    match arboard::Clipboard::new() {
                        Ok(mut cb) => {
                            match cb.set_text(full_text.clone()) {
                                Ok(_) => {
                                    let preview = if full_text.len() > 50 {
                                        format!("{}...", &full_text[..47].replace(['\r', '\n'], " "))
                                    } else {
                                        full_text.replace(['\r', '\n'], " ")
                                    };
                                    self.status = OperationStatus::Success(format!(
                                        "Teks halaman {} berhasil disalin: \"{}\"",
                                        self.viewer_current_page, preview
                                    ));
                                }
                                Err(e) => {
                                    self.status = OperationStatus::Error(format!("Gagal menyalin teks: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.status = OperationStatus::Error(format!("Gagal membuka clipboard: {}", e));
                        }
                    }
                }
                Task::none()
            }

            // --- SNIP TOOL ACTIONS (Opsi 2: Cuplik Area) ---
            Message::ToggleSnipMode => {
                self.snip_mode = !self.snip_mode;
                self.snip_start = None;
                self.snip_current = None;
                self.snip_rect = None;
                if self.snip_mode {
                    self.context_menu_open = false;
                    self.status = OperationStatus::Processing(
                        "Mode Cuplik Aktif: Seret mouse pada dokumen untuk memilih area, atau pilih preset. Tekan ESC untuk batal.".to_string()
                    );
                } else {
                    self.status = OperationStatus::Idle;
                }
                Task::none()
            }
            Message::CancelSnip => {
                self.snip_mode = false;
                self.snip_start = None;
                self.snip_current = None;
                self.snip_rect = None;
                self.status = OperationStatus::Idle;
                Task::none()
            }
            Message::SnipPresetFull => {
                self.snip_rect = None;
                self.status = OperationStatus::Processing("Preset: Seluruh Halaman dipilih.".to_string());
                Task::none()
            }
            Message::SnipPresetTop => {
                let (pw, ph) = self.viewer_page_dims;
                let pw = if pw > 0 { pw as f32 } else { 800.0 };
                let ph = if ph > 0 { ph as f32 } else { 1000.0 };
                self.snip_rect = Some((0.0, 0.0, pw, ph / 2.0));
                self.status = OperationStatus::Processing("Preset: 50% Bagian Atas dipilih.".to_string());
                Task::none()
            }
            Message::SnipPresetBottom => {
                let (pw, ph) = self.viewer_page_dims;
                let pw = if pw > 0 { pw as f32 } else { 800.0 };
                let ph = if ph > 0 { ph as f32 } else { 1000.0 };
                self.snip_rect = Some((0.0, ph / 2.0, pw, ph / 2.0));
                self.status = OperationStatus::Processing("Preset: 50% Bagian Bawah dipilih.".to_string());
                Task::none()
            }
            Message::SnipPresetCenter => {
                let (pw, ph) = self.viewer_page_dims;
                let pw = if pw > 0 { pw as f32 } else { 800.0 };
                let ph = if ph > 0 { ph as f32 } else { 1000.0 };
                self.snip_rect = Some((pw * 0.1, ph * 0.2, pw * 0.8, ph * 0.6));
                self.status = OperationStatus::Processing("Preset: Bagian Tengah dipilih.".to_string());
                Task::none()
            }
            Message::CopySnipArea => {
                let data_opt = self.viewer_pages.get(&self.viewer_current_page)
                    .map(|(_, d)| d.clone())
                    .or_else(|| self.viewer_current_data.clone());

                if let Some(ref data) = data_opt {
                    let (crop_x, crop_y, crop_w, crop_h) = self.get_snip_crop_coords(data.width, data.height);
                    match crop_rgba(data.width, data.height, &data.rgba_bytes, crop_x, crop_y, crop_w, crop_h) {
                        Ok((w, h, cropped_rgba)) => {
                            match arboard::Clipboard::new() {
                                Ok(mut cb) => {
                                    let img = arboard::ImageData {
                                        width: w as usize,
                                        height: h as usize,
                                        bytes: std::borrow::Cow::Borrowed(&cropped_rgba),
                                    };
                                    match cb.set_image(img) {
                                        Ok(_) => {
                                            self.snip_mode = false;
                                            self.snip_rect = None;
                                            self.status = OperationStatus::Success(format!(
                                                "Cuplikan area ({}x{} px) berhasil disalin ke clipboard! Siap dipaste (Ctrl+V) ke WA/Word.",
                                                w, h
                                            ));
                                        }
                                        Err(e) => {
                                            self.status = OperationStatus::Error(format!("Gagal menyalin cuplikan ke clipboard: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.status = OperationStatus::Error(format!("Gagal membuka clipboard: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.status = OperationStatus::Error(e);
                        }
                    }
                } else {
                    self.status = OperationStatus::Error("Tidak ada halaman aktif untuk dicuplik.".to_string());
                }
                Task::none()
            }
            Message::SaveSnipArea => {
                let data_opt = self.viewer_pages.get(&self.viewer_current_page)
                    .map(|(_, d)| d.clone())
                    .or_else(|| self.viewer_current_data.clone());

                if let Some(ref data) = data_opt {
                    let (crop_x, crop_y, crop_w, crop_h) = self.get_snip_crop_coords(data.width, data.height);
                    match crop_rgba(data.width, data.height, &data.rgba_bytes, crop_x, crop_y, crop_w, crop_h) {
                        Ok((w, h, cropped_rgba)) => {
                            let curr_p = self.viewer_current_page;
                            let default_name = format!("cuplikan_hal_{}_{}x{}.png", curr_p, w, h);
                            self.status = OperationStatus::Processing("Menyimpan cuplikan area ke file PNG...".to_string());
                            Task::future(async move {
                                let dest = rfd::AsyncFileDialog::new()
                                    .add_filter("PNG Image (*.png)", &["png"])
                                    .set_file_name(&default_name)
                                    .set_title("Simpan Cuplikan Area Sebagai PNG")
                                    .save_file()
                                    .await;

                                if let Some(handle) = dest {
                                    let path = handle.path().to_path_buf();
                                    match ::image::save_buffer_with_format(
                                        &path,
                                        &cropped_rgba,
                                        w,
                                        h,
                                        ::image::ExtendedColorType::Rgba8,
                                        ::image::ImageFormat::Png,
                                    ) {
                                        Ok(_) => Message::SnipAreaSaved(Ok(path)),
                                        Err(e) => Message::SnipAreaSaved(Err(e.to_string())),
                                    }
                                } else {
                                    Message::SnipAreaSaved(Err("Penyimpanan cuplikan dibatalkan.".to_string()))
                                }
                            })
                        }
                        Err(e) => {
                            self.status = OperationStatus::Error(e);
                            Task::none()
                        }
                    }
                } else {
                    self.status = OperationStatus::Error("Tidak ada halaman aktif untuk dicuplik.".to_string());
                    Task::none()
                }
            }
            Message::SnipAreaSaved(res) => {
                match res {
                    Ok(path) => {
                        self.snip_mode = false;
                        self.snip_rect = None;
                        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("cuplikan.png");
                        self.status = OperationStatus::Success(format!("Cuplikan area berhasil disimpan: {}", name));
                    }
                    Err(msg) => {
                        if msg.contains("dibatalkan") {
                            self.status = OperationStatus::Idle;
                        } else {
                            self.status = OperationStatus::Error(format!("Gagal menyimpan cuplikan: {}", msg));
                        }
                    }
                }
                Task::none()
            }

            // --- PDF TO IMAGES ACTIONS ---
            Message::PickPdfToImgFile => {
                Task::future(async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("PDF files", &["pdf"])
                        .set_title("Pilih File PDF untuk Diekspor Jadi Gambar")
                        .pick_file()
                        .await;

                    Message::PdfToImgFileSelected(file.map(|h| h.path().to_path_buf()))
                })
            }
            Message::PdfToImgFileSelected(opt) => {
                if let Some(path) = opt {
                    match get_pdf_page_count(&path) {
                        Ok(count) => {
                            self.pdf_to_img_total_pages = count;
                            self.pdf_to_img_file = Some(path);
                            self.pdf_to_img_range = format!("1-{}", count);
                            self.status = OperationStatus::Idle;
                        }
                        Err(e) => {
                            self.status = OperationStatus::Error(format!(
                                "Gagal membaca file PDF: {}",
                                e
                            ));
                        }
                    }
                }
                Task::none()
            }
            Message::PdfToImgRangeChanged(val) => {
                self.pdf_to_img_range = val;
                Task::none()
            }
            Message::PdfToImgFormatChanged(fmt) => {
                self.pdf_to_img_format = fmt;
                Task::none()
            }
            Message::PdfToImgScaleChanged(scale) => {
                self.pdf_to_img_scale = scale;
                Task::none()
            }
            Message::PdfToImgToggleAllPages(all) => {
                self.pdf_to_img_all_pages = all;
                Task::none()
            }
            Message::RunPdfToImages => {
                let file = match &self.pdf_to_img_file {
                    Some(f) => f.clone(),
                    None => {
                        self.status = OperationStatus::Error(
                            "Pilih file PDF terlebih dahulu.".to_string(),
                        );
                        return Task::none();
                    }
                };

                let total_pages = self.pdf_to_img_total_pages;
                let pages: Vec<usize> = if self.pdf_to_img_all_pages {
                    (1..=total_pages).collect()
                } else {
                    match parse_page_ranges(&self.pdf_to_img_range, total_pages) {
                        Ok(p) => p.into_iter().map(|n| n as usize).collect(),
                        Err(e) => {
                            self.status = OperationStatus::Error(format!(
                                "Format rentang halaman tidak valid: {}",
                                e
                            ));
                            return Task::none();
                        }
                    }
                };

                if pages.is_empty() {
                    self.status = OperationStatus::Error("Tidak ada halaman yang dipilih.".to_string());
                    return Task::none();
                }

                let format = self.pdf_to_img_format;
                let scale = self.pdf_to_img_scale;
                let count = pages.len();

                self.status = OperationStatus::Processing(format!(
                    "Mengekspor {} halaman ke {}...",
                    count,
                    format.extension().to_uppercase()
                ));

                Task::future(async move {
                    let dest = rfd::AsyncFileDialog::new()
                        .set_title("Pilih Folder untuk Menyimpan Hasil Gambar")
                        .pick_folder()
                        .await;

                    if let Some(folder) = dest {
                        let out_dir = folder.path().to_path_buf();
                        match export_pdf_pages_to_dir(&file, &pages, &out_dir, format, scale) {
                            Ok(paths) => Message::PdfToImagesCompleted(Ok(paths.len())),
                            Err(e) => Message::PdfToImagesCompleted(Err(e.to_string())),
                        }
                    } else {
                        Message::PdfToImagesCompleted(Err("Penyimpanan dibatalkan oleh pengguna.".to_string()))
                    }
                })
            }
            Message::PdfToImagesCompleted(res) => {
                match res {
                    Ok(count) => {
                        self.status = OperationStatus::Success(format!(
                            "Berhasil mengekspor {} halaman gambar ke folder tujuan!",
                            count
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
        let top_bar = self.view_top_bar();
        let status_bar = self.view_status();

        let content: Element<'_, Message> = match self.active_tab {
            ActiveTab::View => self.view_viewer_tab(),
            ActiveTab::PdfToImages => self.view_pdf_to_images_tab(),
            ActiveTab::ImagesToPdf => self.view_images_to_pdf_tab(),
            ActiveTab::Merge => self.view_merge_tab(),
            ActiveTab::Split => self.view_split_tab(),
            ActiveTab::Compress => self.view_compress_tab(),
        };

        let mut body_row = row![];

        // Left sidebar (Edge-style table of contents / page thumbnails)
        if self.active_tab == ActiveTab::View && self.sidebar_open && self.viewer_file.is_some() {
            body_row = body_row.push(
                container(self.view_sidebar())
                    .width(220)
                    .height(Length::Fill)
            );
        }

        // Center PDF Viewer or active tool
        body_row = body_row.push(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(if self.active_tab == ActiveTab::View { 0 } else { 16 })
        );

        // Right Text Drawer (Option 1: Blok & Salin Teks)
        if self.active_tab == ActiveTab::View && self.text_drawer_open && self.viewer_file.is_some() {
            body_row = body_row.push(self.view_text_drawer());
        }

        // Right tools drawer
        if self.menu_open {
            body_row = body_row.push(
                container(self.view_menu_dropdown())
                    .padding(Padding::from([8, 12]))
            );
        }

        let main_layout = if self.active_tab == ActiveTab::View {
            column![
                top_bar,
                body_row.width(Length::Fill).height(Length::Fill)
            ]
        } else {
            column![
                top_bar,
                status_bar,
                body_row.width(Length::Fill).height(Length::Fill)
            ]
        }
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0)
            .into()
    }
}

fn edge_bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0x27, 0x27, 0x2b))),
        border: Border {
            color: Color::from_rgb8(0x1a, 0x1a, 0x1e),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn edge_btn(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb8(0x3a, 0x3a, 0x42))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgb8(0x4c, 0x4c, 0x56),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb8(0x44, 0x44, 0x4e))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgb8(0x00, 0x78, 0xd4),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        },
        button::Status::Disabled => button::Style {
            background: None,
            text_color: Color::from_rgb8(0x60, 0x60, 0x68),
            ..Default::default()
        },
        button::Status::Active => button::Style {
            background: None,
            text_color: Color::from_rgb8(0xdd, 0xdd, 0xe2),
            border: Border {
                color: Color::TRANSPARENT,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        },
    }
}

fn edge_btn_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::from_rgb8(0x00, 0x5a, 0x9e))),
        text_color: Color::WHITE,
        border: Border {
            color: Color::from_rgb8(0x00, 0x78, 0xd4),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn edge_btn_primary(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb8(0x10, 0x6e, 0xbe))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgb8(0x28, 0x86, 0xde),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        },
        _ => button::Style {
            background: Some(Background::Color(Color::from_rgb8(0x00, 0x78, 0xd4))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgb8(0x00, 0x78, 0xd4),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        },
    }
}

impl App {
    fn view_sidebar(&self) -> Element<'_, Message> {
        let header = row![
            text("Daftar Halaman").size(12).font(BOLD).style(text::secondary),
            Space::new().width(Length::Fill),
            button(text("✕").size(11))
                .style(edge_btn)
                .padding(Padding::from([2, 6]))
                .on_press(Message::ToggleSidebar)
        ]
        .align_y(Alignment::Center)
        .padding(Padding::from([10, 12]));

        let mut page_list = column![].spacing(3).padding(Padding::from([4, 8]));

        for p in 1..=self.viewer_total_pages {
            let is_current = p == self.viewer_current_page;
            let page_btn = button(
                row![
                    container(text(format!("{}", p)).size(10).font(BOLD))
                        .padding(Padding::from([2, 6]))
                        .style(if is_current { container::rounded_box } else { |_t: &Theme| container::Style::default() }),
                    text(format!("Halaman {}", p))
                        .size(11)
                        .font(if is_current { BOLD } else { Font::DEFAULT }),
                    Space::new().width(Length::Fill),
                    if is_current {
                        text("●").size(7).style(text::primary)
                    } else {
                        text("")
                    }
                ]
                .spacing(8)
                .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .style(if is_current { edge_btn_active } else { edge_btn })
            .padding(Padding::from([5, 8]))
            .on_press(Message::ViewerGoToPage(p));

            page_list = page_list.push(page_btn);
        }

        let sidebar_content = column![
            header,
            rule::horizontal(1),
            scrollable(page_list).height(Length::Fill)
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill);

        container(sidebar_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0x1e, 0x1f, 0x24))),
                border: Border {
                    color: Color::from_rgb8(0x2a, 0x2b, 0x32),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_text_drawer(&self) -> Element<'_, Message> {
        let header = row![
            text(format!("📝 Teks Halaman {}", self.viewer_current_page))
                .size(12)
                .font(BOLD)
                .style(text::primary),
            Space::new().width(Length::Fill),
            button(
                row![text("📋"), text("Salin Semua").size(10)]
                    .spacing(3)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .padding(Padding::from([3, 7]))
            .on_press(Message::CopyPageText),
            button(text("✕").size(11))
                .style(edge_btn)
                .padding(Padding::from([3, 6]))
                .on_press(Message::CloseTextDrawer)
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        let hint = text("💡 Blok teks dengan mouse lalu tekan Ctrl+C")
            .size(10)
            .style(text::secondary);

        let editor = text_editor(&self.text_drawer_content)
            .on_action(Message::PageTextEditorAction)
            .height(Length::Fill)
            .padding(Padding::from([8, 10]));

        let card = column![
            header,
            hint,
            rule::horizontal(1),
            editor
        ]
        .spacing(6)
        .width(Length::Fill)
        .height(Length::Fill);

        container(card)
            .width(360)
            .height(Length::Fill)
            .padding(Padding::from([8, 10]))
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0x1e, 0x1e, 0x22))),
                border: Border {
                    color: Color::from_rgb8(0x33, 0x33, 0x3a),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_top_bar(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let logo_badge = container(
            text("PD").size(11).font(BOLD).style(text::primary)
        )
        .padding(Padding::from([3, 7]))
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0x18, 0x18, 0x1c))),
            border: Border {
                color: Color::from_rgb8(0x38, 0x38, 0x42),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

        let menu_btn = button(
            row![
                text(if self.menu_open { "✕" } else { "⋯" }).size(14).font(BOLD),
                text("Alat").size(11)
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .style(if self.menu_open { edge_btn_active } else { edge_btn })
        .on_press(Message::ToggleMenu)
        .padding(Padding::from([4, 10]));

        if self.active_tab == ActiveTab::View {
            let sidebar_btn = button(
                text("▤").size(14).font(BOLD)
            )
            .style(if self.sidebar_open { edge_btn_active } else { edge_btn })
            .padding(Padding::from([4, 8]));
            let sidebar_btn = if self.viewer_file.is_some() && !is_processing {
                sidebar_btn.on_press(Message::ToggleSidebar)
            } else {
                sidebar_btn
            };

            let open_btn = button(
                row![text("📂").size(13), text("Buka").size(11).font(BOLD)]
                    .spacing(5)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .padding(Padding::from([4, 8]));
            let open_btn = if is_processing { open_btn } else { open_btn.on_press(Message::PickViewerFile) };

            let copy_btn = button(
                row![text("📋").size(12), text("Salin").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .padding(Padding::from([4, 8]));
            let copy_btn = if self.viewer_file.is_some() && !is_processing {
                copy_btn.on_press(Message::ViewerCopyPageToClipboard)
            } else {
                copy_btn
            };

            let save_btn = button(
                row![text("💾").size(12), text("Simpan").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .padding(Padding::from([4, 8]));
            let save_btn = if self.viewer_file.is_some() && !is_processing {
                save_btn.on_press(Message::ViewerExportPagePng)
            } else {
                save_btn
            };

            let text_drawer_btn = button(
                row![text("📝").size(12), text("Teks").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .style(if self.text_drawer_open { edge_btn_active } else { edge_btn })
            .padding(Padding::from([4, 8]));
            let text_drawer_btn = if self.viewer_file.is_some() && !is_processing {
                text_drawer_btn.on_press(Message::ToggleTextDrawer)
            } else {
                text_drawer_btn
            };

            let snip_btn = button(
                row![text("✂️").size(12), text("Cuplik").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .style(if self.snip_mode { edge_btn_active } else { edge_btn })
            .padding(Padding::from([4, 8]));
            let snip_btn = if self.viewer_file.is_some() && !is_processing {
                snip_btn.on_press(Message::ToggleSnipMode)
            } else {
                snip_btn
            };

            let export_all_btn = button(
                row![text("📁").size(12), text("Ekspor").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .padding(Padding::from([4, 8]));
            let export_all_btn = if self.viewer_file.is_some() && !is_processing {
                export_all_btn.on_press(Message::ViewerExportAll)
            } else {
                export_all_btn
            };

            let file_name = self.viewer_file.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Belum ada dokumen".to_string());

            let display_name = if file_name.len() > 20 {
                format!("{}...", &file_name[..18])
            } else {
                file_name
            };

            let sep = || text("│").size(12).style(|_t: &Theme| text::Style { color: Some(Color::from_rgb8(0x48, 0x48, 0x52)) });

            let zoom_out = button(text("−").size(13).font(BOLD))
                .style(edge_btn)
                .padding(Padding::from([4, 8]));
            let zoom_out = if self.viewer_file.is_some() && !is_processing { zoom_out.on_press(Message::ViewerZoomOut) } else { zoom_out };

            let zoom_lbl = button(text(format!("{:.0}%", self.viewer_scale * 100.0)).size(11).font(BOLD))
                .style(edge_btn)
                .padding(Padding::from([4, 6]));
            let zoom_lbl = if self.viewer_file.is_some() && !is_processing { zoom_lbl.on_press(Message::ViewerZoomReset) } else { zoom_lbl };

            let zoom_in = button(text("+").size(13).font(BOLD))
                .style(edge_btn)
                .padding(Padding::from([4, 8]));
            let zoom_in = if self.viewer_file.is_some() && !is_processing { zoom_in.on_press(Message::ViewerZoomIn) } else { zoom_in };

            let fit_width_btn = button(text("↔").size(12).font(BOLD))
                .style(edge_btn)
                .padding(Padding::from([4, 8]));
            let fit_width_btn = if self.viewer_file.is_some() && !is_processing { fit_width_btn.on_press(Message::ViewerFitWidth) } else { fit_width_btn };

            let prev_btn = button(text("◀").size(10))
                .style(edge_btn)
                .padding(Padding::from([4, 7]));
            let prev_btn = if self.viewer_file.is_some() && self.viewer_current_page > 1 && !is_processing {
                prev_btn.on_press(Message::ViewerPrevPage)
            } else {
                prev_btn
            };

            let next_btn = button(text("▶").size(10))
                .style(edge_btn)
                .padding(Padding::from([4, 7]));
            let next_btn = if self.viewer_file.is_some() && self.viewer_current_page < self.viewer_total_pages && !is_processing {
                next_btn.on_press(Message::ViewerNextPage)
            } else {
                next_btn
            };

            let page_box = container(
                row![
                    text_input("", &self.viewer_page_input)
                        .size(11)
                        .width(36)
                        .padding(Padding::from([2, 4]))
                        .on_input(Message::ViewerPageInputChanged)
                        .on_submit(Message::ViewerPageInputSubmitted),
                    text(format!("dari {}", self.viewer_total_pages.max(1)))
                        .size(11)
                        .font(BOLD)
                        .style(|_t: &Theme| text::Style { color: Some(Color::from_rgb8(0xaa, 0xaa, 0xb4)) })
                ]
                .spacing(5)
                .align_y(Alignment::Center)
            )
            .padding(Padding::from([2, 6]))
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0x1a, 0x1a, 0x1e))),
                border: Border {
                    color: Color::from_rgb8(0x3e, 0x3e, 0x48),
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            });

            let rotate_btn = button(text("⟳").size(14).font(BOLD))
                .style(edge_btn)
                .padding(Padding::from([4, 8]));
            let rotate_btn = if self.viewer_file.is_some() && !is_processing {
                rotate_btn.on_press(Message::ViewerRotateClockwise)
            } else {
                rotate_btn
            };

            let print_btn = button(
                row![text("🖨️").size(12), text("Cetak").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .padding(Padding::from([4, 8]));
            let print_btn = if self.viewer_file.is_some() && !is_processing {
                print_btn.on_press(Message::ViewerPrint)
            } else {
                print_btn
            };

            let tools_btn = button(
                row![
                    text("🛠️").size(12),
                    text("Alat PDF").size(11).font(BOLD),
                    text("▾").size(9)
                ]
                .spacing(5)
                .align_y(Alignment::Center)
            )
            .style(if self.menu_open { edge_btn_active } else { edge_btn_primary })
            .padding(Padding::from([4, 10]))
            .on_press(Message::ToggleMenu);

            let status_indicator: Element<'_, Message> = match &self.status {
                OperationStatus::Processing(msg) => {
                    row![
                        text("⏳").size(11),
                        text(msg).size(11).style(text::primary)
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into()
                }
                OperationStatus::Success(msg) => {
                    text(msg).size(11).style(text::success).into()
                }
                OperationStatus::Error(msg) => {
                    text(msg).size(11).style(text::danger).into()
                }
                OperationStatus::Idle => Space::new().width(0).into(),
            };

            let title_with_status = row![
                text(display_name).size(12).font(BOLD).style(text::secondary),
                status_indicator
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let top_row = row![
                logo_badge,
                sidebar_btn,
                open_btn,
                sep(),
                copy_btn,
                save_btn,
                text_drawer_btn,
                snip_btn,
                export_all_btn,
                sep(),
                title_with_status,
                Space::new().width(Length::Fill),
                zoom_out,
                zoom_lbl,
                zoom_in,
                fit_width_btn,
                sep(),
                prev_btn,
                page_box,
                next_btn,
                sep(),
                rotate_btn,
                Space::new().width(Length::Fill),
                print_btn,
                tools_btn
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            container(top_row)
                .width(Length::Fill)
                .padding(Padding::from([4, 10]))
                .style(edge_bar_style)
                .into()
        } else {
            let back_btn = button(
                row![text("←"), text("Kembali ke Dokumen").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center)
            )
            .style(edge_btn)
            .on_press(Message::SwitchTab(ActiveTab::View))
            .padding(Padding::from([4, 10]));

            let tab_title = match self.active_tab {
                ActiveTab::View => "Viewer Dokumen",
                ActiveTab::PdfToImages => "📷 Konversi PDF ke Gambar",
                ActiveTab::ImagesToPdf => "🖼️ Konversi Gambar ke PDF",
                ActiveTab::Merge => "🔀 Gabungkan Dokumen PDF (Merge)",
                ActiveTab::Split => "✂️ Pisahkan Halaman PDF (Split)",
                ActiveTab::Compress => "🗜️ Kompres Dokumen PDF",
            };

            let top_row = row![
                logo_badge,
                back_btn,
                text(tab_title).size(13).font(BOLD),
                Space::new().width(Length::Fill),
                menu_btn
            ]
            .spacing(10)
            .align_y(Alignment::Center);

            container(top_row)
                .width(Length::Fill)
                .padding(Padding::from([4, 10]))
                .style(edge_bar_style)
                .into()
        }
    }

    fn view_menu_dropdown(&self) -> Element<'_, Message> {
        let menu_item = |label: &'static str, tab: ActiveTab, current: ActiveTab| {
            let is_current = tab == current;
            button(
                row![
                    text(label).size(12).font(if is_current { BOLD } else { Font::DEFAULT }),
                    Space::new().width(Length::Fill),
                    if is_current { text("●").size(9).style(text::primary) } else { text("") }
                ]
                .align_y(Alignment::Center)
            )
            .style(if is_current { edge_btn_active } else { edge_btn })
            .width(Length::Fill)
            .padding(Padding::from([8, 12]))
            .on_press(Message::SwitchTab(tab))
        };

        let mut menu_content = column![
            row![
                text("ALAT & UTILITAS").size(11).font(BOLD).style(text::secondary),
                Space::new().width(Length::Fill),
                button(text("✕").size(11))
                    .style(edge_btn)
                    .on_press(Message::CloseMenu)
            ]
            .align_y(Alignment::Center),
            rule::horizontal(1),
            menu_item("👁️ View PDF (Reader)", ActiveTab::View, self.active_tab),
            menu_item("📷 PDF ke Gambar", ActiveTab::PdfToImages, self.active_tab),
            menu_item("🖼️ Gambar ke PDF", ActiveTab::ImagesToPdf, self.active_tab),
            menu_item("🔀 Gabungkan PDF (Merge)", ActiveTab::Merge, self.active_tab),
            menu_item("✂️ Pisahkan PDF (Split)", ActiveTab::Split, self.active_tab),
            menu_item("🗜️ Kompres Dokumen PDF", ActiveTab::Compress, self.active_tab),
        ]
        .spacing(6)
        .width(260);

        if self.viewer_file.is_some() {
            let action_btn = |icon: &'static str, label: &'static str, msg: Message| {
                button(
                    row![text(icon).size(13), text(label).size(11).font(BOLD)]
                        .spacing(6)
                        .align_y(Alignment::Center)
                )
                .style(edge_btn)
                .width(Length::Fill)
                .padding(Padding::from([6, 10]))
                .on_press(msg)
            };

            menu_content = menu_content
                .push(rule::horizontal(1))
                .push(text(format!("AKSI HALAMAN {}", self.viewer_current_page)).size(10).font(BOLD).style(text::secondary))
                .push(action_btn("📋", "Salin Gambar ke Clipboard", Message::ViewerCopyPageToClipboard))
                .push(action_btn("🖼️", "Simpan Gambar (PNG)...", Message::ViewerExportPagePng))
                .push(action_btn("📷", "Simpan Gambar (JPG)...", Message::ViewerExportPageJpg))
                .push(action_btn("📄", "Simpan Halaman sebagai PDF...", Message::ViewerExportPagePdf))
                .push(action_btn("📁", "Ekspor Semua Halaman...", Message::ViewerExportAll));
        }

        container(menu_content)
            .padding(Padding::from([12, 14]))
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0x22, 0x22, 0x28))),
                border: Border {
                    color: Color::from_rgb8(0x3a, 0x3a, 0x46),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.5),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 14.0,
                },
                ..Default::default()
            })
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

    // --- VIEWER TAB VIEW (CONTINUOUS VERTICAL SCROLL PDF READER) ---
    fn view_viewer_tab(&self) -> Element<'_, Message> {
        if self.viewer_file.is_some() && self.viewer_total_pages > 0 {
            let mut pages_col = column![].spacing(18).align_x(Alignment::Center);

            for p in 1..=self.viewer_total_pages {
                let page_element: Element<'_, Message> = if let Some((handle, _)) = self.viewer_pages.get(&p) {
                    let page_img = image(handle.clone());
                    container(page_img)
                        .padding(0)
                        .style(|_t: &Theme| container::Style {
                            background: Some(Background::Color(Color::WHITE)),
                            border: Border {
                                color: Color::from_rgba8(0, 0, 0, 0.25),
                                width: 1.0,
                                radius: 2.0.into(),
                            },
                            shadow: Shadow {
                                color: Color::from_rgba8(0, 0, 0, 0.45),
                                offset: Vector::new(0.0, 4.0),
                                blur_radius: 12.0,
                            },
                            ..Default::default()
                        })
                        .into()
                } else {
                    let (pw, ph) = if self.viewer_page_dims.0 > 0 {
                        (self.viewer_page_dims.0 as f32, self.viewer_page_dims.1 as f32)
                    } else {
                        (640.0 * self.viewer_scale, 890.0 * self.viewer_scale)
                    };
                    container(
                        column![
                            text(format!("Halaman {}", p)).size(13).font(BOLD).style(text::secondary),
                            text("Memuat tampilan...").size(11).style(text::secondary),
                        ]
                        .spacing(6)
                        .align_x(Alignment::Center)
                    )
                    .width(pw)
                    .height(ph)
                    .center_x(pw)
                    .center_y(ph)
                    .style(|_t: &Theme| container::Style {
                        background: Some(Background::Color(Color::WHITE)),
                        border: Border {
                            color: Color::from_rgba8(0, 0, 0, 0.15),
                            width: 1.0,
                            radius: 2.0.into(),
                        },
                        shadow: Shadow {
                            color: Color::from_rgba8(0, 0, 0, 0.3),
                            offset: Vector::new(0.0, 3.0),
                            blur_radius: 8.0,
                        },
                        ..Default::default()
                    })
                    .into()
                };

                pages_col = pages_col.push(page_element);
            }

            let scroll_content = container(pages_col)
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding::from([20, 24]));

            let main_scroll = scrollable(scroll_content)
                .id("viewer_scroll")
                .height(Length::Fill)
                .on_scroll(|vp| Message::ViewerScrolled {
                    relative_y: vp.relative_offset().y,
                    content_taller: true,
                });

            let canvas: Element<'_, Message> = if self.snip_mode {
                let rect_info = if let Some((_, _, rw, rh)) = self.snip_rect {
                    format!("📐 Area: {:.0} × {:.0} px", rw, rh)
                } else {
                    "📐 Seluruh Halaman".to_string()
                };

                let snip_bar = container(
                    row![
                        text(format!("✂️ Cuplik Halaman {}", self.viewer_current_page))
                            .size(12)
                            .font(BOLD)
                            .style(text::primary),
                        text(rect_info).size(11).style(text::secondary),
                        rule::vertical(1),
                        button(text("100% Full").size(10)).style(edge_btn).padding(Padding::from([3, 6])).on_press(Message::SnipPresetFull),
                        button(text("50% Atas").size(10)).style(edge_btn).padding(Padding::from([3, 6])).on_press(Message::SnipPresetTop),
                        button(text("50% Bawah").size(10)).style(edge_btn).padding(Padding::from([3, 6])).on_press(Message::SnipPresetBottom),
                        button(text("Tengah").size(10)).style(edge_btn).padding(Padding::from([3, 6])).on_press(Message::SnipPresetCenter),
                        Space::new().width(Length::Fill),
                        button(
                            row![text("📋"), text("Salin Cuplikan").size(11).font(BOLD)]
                                .spacing(4)
                                .align_y(Alignment::Center)
                        )
                        .style(edge_btn_primary)
                        .padding(Padding::from([4, 10]))
                        .on_press(Message::CopySnipArea),
                        button(
                            row![text("💾"), text("Simpan PNG").size(11)]
                                .spacing(4)
                                .align_y(Alignment::Center)
                        )
                        .style(edge_btn)
                        .padding(Padding::from([4, 8]))
                        .on_press(Message::SaveSnipArea),
                        button(text("✕ Selesai (ESC)").size(11))
                            .style(edge_btn)
                            .padding(Padding::from([4, 8]))
                            .on_press(Message::CancelSnip)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                )
                .width(Length::Fill)
                .padding(Padding::from([6, 12]))
                .style(|_t: &Theme| container::Style {
                    background: Some(Background::Color(Color::from_rgb8(0x18, 0x24, 0x36))),
                    border: Border {
                        color: Color::from_rgb8(0x28, 0x44, 0x6e),
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                });

                column![
                    snip_bar,
                    container(main_scroll)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_t: &Theme| container::Style {
                            background: Some(Background::Color(Color::from_rgb8(0x28, 0x2a, 0x30))),
                            ..Default::default()
                        })
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            } else {
                container(main_scroll)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|_t: &Theme| container::Style {
                        background: Some(Background::Color(Color::from_rgb8(0x32, 0x34, 0x38))),
                        ..Default::default()
                    })
                    .into()
            };

            if self.context_menu_open {
                let action_btn = |icon: &'static str, title: &'static str, desc: &'static str, msg: Message| {
                    button(
                        row![
                            text(icon).size(16),
                            column![
                                text(title).size(12).font(BOLD),
                                text(desc).size(10).style(text::secondary)
                            ]
                            .spacing(2)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                    )
                    .style(edge_btn)
                    .width(Length::Fill)
                    .padding(Padding::from([8, 12]))
                    .on_press(msg)
                };

                let menu_card = container(
                    column![
                        row![
                            text(format!("Aksi Halaman {}", self.viewer_current_page)).size(12).font(BOLD).style(text::primary),
                            Space::new().width(Length::Fill),
                            button(text("✕").size(11))
                                .style(edge_btn)
                                .on_press(Message::CloseContextMenu)
                        ]
                        .align_y(Alignment::Center),
                        rule::horizontal(1),
                        action_btn("📝", "Blok & Salin Teks...", "Buka panel untuk memilih & blok teks halaman", Message::ToggleTextDrawer),
                        action_btn("✂️", "Cuplik Area (Snip Tool)...", "Pilih area kotak untuk disalin atau disimpan", Message::ToggleSnipMode),
                        action_btn("📋", "Salin Gambar ke Clipboard", "Paste langsung (Ctrl+V) ke WA/Word", Message::ViewerCopyPageToClipboard),
                        action_btn("🖼️", "Simpan Gambar (PNG)...", "Ekspor gambar beresolusi tinggi", Message::ViewerExportPagePng),
                        action_btn("📷", "Simpan Gambar (JPG)...", "Ekspor file JPG standar", Message::ViewerExportPageJpg),
                        action_btn("📄", "Simpan Halaman sebagai PDF...", "Ekstrak halaman ini ke file PDF baru", Message::ViewerExportPagePdf),
                        action_btn("📁", "Ekspor Semua Halaman...", "Simpan seluruh halaman PDF ke folder", Message::ViewerExportAll),
                    ]
                    .spacing(6)
                    .width(280)
                )
                .padding(Padding::from([10, 14]))
                .style(|_t: &Theme| container::Style {
                    background: Some(Background::Color(Color::from_rgb8(0x22, 0x22, 0x28))),
                    border: Border {
                        color: Color::from_rgb8(0x3a, 0x3a, 0x46),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    shadow: Shadow {
                        color: Color::from_rgba8(0, 0, 0, 0.5),
                        offset: Vector::new(0.0, 6.0),
                        blur_radius: 14.0,
                    },
                    ..Default::default()
                });

                let dismissible_scroll = mouse_area(canvas)
                    .on_press(Message::CloseContextMenu);

                row![
                    container(dismissible_scroll).width(Length::Fill).height(Length::Fill),
                    container(menu_card).padding(Padding::from([12, 16])).align_y(Alignment::Start)
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            } else {
                canvas
            }
        } else {
            container(
                column![
                    text("📖").size(48),
                    text("PD - Modern Edge-Style PDF Reader").size(17).font(BOLD),
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
                    .style(edge_btn_primary)
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
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0x28, 0x29, 0x2e))),
                ..Default::default()
            })
            .into()
        }
    }

    // --- PDF TO IMAGES TAB VIEW ---
    fn view_pdf_to_images_tab(&self) -> Element<'_, Message> {
        let is_processing = matches!(self.status, OperationStatus::Processing(_));

        let pick_btn = button(
            row![
                text("📂"),
                text("Pilih Dokumen PDF").size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding(Padding::from([10, 18]));

        let pick_btn = if is_processing {
            pick_btn
        } else {
            pick_btn.on_press(Message::PickPdfToImgFile)
        };

        let file_info_view: Element<'_, Message> = if let Some(ref path) = self.pdf_to_img_file {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "PDF".to_string());

            let size_text = std::fs::metadata(path)
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|_| "-".to_string());

            container(
                row![
                    text("📄").size(36),
                    column![
                        text(file_name).size(15).font(BOLD),
                        text(format!("Ukuran: {} • Path: {}", size_text, path.display()))
                            .size(11)
                            .style(text::secondary),
                        row![
                            container(
                                text(format!("{} Halaman", self.pdf_to_img_total_pages))
                                    .font(BOLD)
                                    .size(12)
                                    .style(text::primary)
                            )
                            .padding(Padding::from([3, 8]))
                            .style(container::rounded_box)
                        ]
                        .spacing(8)
                    ]
                    .spacing(4)
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
                    text("📷").size(48),
                    text("Belum ada file PDF yang dipilih").size(16).font(BOLD),
                    text("Pilih dokumen PDF yang ingin diekspor setiap halamannya menjadi file gambar (PNG / JPG).")
                        .size(13)
                        .style(text::secondary),
                    text("Mendukung ekspor seluruh halaman sekaligus atau rentang halaman tertentu dengan resolusi tajam.")
                        .size(12)
                        .style(text::secondary),
                    Space::new().height(10),
                    pick_btn
                ]
                .spacing(8)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(Padding::from([32, 20]))
            .center_x(Length::Fill)
            .style(container::rounded_box)
            .into()
        };

        // Options: Page Scope
        let all_pages_btn = button(text("✓ Semua Halaman").size(13))
            .style(if self.pdf_to_img_all_pages {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::PdfToImgToggleAllPages(true))
            .padding(Padding::from([6, 14]));

        let range_pages_btn = button(text("Halaman Tertentu").size(13))
            .style(if !self.pdf_to_img_all_pages {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::PdfToImgToggleAllPages(false))
            .padding(Padding::from([6, 14]));

        let scope_row = row![all_pages_btn, range_pages_btn]
            .spacing(8)
            .align_y(Alignment::Center);

        let mut scope_col = column![
            text("1. Cakupan Halaman").size(14).font(BOLD),
            scope_row
        ]
        .spacing(8);

        if !self.pdf_to_img_all_pages {
            let range_input = text_input(
                "Contoh: 1, 3-5, 8",
                &self.pdf_to_img_range,
            )
            .on_input(Message::PdfToImgRangeChanged)
            .padding(8)
            .size(13);

            scope_col = scope_col.push(
                column![
                    text("Masukkan nomor / rentang halaman:").size(12).style(text::secondary),
                    range_input
                ]
                .spacing(4)
            );
        }

        let scope_card = container(scope_col)
            .width(Length::FillPortion(1))
            .padding(Padding::from([14, 16]))
            .style(container::rounded_box);

        // Options: Image Format
        let png_btn = button(
            row![
                text("PNG"),
                text("Lossless").size(11).style(text::secondary)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.pdf_to_img_format == ExportImageFormat::Png {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::PdfToImgFormatChanged(ExportImageFormat::Png))
        .padding(Padding::from([6, 14]));

        let jpg_btn = button(
            row![
                text("JPG"),
                text("Kompak").size(11).style(text::secondary)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(if self.pdf_to_img_format == ExportImageFormat::Jpeg {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::PdfToImgFormatChanged(ExportImageFormat::Jpeg))
        .padding(Padding::from([6, 14]));

        let format_card = container(
            column![
                text("2. Format Gambar").size(14).font(BOLD),
                row![png_btn, jpg_btn].spacing(8).align_y(Alignment::Center),
                text("PNG ideal untuk teks tajam; JPG ideal untuk foto & ukuran hemat.")
                    .size(11)
                    .style(text::secondary)
            ]
            .spacing(8)
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([14, 16]))
        .style(container::rounded_box);

        // Options: Resolution / Scale
        let dpi_150_btn = button(text("150 DPI (1.5x)").size(12))
            .style(if (self.pdf_to_img_scale - 1.5).abs() < 0.1 {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::PdfToImgScaleChanged(1.5))
            .padding(Padding::from([6, 10]));

        let dpi_200_btn = button(text("200 DPI (HD)").size(12))
            .style(if (self.pdf_to_img_scale - 2.0).abs() < 0.1 {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::PdfToImgScaleChanged(2.0))
            .padding(Padding::from([6, 10]));

        let dpi_300_btn = button(text("300 DPI (Ultra HD)").size(12))
            .style(if (self.pdf_to_img_scale - 3.0).abs() < 0.1 {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::PdfToImgScaleChanged(3.0))
            .padding(Padding::from([6, 10]));

        let scale_card = container(
            column![
                text("3. Resolusi / Kualitas").size(14).font(BOLD),
                row![dpi_150_btn, dpi_200_btn, dpi_300_btn].spacing(6).align_y(Alignment::Center),
                text("Resolusi lebih tinggi menghasilkan gambar yang lebih jernih dan tajam.")
                    .size(11)
                    .style(text::secondary)
            ]
            .spacing(8)
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([14, 16]))
        .style(container::rounded_box);

        let options_row = row![scope_card, format_card, scale_card]
            .spacing(12)
            .width(Length::Fill);

        // Action button
        let export_action_btn = button(
            row![
                text("⚡"),
                text("Ekspor Halaman ke Gambar").size(15).font(BOLD)
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        )
        .style(button::success)
        .padding(Padding::from([12, 28]));

        let export_action_btn = if self.pdf_to_img_file.is_some() && !is_processing {
            export_action_btn.on_press(Message::RunPdfToImages)
        } else {
            export_action_btn
        };

        let action_bar = container(
            row![
                column![
                    text("Folder Penyimpanan").size(13).font(BOLD),
                    text("Anda akan diminta memilih folder tujuan saat tombol ditekan.")
                        .size(11)
                        .style(text::secondary)
                ]
                .spacing(2),
                Space::new().width(Length::Fill),
                export_action_btn
            ]
            .spacing(16)
            .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .padding(Padding::from([14, 20]))
        .style(container::rounded_box);

        let content_col = column![
            file_info_view,
            options_row,
            action_bar
        ]
        .spacing(14);

        scrollable(content_col).height(Length::Fill).into()
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

fn rotate_rgba_clockwise(width: u32, height: u32, data: &[u8], degrees: usize) -> (u32, u32, Vec<u8>) {
    match degrees % 360 {
        90 => {
            let mut out = vec![0u8; data.len()];
            for y in 0..height {
                for x in 0..width {
                    let src_idx = ((y * width + x) * 4) as usize;
                    let dest_x = height - 1 - y;
                    let dest_y = x;
                    let dest_idx = ((dest_y * height + dest_x) * 4) as usize;
                    out[dest_idx..dest_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
                }
            }
            (height, width, out)
        }
        180 => {
            let mut out = vec![0u8; data.len()];
            let total_pixels = (width * height) as usize;
            for i in 0..total_pixels {
                let src_idx = i * 4;
                let dest_idx = (total_pixels - 1 - i) * 4;
                out[dest_idx..dest_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
            }
            (width, height, out)
        }
        270 => {
            let mut out = vec![0u8; data.len()];
            for y in 0..height {
                for x in 0..width {
                    let src_idx = ((y * width + x) * 4) as usize;
                    let dest_x = y;
                    let dest_y = width - 1 - x;
                    let dest_idx = ((dest_y * width + dest_x) * 4) as usize;
                    out[dest_idx..dest_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
                }
            }
            (height, width, out)
        }
        _ => (width, height, data.to_vec()),
    }
}

fn crop_rgba(
    src_w: u32,
    src_h: u32,
    src_rgba: &[u8],
    rx: u32,
    ry: u32,
    rw: u32,
    rh: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    if rw == 0 || rh == 0 {
        return Err("Ukuran area cuplikan tidak boleh kosong".to_string());
    }
    let end_x = (rx + rw).min(src_w);
    let end_y = (ry + rh).min(src_h);
    let actual_w = end_x.saturating_sub(rx);
    let actual_h = end_y.saturating_sub(ry);
    if actual_w == 0 || actual_h == 0 {
        return Err("Area cuplikan berada di luar batas dokumen".to_string());
    }

    let mut cropped = Vec::with_capacity((actual_w * actual_h * 4) as usize);
    for y in ry..end_y {
        let row_start = ((y * src_w + rx) * 4) as usize;
        let row_end = row_start + (actual_w * 4) as usize;
        if row_end <= src_rgba.len() {
            cropped.extend_from_slice(&src_rgba[row_start..row_end]);
        } else {
            return Err("Index piksel di luar jangkauan".to_string());
        }
    }
    Ok((actual_w, actual_h, cropped))
}


