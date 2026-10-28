mod app;
mod audio;
mod pitch;

use app::TunerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 560.0])
            .with_min_inner_size([620.0, 480.0])
            .with_title("Polar Tuner"),
        ..Default::default()
    };
    eframe::run_native(
        "Polar Tuner",
        options,
        Box::new(|cc| Ok(Box::new(TunerApp::new(cc)))),
    )
}
