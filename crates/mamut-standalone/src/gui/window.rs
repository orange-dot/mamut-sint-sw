use super::*;

pub(crate) fn run_performance_window(session: RuntimeSession) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EPM1 Performance Rig")
            .with_inner_size([1140.0, 720.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EPM1 Performance Rig",
        options,
        Box::new(move |cc| {
            apply_epm_gui_style(&cc.egui_ctx);
            Ok(Box::new(PerformanceApp::new(session)))
        }),
    )
    .map_err(|error| anyhow!("failed to launch performance window: {error}"))
}
