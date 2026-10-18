use std::path::PathBuf;
use iced::widget::{
    button, column, container, progress_bar, row, rule, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Padding, Task, Theme};

use crate::pdf_ops::{compress_pdf, extract_pages_to_file, get_pdf_page_count, merge_pdfs, CompressionReport};

const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
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
            active_tab: ActiveTab::Merge,
            status: OperationStatus::Idle,
            merge_files: Vec::new(),
            split_file: None,
            split_total_pages: 0,
            split_range_input: String::new(),
            compress_file: None,
            compress_report: None,
        }
    }


    pub fn theme(&self) -> Theme {
        Theme::TokyoNight
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
            text("PDF TOOLBOX")
                .size(11)
                .style(text::primary)
        )
        .padding(Padding::from([4, 10]))
        .style(container::rounded_box);

        let title_col = column![
            row![
                text("PD").size(26).font(BOLD),
                text(" - Desktop PDF Utility").size(20),
                Space::new().width(12),
                app_badge
            ]
            .align_y(Alignment::Center),
            text("Aplikasi desktop cepat, aman, dan offline-first untuk manipulasi dokumen PDF")
                .size(13)
                .style(text::secondary)
        ]
        .spacing(4);

        container(title_col)
            .width(Length::Fill)
            .padding(Padding::from([16, 20]))
            .into()
    }

    fn view_tabs(&self) -> Element<'_, Message> {
        let merge_btn = button(
            row![
                text("📑"),
                text("Merge PDF").size(14).font(BOLD)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::Merge {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::Merge))
        .padding(Padding::from([10, 18]));

        let split_btn = button(
            row![
                text("✂️"),
                text("Split PDF").size(14).font(BOLD)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::Split {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::Split))
        .padding(Padding::from([10, 18]));

        let compress_btn = button(
            row![
                text("🗜️"),
                text("Compress PDF").size(14).font(BOLD)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .style(if self.active_tab == ActiveTab::Compress {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SwitchTab(ActiveTab::Compress))
        .padding(Padding::from([10, 18]));

        let tabs_row = row![merge_btn, split_btn, compress_btn]
            .spacing(12)
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
                        text("⏳").size(16),
                        text(msg).size(14).font(BOLD)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    bar
                ]
                .spacing(8);

                container(content)
                    .width(Length::Fill)
                    .padding(Padding::from([10, 16]))
                    .style(container::rounded_box)
                    .into()
            }
            OperationStatus::Success(msg) => {
                let dismiss_btn = button(text("✕").size(12))
                    .style(button::text)
                    .on_press(Message::DismissStatus);

                let content = row![
                    text("✅").size(16),
                    text(msg).size(14).style(text::success),
                    Space::new().width(Length::Fill),
                    dismiss_btn
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                container(content)
                    .width(Length::Fill)
                    .padding(Padding::from([10, 16]))
                    .style(container::rounded_box)
                    .into()
            }
            OperationStatus::Error(msg) => {
                let dismiss_btn = button(text("✕").size(12))
                    .style(button::text)
                    .on_press(Message::DismissStatus);

                let content = row![
                    text("⚠️").size(16),
                    text(msg).size(14).style(text::danger),
                    Space::new().width(Length::Fill),
                    dismiss_btn
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                container(content)
                    .width(Length::Fill)
                    .padding(Padding::from([10, 16]))
                    .style(container::rounded_box)
                    .into()
            }
        }
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
