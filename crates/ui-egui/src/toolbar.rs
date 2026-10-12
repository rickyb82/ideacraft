//! Top toolbar and bottom status bar.

use egui::{Color32, RichText, Stroke, StrokeKind, vec2};
use ideacraft_engine::core::{Rgb, Shape};
use serde_json::json;

use crate::IdeaApp;
use crate::theme::Tokens;

fn tip(app: &IdeaApp, info: &str) -> String {
    match ideacraft_engine::command(info) {
        Some(c) if !c.shortcut.is_empty() => format!("{} ({})", app.tr(c.label), c.shortcut),
        Some(c) => app.tr(c.label).to_string(),
        None => info.to_string(),
    }
}

fn button(app: &mut IdeaApp, ui: &mut egui::Ui, text: &str, command: &str, enabled: bool) {
    let tip = tip(app, command);
    if ui.add_enabled(enabled, egui::Button::new(app.tr(text))).on_hover_text(tip).clicked() {
        app.commit_edit();
        if command == "map.new" {
            app.new_map();
        } else if matches!(command, "node.addChild" | "node.addSibling" | "node.add") {
            app.run_and_edit(command, json!({}));
        } else {
            app.run(command, json!({}));
        }
    }
}

pub fn top(app: &mut IdeaApp, ui: &mut egui::Ui) {
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("IdeaCraft").strong());
            let mut title = app.title_draft.clone().unwrap_or_else(|| app.session.map().title().to_string());
            let edit = ui.add(egui::TextEdit::singleline(&mut title).desired_width(180.0).hint_text(app.tr("Map title")));
            if edit.changed() {
                app.title_draft = Some(title.clone());
            }
            if edit.lost_focus()
                && let Some(t) = app.title_draft.take().filter(|t| t != app.session.map().title())
            {
                app.run("map.setTitle", json!({"title": t}));
            }
            ui.separator();
            button(app, ui, "New", "map.new", true);
            if ui.button(app.tr("Templates…")).on_hover_text(app.tr("New map from a template")).clicked() {
                app.gallery_open = true;
            }
            if ui
                .button(app.tr("Open…"))
                .on_hover_text(app.tr("Open a map or import FreeMind, Freeplane, XMind, CmapTools, OPML, Markdown or Mermaid (Cmd+O)"))
                .clicked()
            {
                app.requests.push(crate::Request::Open);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button(app.tr("Save")).on_hover_text(app.tr("Save (Cmd+S)")).clicked() {
                if app.path.is_some() {
                    app.save();
                } else {
                    app.requests.push(crate::Request::SaveAs);
                }
            }
            ui.menu_button(app.tr("Export"), |ui| {
                for f in crate::ExportFormat::ALL {
                    if ui.button(app.tr(f.label())).clicked() {
                        app.requests.push(crate::Request::Export(f));
                        ui.close();
                    }
                }
            });
            ui.separator();
            let has_sel = !app.session.selection().is_empty();
            button(app, ui, "+ Idea", "node.add", true);
            button(app, ui, "+ Child", "node.addChild", has_sel);
            button(app, ui, "+ Sibling", "node.addSibling", has_sel);
            button(app, ui, "Delete", "node.delete", has_sel);
            button(app, ui, "Collapse", "node.toggleCollapse", has_sel);
            {
                let l = app.language;
                ui.toggle_value(&mut app.link_mode, l.tr("Link"))
                    .on_hover_text(l.tr("Drag from one idea to another to link them (or hold Shift while dragging)"));
            }
            ui.separator();
            button(app, ui, "Tree", "layout.tree", true);
            button(app, ui, "Radial", "layout.radial", true);
            if ui.button(app.tr("Fit")).on_hover_text(app.tr("Fit the map in the window (Cmd+0)")).clicked() {
                app.view.fit_pending = true;
            }
            ui.separator();
            let (can_undo, can_redo) = (app.session.can_undo(), app.session.can_redo());
            button(app, ui, "Undo", "edit.undo", can_undo);
            button(app, ui, "Redo", "edit.redo", can_redo);
        });
        ui.horizontal(|ui| {
            let has_sel = !app.session.selection().is_empty();
            ui.label(RichText::new(app.tr("Fill")).small());
            ui.add_enabled_ui(has_sel, |ui| {
                for color in Rgb::PALETTE {
                    let fill = Color32::from_rgb(color.0, color.1, color.2);
                    let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 4.0, fill);
                    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, ui.visuals().widgets.inactive.fg_stroke.color), StrokeKind::Inside);
                    if resp.on_hover_text(color.hex()).clicked() {
                        app.run("node.setColor", json!({"color": color.hex()}));
                    }
                }
                let current = app.session.selection().first().and_then(|id| app.session.map().node(*id)).map(|n| n.shape).unwrap_or_default();
                egui::ComboBox::from_id_salt("shape").selected_text(app.tr(current.name())).width(80.0).show_ui(ui, |ui| {
                    for shape in Shape::ALL {
                        if ui.selectable_label(shape == current, app.tr(shape.name())).clicked() {
                            app.run("node.setShape", json!({"shape": shape.name()}));
                        }
                    }
                });
            });
            ui.separator();
            ui.label(RichText::new(app.tr("Language")).small());
            let current = app.language;
            egui::ComboBox::from_id_salt("language").selected_text(current.name()).width(90.0).show_ui(ui, |ui| {
                for l in crate::Language::ALL {
                    if ui.selectable_label(l == current, l.name()).clicked() {
                        app.run("app.language.set", json!({"language": l.code()}));
                    }
                }
            });
        });
        ui.add_space(4.0);
    });
}

pub fn status(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::bottom("status").show(ui, |ui| {
        ui.horizontal(|ui| {
            let map = app.session.map();
            let counts = app.language.fill(
                "{ideas} ideas · {links} links · {selected} selected",
                &[
                    ("ideas", &map.node_count().to_string()),
                    ("links", &map.edge_count().to_string()),
                    ("selected", &app.session.selection().len().to_string()),
                ],
            );
            ui.label(RichText::new(counts).color(t.muted));
            ui.separator();
            ui.label(RichText::new(format!("{:.0}%", app.view.zoom * 100.0)).color(t.muted));
            ui.separator();
            let hint = app.tr("Double-click: new idea / edit · Tab: child · Enter: sibling · Shift+drag: link · Space: collapse");
            ui.label(RichText::new(if app.status.is_empty() { hint } else { &app.status }).color(t.muted));
        });
    });
}
