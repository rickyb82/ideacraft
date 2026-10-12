//! IdeaCraft desktop app.
//!
//! Usage: `ideacraft [--sample | --template <id>] [file]`. The file can be an IdeaCraft map or any
//! importable format (FreeMind/Freeplane `.mm`, XMind, CmapTools `.cxl`, OPML, Markdown, Mermaid).
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use ideacraft_ui_egui::core::Map;
use ideacraft_ui_egui::{IdeaApp, Request};
use serde_json::json;

struct App(IdeaApp);

impl App {
    /// File dialogs requested by the UI.
    fn serve_requests(&mut self) {
        for request in std::mem::take(&mut self.0.requests) {
            match request {
                Request::Open => {
                    let exts = ideacraft_ui_egui::formats::open_extensions();
                    if let Some(path) = rfd::FileDialog::new().add_filter("Maps and outlines", &exts).add_filter("All files", &["*"]).pick_file() {
                        self.0.open_path(&path);
                    }
                }
                Request::SaveAs => {
                    let name = format!("{}.ideacraft.json", self.0.session.map().title());
                    if let Some(path) = rfd::FileDialog::new().add_filter("IdeaCraft map", &["json"]).set_file_name(name).save_file() {
                        self.0.save_as(path);
                    }
                }
                Request::Export(format) => {
                    let name = format!("{}.{}", self.0.session.map().title(), format.extension());
                    if let Some(path) = rfd::FileDialog::new().add_filter(format.label(), &[format.extension()]).set_file_name(name).save_file() {
                        self.0.export_to(&path, format);
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        // Files dropped on the window open (or import) as maps.
        for f in std::mem::take(&mut raw.dropped_files) {
            let path = f.path().to_path_buf();
            if !path.as_os_str().is_empty() {
                self.0.open_path(&path);
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let title = self.0.window_title();
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title));
        self.0.ui(ui);
        self.serve_requests();
    }
}

const USAGE: &str = "Usage: ideacraft [--sample | --template <id>] [file]";

fn main() -> eframe::Result {
    let mut app = IdeaApp::default();
    let mut opened = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sample" => {
                app = IdeaApp::new(Map::sample());
                opened = true;
            }
            "--template" => {
                let id = args.next().unwrap_or_default();
                opened = app.run("map.newFromTemplate", json!({"template": id})).is_some();
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            path => {
                app.open_path(std::path::Path::new(path));
                opened = true;
            }
        }
    }
    // A fresh start offers the templates.
    app.gallery_open = !opened;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("IdeaCraft").with_inner_size([1280.0, 800.0]).with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native("IdeaCraft", options, Box::new(|_cc| Ok(Box::new(App(app)))))
}
