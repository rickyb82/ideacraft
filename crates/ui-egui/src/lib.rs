//! IdeaCraft's egui front end (layer L6). It is thin: it draws the engine's state and acts only
//! through [`IdeaApp::run`], so every action is also available to the CLI and automation.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod canvas;
mod gallery;
pub mod theme;
mod toolbar;

use ideacraft_engine::Session;
use ideacraft_engine::core::{Map, NodeId};
use serde_json::{Value, json};

pub use canvas::View;
pub use gallery::thumbnail;
pub use ideacraft_engine::{core, formats};

/// The whole app state the UI keeps on top of the engine session.
pub struct IdeaApp {
    pub session: Session,
    /// Where the map is saved (desktop only).
    pub path: Option<std::path::PathBuf>,
    pub view: View,
    canvas: canvas::CanvasState,
    /// Text being edited: node and draft.
    editing: Option<(NodeId, String)>,
    title_draft: Option<String>,
    /// Drag from node to node draws a link instead of moving.
    pub link_mode: bool,
    pub status: String,
    /// The template gallery is showing.
    pub gallery_open: bool,
    template_previews: Vec<Map>,
    /// File dialogs the host should show (the UI crate has no file-dialog dependency).
    pub requests: Vec<Request>,
}

/// Something only the host app can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Pick a file to open (IdeaCraft or any importable format).
    Open,
    /// Pick where to save the map.
    SaveAs,
    /// Pick where to export the map in another format.
    Export(ExportFormat),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Opml,
    FreeMind,
}

impl ExportFormat {
    pub const ALL: [Self; 3] = [Self::Markdown, Self::Opml, Self::FreeMind];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown outline (.md)",
            Self::Opml => "OPML (.opml)",
            Self::FreeMind => "FreeMind / Freeplane (.mm)",
        }
    }

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Opml => "opml",
            Self::FreeMind => "mm",
        }
    }

    /// The command's `format` parameter.
    pub const fn command_format(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Opml => "opml",
            Self::FreeMind => "freemind",
        }
    }
}

impl Default for IdeaApp {
    fn default() -> Self {
        Self::new(Map::default())
    }
}

impl IdeaApp {
    pub fn new(map: Map) -> Self {
        Self {
            session: Session::new(map),
            path: None,
            view: View::default(),
            canvas: canvas::CanvasState::default(),
            editing: None,
            title_draft: None,
            link_mode: false,
            status: String::new(),
            gallery_open: false,
            template_previews: Vec::new(),
            requests: Vec::new(),
        }
    }

    /// Runs an engine command; failures go to the status bar.
    pub fn run(&mut self, id: &str, params: Value) -> Option<Value> {
        match self.session.execute(id, &params) {
            Ok(v) => Some(v),
            Err(e) => {
                self.status = e.to_string();
                None
            }
        }
    }

    /// Runs a command that creates a node, then starts editing its text.
    fn run_and_edit(&mut self, id: &str, params: Value) {
        if let Some(new) = self.run(id, params).and_then(|v| v["id"].as_u64()) {
            // A new idea starts with an empty draft: typing replaces the default label.
            self.editing = Some((NodeId(new), String::new()));
        }
    }

    fn begin_edit(&mut self, id: NodeId) {
        if let Some(n) = self.session.map().node(id) {
            self.editing = Some((id, n.label.clone()));
        }
    }

    fn commit_edit(&mut self) {
        if let Some((id, text)) = self.editing.take() {
            // An empty draft keeps the current label.
            if !text.trim().is_empty() && self.session.map().node(id).is_some_and(|n| n.label != text) {
                self.run("node.setLabel", json!({"id": id.0, "label": text}));
            }
        }
    }

    /// Opens a file's bytes: an IdeaCraft map or any importable format.
    pub fn open_bytes(&mut self, name: &str, bytes: &[u8]) -> bool {
        match ideacraft_engine::formats::import_bytes(name, bytes) {
            Ok((map, format)) => {
                self.session.replace_map(map);
                self.view.fit_pending = true;
                self.editing = None;
                self.status = if format == ideacraft_engine::formats::Format::IdeaCraft {
                    format!("Opened {name}")
                } else {
                    format!("Imported {name} ({})", format.name())
                };
                true
            }
            Err(e) => {
                self.status = format!("Couldn't open {name}: {e}");
                false
            }
        }
    }

