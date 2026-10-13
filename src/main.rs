#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod audio;
mod engine;
mod output;
mod speech;
mod store;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--transcribe-test") {
        let result = engine::transcribe_test(std::path::Path::new(
            args.get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing WAV path"))?,
        ));
        let output = args
            .get(3)
            .ok_or_else(|| anyhow::anyhow!("Missing output path"))?;
        std::fs::write(
            output,
            match &result {
                Ok(text) => text.clone(),
                Err(e) => format!("ERROR: {e:#}"),
            },
        )?;
        return result.map(|_| ());
    }
    let preview = args.get(1).map(String::as_str) == Some("--preview");
    let root = if preview {
        std::path::PathBuf::from(
            args.get(2)
                .ok_or_else(|| anyhow::anyhow!("Preview directory required"))?,
        )
        .join("preview-data")
    } else {
        std::path::PathBuf::from(
            std::env::var_os("LOCALAPPDATA")
                .ok_or_else(|| anyhow::anyhow!("Windows local app data folder unavailable"))?,
        )
        .join("Whispering")
    };
    let store = match store::Store::open(root) {
        Ok(store) => store,
        Err(e) => {
            speech::error_dialog(&format!("{e:#}"));
            return Err(e);
        }
    };
    let icon = image::load_from_memory(include_bytes!("../assets/whispering.png"))?.into_rgba8();
    let (w, h) = icon.dimensions();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Whispering")
            .with_inner_size([1060.0, 720.0])
            .with_min_inner_size([800.0, 570.0])
            .with_icon(eframe::egui::IconData {
                rgba: icon.into_raw(),
                width: w,
                height: h,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Whispering",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, store)))),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
