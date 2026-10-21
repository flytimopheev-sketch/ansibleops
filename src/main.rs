mod ansible;
mod app;
mod models;
mod theme;
mod ui_ops;
mod ui_tabs;

use app::AnsibleApp;

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([1024.0, 700.0])
            .with_title(format!("{} v{}", theme::APP_NAME, theme::APP_VERSION)),
        ..Default::default()
    };

    eframe::run_native(
        theme::APP_NAME,
        options,
        Box::new(|cc| {
            theme::apply_theme(&cc.egui_ctx, 0);
            let mut fonts = egui::FontDefinitions::default();
            let candidates = [
                "C:\\Windows\\Fonts\\seguiemj.ttf",
                "/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf",
                "/usr/share/fonts/noto-cjk/NotoColorEmoji.ttf",
                "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
            ];
            for path in candidates {
                if let Ok(data) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("emoji".to_owned(), egui::FontData::from_owned(data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .push("emoji".to_owned());
                    break;
                }
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(AnsibleApp::new(cc)))
        }),
    )
}