    /// Opens a map or imports another format (desktop). Imported files save as a new
    /// `.ideacraft.json` next to the original, never over it.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_path(&mut self, path: &std::path::Path) {
        let name = path.display().to_string();
        let bytes = match std::fs::metadata(path).map(|m| m.len()) {
            Ok(len) if len > ideacraft_engine::formats::MAX_FILE_BYTES as u64 => {
                self.status = format!("Couldn't open {name}: file too large");
                return;
            }
            _ => std::fs::read(path),
        };
        match bytes {
            Ok(bytes) if self.open_bytes(&name, &bytes) => {
                let native = name.to_ascii_lowercase().ends_with(".ideacraft.json");
                self.path = Some(if native { path.to_path_buf() } else { path.with_extension("ideacraft.json") });
                if !native {
                    self.session.mark_dirty();
                }
            }
            Ok(_) => {}
            Err(e) => self.status = format!("Couldn't open {name}: {e}"),
        }
    }

    /// Writes the map in another format (desktop).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_to(&mut self, path: &std::path::Path, format: ExportFormat) {
        let text = self.run("map.export", json!({"format": format.command_format()})).and_then(|v| v["text"].as_str().map(str::to_string));
        if let Some(text) = text {
            self.status = match std::fs::write(path, text) {
                Ok(()) => format!("Exported {}", path.display()),
                Err(e) => format!("Couldn't export {}: {e}", path.display()),
            };
        }
    }

    /// Saves to a chosen path (desktop).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as(&mut self, path: std::path::PathBuf) {
        self.path = Some(path);
        self.save();
    }

    /// Saves to [`Self::path`], or to `<title>.ideacraft.json` in the current folder.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&mut self) {
        self.commit_edit();
        let path = self.path.clone().unwrap_or_else(|| {
            let stem: String =
                self.session.map().title().chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).take(80).collect();
            std::path::PathBuf::from(format!("{}.ideacraft.json", if stem.is_empty() { "map".into() } else { stem }))
        });
        match std::fs::write(&path, self.session.map().to_json()) {
            Ok(()) => {
                self.session.mark_saved();
                self.status = format!("Saved {}", path.display());
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Couldn't save {}: {e}", path.display()),
        }
    }

    /// Window title: map title, with a dot while unsaved.
    pub fn window_title(&self) -> String {
        format!("{}{} — IdeaCraft", if self.session.is_dirty() { "• " } else { "" }, self.session.map().title())
    }

    /// Draws one frame.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let tokens = theme::Tokens::for_dark_mode(ui.visuals().dark_mode);
        canvas::keyboard(self, ui);
        toolbar::top(self, ui);
        toolbar::status(self, ui, &tokens);
        gallery::show(self, &ui.ctx().clone(), &tokens);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(tokens.canvas)).show(ui, |ui| canvas::show(self, ui, &tokens));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(app: &mut IdeaApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui));
        out.textures_delta.clear();
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    #[test]
    fn renders_the_sample_and_survives_keyboard_editing() {
        let ctx = egui::Context::default();
        let mut app = IdeaApp::new(Map::sample());
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let n = app.session.map().node_count();
        let root = app.session.map().roots()[0];
        app.run("selection.set", json!({"ids": [root.0]}));
        frame(&mut app, &ctx, vec![key(egui::Key::Tab, egui::Modifiers::NONE)]);
        assert_eq!(app.session.map().node_count(), n + 1, "Tab adds a child");
        assert!(app.editing.is_some(), "and starts editing it");
        frame(&mut app, &ctx, vec![egui::Event::Text("Nuova idea".into())]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.editing.is_none());
        let sel = app.session.selection()[0];
        assert_eq!(app.session.map().node(sel).unwrap().label, "Nuova idea");
        frame(&mut app, &ctx, vec![key(egui::Key::Delete, egui::Modifiers::NONE)]);
        assert_eq!(app.session.map().node_count(), n);
        frame(&mut app, &ctx, vec![key(egui::Key::Z, egui::Modifiers::COMMAND)]);
        assert_eq!(app.session.map().node_count(), n + 1, "Cmd+Z undoes the delete");
        for _ in 0..2 {
            frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::Zoom(1.5),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(10.0, -30.0),
                        modifiers: egui::Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ],
            );
        }
    }

    #[test]
    fn template_gallery_renders_and_open_bytes_imports() {
        let ctx = egui::Context::default();
        let mut app = IdeaApp { gallery_open: true, ..IdeaApp::default() };
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.template_previews.len(), ideacraft_engine::formats::templates::ALL.len());
        assert!(app.open_bytes("trip.opml", b"<opml><body><outline text='Trip'><outline text='Pack'/></outline></body></opml>"));
        assert_eq!(app.session.map().node_count(), 2);
        assert!(!app.open_bytes("bad.mm", b"<nope"));
        assert_eq!(app.session.map().node_count(), 2, "a failed import keeps the map");
    }

    #[test]
    fn renders_an_empty_map_in_dark_mode() {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let mut app = IdeaApp::default();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Tab, egui::Modifiers::NONE), key(egui::Key::Space, egui::Modifiers::NONE)]);
        assert_eq!(app.session.map().node_count(), 0, "nothing selected, nothing added");
    }
}
