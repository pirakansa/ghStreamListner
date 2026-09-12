use eframe::egui;
use ghtl::{app::fonts, catalog::CatalogApp};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "ghTimeLine Demo",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 760.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            fonts::install_fonts(&cc.egui_ctx);
            Ok(Box::new(CatalogApp::new()?))
        }),
    )
}
