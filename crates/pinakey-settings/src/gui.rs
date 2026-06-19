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
            .with_inner_size([560.0, 660.0])
            .with_min_inner_size([460.0, 560.0])
            .with_title("PinaKey — Thiết lập"),
        ..Default::default()
    };
    eframe::run_native(
        "PinaKey — Thiết lập",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            configure_style(&cc.egui_ctx);
            Ok(Box::new(SettingsApp::new(controller)) as Box<dyn eframe::App>)
        }),
    )
}

/// Nạp một font phủ tiếng Việt làm font chính, để các chữ có dấu chồng (ộ, ế, ệ…) không bị hiện
/// thành ô vuông. Nếu không tìm thấy font hệ thống nào thì giữ font mặc định của egui.
fn configure_fonts(ctx: &egui::Context) {
    let Some(path) = crate::fonts::find_vietnamese_font() else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("vn".to_owned(), egui::FontData::from_owned(bytes));
    // Đặt font tiếng Việt lên đầu cả hai họ để nó được ưu tiên khi vẽ glyph.
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "vn".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("vn".to_owned());
    ctx.set_fonts(fonts);
}

/// Tinh chỉnh typography và khoảng cách cho dễ đọc hơn mặc định (không phụ thuộc sáng/tối).
fn configure_style(ctx: &egui::Context) {
    use egui::{FontFamily::Proportional, FontId, TextStyle};
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(24.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(14.0, egui::FontFamily::Monospace),
        ),
        (TextStyle::Small, FontId::new(12.5, Proportional)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(10.0, 9.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.interact_size.y = 28.0;
    ctx.set_style(style);
}

/// Bảng màu theo chế độ sáng/tối, kèm bo góc mềm cho widget.
fn themed_visuals(dark: bool) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let rounding = egui::Rounding::same(6.0);
    v.widgets.noninteractive.rounding = rounding;
    v.widgets.inactive.rounding = rounding;
    v.widgets.hovered.rounding = rounding;
    v.widgets.active.rounding = rounding;
    v.widgets.open.rounding = rounding;
    v.window_rounding = egui::Rounding::same(8.0);
    v
}

struct SettingsApp {
    controller: SettingsController,
    status: String,
    /// Chế độ tối; mặc định là chế độ **sáng** (light mode).
    dark_mode: bool,
}

impl SettingsApp {
    fn new(controller: SettingsController) -> Self {
        SettingsApp {
            controller,
            status: String::new(),
            dark_mode: false,
        }
    }
}

/// Vẽ một "thẻ" (card) có tiêu đề và viền bo tròn.
fn card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title).strong().size(16.0));
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
    ui.add_space(6.0);
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
        let mut toggle_theme = false;
        let dark_mode = self.dark_mode;

        // Áp dụng bảng màu sáng/tối cho khung hình này.
        ctx.set_visuals(themed_visuals(dark_mode));

        let panel = egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin::same(20.0)));
        panel.show(ctx, |ui| {
            // Tiêu đề + nút chuyển sáng/tối.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("PinaKey");
                    ui.label(
                        egui::RichText::new("Bộ gõ tiếng Việt — Thiết lập")
                            .weak()
                            .size(14.0),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if dark_mode {
                        "Nền sáng"
                    } else {
                        "Nền tối"
                    };
                    if ui.button(label).clicked() {
                        toggle_theme = true;
                    }
                });
            });
            ui.add_space(10.0);

            card(ui, "Cấu hình", |ui| {
                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([16.0, 12.0])
                    .min_col_width(110.0)
                    .show(ui, |ui| {
                        ui.label("Kiểu gõ");
                        egui::ComboBox::from_id_salt("im")
                            .width(240.0)
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
                            .width(240.0)
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
                            .width(240.0)
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
            });

            card(ui, "Tùy chọn", |ui| {
                for (flag, label, mut on) in flag_states {
                    if ui.checkbox(&mut on, label).changed() {
                        set_flags.push((flag, on));
                    }
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let save = egui::Button::new(
                    egui::RichText::new("Lưu")
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(0x2f, 0x80, 0x46));
                if ui.add(save).clicked() {
                    do_save = true;
                }
                if ui.button("Khôi phục mặc định").clicked() {
                    do_reset = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_dirty {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xd0, 0x90, 0x20),
                            "● có thay đổi chưa lưu",
                        );
                    }
                });
            });
            if !status.is_empty() {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(status).italics());
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
        if toggle_theme {
            self.dark_mode = !self.dark_mode;
        }
        if do_reset {
            self.controller.reset_to_default();
            self.status = "Đã khôi phục mặc định (chưa lưu).".to_string();
        }
        if do_save {
            self.status = match self.controller.save() {
                Ok(()) => "Đã lưu cấu hình. Khởi động lại IBus để áp dụng.".to_string(),
                Err(e) => format!("Lỗi lưu: {e}"),
            };
        }
    }
}
