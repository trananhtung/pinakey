//! Lớp vẽ giao diện thiết lập bằng eframe/egui (thuần Rust). Lớp này mỏng: mọi logic nằm ở
//! [`crate::controller`]. Để tránh xung đột borrow của egui (immediate-mode), các tương tác ghi vào
//! biến cục bộ trong closure rồi mới áp dụng lên controller sau khi vẽ xong.

use eframe::egui;

use crate::controller::{settings_flags, SettingsController};

/// Mở cửa sổ thiết lập cho `engine_name` và chạy vòng lặp GUI tới khi người dùng đóng.
pub fn run(engine_name: &str) -> Result<(), eframe::Error> {
    let controller = SettingsController::load(engine_name);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 600.0])
            .with_title("PinaKey — Thiết lập"),
        ..Default::default()
    };
    eframe::run_native(
        "PinaKey — Thiết lập",
        options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::new(controller)) as Box<dyn eframe::App>)),
    )
}

struct SettingsApp {
    controller: SettingsController,
    status: String,
}

impl SettingsApp {
    fn new(controller: SettingsController) -> Self {
        SettingsApp {
            controller,
            status: String::new(),
        }
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Đọc trước trạng thái (read-only) để closure không mượn `self`.
        let ims = self.controller.input_methods();
        let charsets = self.controller.charsets();
        let modes = self.controller.input_modes();
        let cur_im = self.controller.input_method().to_string();
        let cur_cs = self.controller.output_charset().to_string();
        let cur_mode = self.controller.input_mode();
        let cur_mode_label = modes
            .iter()
            .find(|(m, _)| *m == cur_mode)
            .map(|(_, l)| *l)
            .unwrap_or("?")
            .to_string();
        let flag_states: Vec<(u32, &'static str, bool)> = settings_flags()
            .into_iter()
            .map(|(f, l)| (f, l, self.controller.flag_enabled(f)))
            .collect();
        let is_dirty = self.controller.is_dirty();
        let status = self.status.clone();

        // Các thay đổi do người dùng tạo, áp dụng sau khi vẽ.
        let mut set_im: Option<String> = None;
        let mut set_cs: Option<String> = None;
        let mut set_mode: Option<i32> = None;
        let mut set_flags: Vec<(u32, bool)> = Vec::new();
        let mut do_save = false;
        let mut do_reset = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PinaKey — Bộ gõ tiếng Việt");
            ui.label("Thiết lập engine");
            ui.add_space(8.0);

            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Kiểu gõ");
                    egui::ComboBox::from_id_salt("im")
                        .selected_text(cur_im.as_str())
                        .show_ui(ui, |ui| {
                            for im in &ims {
                                if ui.selectable_label(*im == cur_im, im).clicked() {
                                    set_im = Some(im.clone());
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Bảng mã");
                    egui::ComboBox::from_id_salt("cs")
                        .selected_text(cur_cs.as_str())
                        .show_ui(ui, |ui| {
                            for cs in &charsets {
                                if ui.selectable_label(*cs == cur_cs, cs).clicked() {
                                    set_cs = Some(cs.clone());
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Chế độ nhập");
                    egui::ComboBox::from_id_salt("mode")
                        .selected_text(cur_mode_label.as_str())
                        .show_ui(ui, |ui| {
                            for (m, l) in &modes {
                                if ui.selectable_label(*m == cur_mode, *l).clicked() {
                                    set_mode = Some(*m);
                                }
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Tùy chọn");
            for (flag, label, mut on) in flag_states {
                if ui.checkbox(&mut on, label).changed() {
                    set_flags.push((flag, on));
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Lưu").clicked() {
                    do_save = true;
                }
                if ui.button("Khôi phục mặc định").clicked() {
                    do_reset = true;
                }
                if is_dirty {
                    ui.colored_label(egui::Color32::from_rgb(0xd0, 0x80, 0x20), "● chưa lưu");
                }
            });
            if !status.is_empty() {
                ui.label(status);
            }
        });

        // Áp dụng thay đổi lên controller (ngoài closure -> không xung đột borrow).
        if let Some(im) = set_im {
            self.controller.set_input_method(&im);
        }
        if let Some(cs) = set_cs {
            self.controller.set_output_charset(&cs);
        }
        if let Some(m) = set_mode {
            self.controller.set_input_mode(m);
        }
        for (flag, on) in set_flags {
            self.controller.set_flag(flag, on);
        }
        if do_reset {
            self.controller.reset_to_default();
            self.status = "Đã khôi phục mặc định (chưa lưu).".to_string();
        }
        if do_save {
            self.status = match self.controller.save() {
                Ok(()) => "Đã lưu cấu hình. Khởi động lại fcitx5 (fcitx5 -r) hoặc IBus để áp dụng."
                    .to_string(),
                Err(e) => format!("Lỗi lưu: {e}"),
            };
        }
    }
}
