//! The left rail and its panels: Templates, Elements, Layout and Files.

use egui::{Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};
use ideacraft_engine::core::{Rgb, Shape};
use ideacraft_engine::formats::templates;
use serde_json::json;

use crate::gallery::thumbnail;
use crate::icons::{self, Icon};
use crate::theme::Tokens;
use crate::widgets::{icon_button, icon_text_button, primary_button, rail_item};
use crate::{ExportFormat, IdeaApp, Request, SidePanel};

const RAIL_W: f32 = 76.0;
const PANEL_W: f32 = 340.0;

pub fn show(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    let rail = egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(4, 10));
    egui::Panel::left("rail").exact_size(RAIL_W).resizable(false).frame(rail).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            for (panel, icon, label) in [
                (SidePanel::Templates, Icon::Templates, "Templates"),
                (SidePanel::Elements, Icon::Elements, "Elements"),
                (SidePanel::Layout, Icon::Layout, "Layout"),
                (SidePanel::Files, Icon::Files, "Files"),
            ] {
                let active = app.sidebar == Some(panel);
                if rail_item(ui, t, icon, app.tr(label), active).clicked() {
                    app.sidebar = if active { None } else { Some(panel) };
                }
            }
        });
    });
    let Some(panel) = app.sidebar else { return };
    let frame = egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(16, 14));
    egui::Panel::left("side_panel").exact_size(PANEL_W).resizable(false).frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            let title = match panel {
                SidePanel::Templates => "Templates",
                SidePanel::Elements => "Elements",
                SidePanel::Layout => "Layout",
                SidePanel::Files => "Files",
            };
            ui.label(RichText::new(app.tr(title)).heading().strong().color(t.ui_text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, t, Icon::Close, app.tr("Close"), true, false).clicked() {
                    app.sidebar = None;
                }
            });
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match panel {
            SidePanel::Templates => templates_panel(app, ui, t),
            SidePanel::Elements => elements_panel(app, ui, t),
            SidePanel::Layout => layout_panel(app, ui, t),
            SidePanel::Files => files_panel(app, ui, t),
        });
    });
}

fn section(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(text).small().strong().color(t.muted));
    ui.add_space(2.0);
}

fn templates_panel(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    let lang = app.language.templates();
    if app.template_previews.len() != templates::ALL.len() || app.previews_language != app.language {
        app.template_previews = templates::ALL.iter().map(|tpl| tpl.build_in(lang)).collect();
        app.previews_language = app.language;
    }
    ui.label(RichText::new(app.tr("Pick a starting point. Everything stays editable.")).color(t.muted));
    if let Some(id) = app.confirm_template {
        ui.add_space(8.0);
        egui::Frame::NONE.fill(t.active).corner_radius(10).inner_margin(10).show(ui, |ui| {
            ui.label(RichText::new(app.tr("Replace the current map? Unsaved changes will be lost.")).color(t.ui_text));
            ui.horizontal(|ui| {
                if primary_button(ui, t, None, app.tr("Replace")).clicked() {
                    app.apply_template(id);
                    app.confirm_template = None;
                }
                if ui.button(app.tr("Cancel")).clicked() {
                    app.confirm_template = None;
                }
            });
        });
    }
    ui.add_space(8.0);
    let card = vec2((ui.available_width() - 10.0) / 2.0, 124.0);
    egui::Grid::new("templates").spacing(vec2(10.0, 10.0)).show(ui, |ui| {
        for (i, tpl) in templates::ALL.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(card, Sense::click());
            let hover = resp.hovered();
            let p = ui.painter();
            p.rect_filled(rect.translate(vec2(0.0, 2.0)), 10.0, Color32::from_black_alpha(if t.dark { 40 } else { 12 }));
            p.rect_filled(rect, 10.0, t.card);
            p.rect_stroke(rect, 10.0, Stroke::new(if hover { 2.0 } else { 1.0 }, if hover { t.accent } else { t.border }), StrokeKind::Inside);
            let thumb = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(card.x - 12.0, 78.0));
            if let Some(map) = app.template_previews.get(i) {
                thumbnail(p, map, thumb, t);
            }
            let text = p.with_clip_rect(rect.shrink(4.0));
            text.text(pos2(rect.left() + 10.0, thumb.bottom() + 8.0), egui::Align2::LEFT_TOP, tpl.name(lang), FontId::proportional(13.0), t.ui_text);
            text.text(
                pos2(rect.left() + 10.0, thumb.bottom() + 25.0),
                egui::Align2::LEFT_TOP,
                tpl.category(lang),
                FontId::proportional(11.0),
                t.muted,
            );
            if resp.on_hover_text(tpl.description(lang)).clicked() {
                let map = app.session.map();
                if app.session.is_dirty() && map.node_count() > 0 {
                    app.confirm_template = Some(tpl.id);
                } else {
                    app.apply_template(tpl.id);
                }
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}

fn elements_panel(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    let has_sel = !app.session.selection().is_empty();
    section(ui, t, app.tr("Add"));
    if icon_text_button(ui, t, Icon::Plus, app.tr("New idea"), true).clicked() {
        app.run_and_edit("node.add", json!({}));
    }
    if icon_text_button(ui, t, Icon::Child, app.tr("Child idea"), has_sel).on_hover_text(app.tip("node.addChild")).clicked() {
        app.run_and_edit("node.addChild", json!({}));
    }
    if icon_text_button(ui, t, Icon::Sibling, app.tr("Same level"), has_sel).on_hover_text(app.tip("node.addSibling")).clicked() {
        app.run_and_edit("node.addSibling", json!({}));
    }
    let link = icon_text_button(ui, t, Icon::Link, app.tr("Link ideas"), true);
    if app.link_mode {
        ui.painter().rect_stroke(link.rect, 8.0, Stroke::new(1.5, t.accent), StrokeKind::Inside);
    }
    if link.on_hover_text(app.tr("Drag from one idea to another to link them (or hold Shift while dragging)")).clicked() {
        app.link_mode = !app.link_mode;
    }
    if !has_sel {
        ui.add_space(8.0);
        ui.label(RichText::new(app.tr("Select an idea to change its shape and colour.")).small().color(t.muted));
    }
    section(ui, t, app.tr("Shape"));
    let current = app.session.selection().first().and_then(|id| app.session.map().node(*id)).map(|n| n.shape);
    ui.horizontal_wrapped(|ui| {
        for shape in Shape::ALL {
            let (rect, resp) = ui.allocate_exact_size(vec2(62.0, 46.0), if has_sel { Sense::click() } else { Sense::hover() });
            let p = ui.painter();
            let on = current == Some(shape);
            p.rect_filled(
                rect,
                8.0,
                if on {
                    t.active
                } else if resp.hovered() && has_sel {
                    t.hover
                } else {
                    t.card
                },
            );
            p.rect_stroke(rect, 8.0, Stroke::new(1.0, if on { t.accent } else { t.border }), StrokeKind::Inside);
            let r = Rect::from_center_size(rect.center(), vec2(40.0, 22.0));
            let fill = Color32::from_rgb(0xb3, 0xd4, 0xff);
            match shape {
                Shape::Ellipse => {
                    p.add(egui::Shape::ellipse_filled(r.center(), r.size() / 2.0, fill));
                }
                Shape::Rect => {
                    p.rect_filled(r, 0.0, fill);
                }
                Shape::Pill => {
                    p.rect_filled(r, 11.0, fill);
                }
                Shape::Rounded => {
                    p.rect_filled(r, 5.0, fill);
                }
            }
            if resp.on_hover_text(app.tr(shape.name())).clicked() {
                app.run("node.setShape", json!({"shape": shape.name()}));
            }
        }
    });
    section(ui, t, app.tr("Colour"));
    swatches(app, ui, t, has_sel, 30.0);
}

/// The fill palette; clicking sets the selection's colour.
pub fn swatches(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens, enabled: bool, size: f32) {
    let current = app.session.selection().first().and_then(|id| app.session.map().node(*id)).map(|n| n.color);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for color in Rgb::PALETTE {
            let (rect, resp) = ui.allocate_exact_size(vec2(size, size), if enabled { Sense::click() } else { Sense::hover() });
            let p = ui.painter();
            p.circle_filled(rect.center(), size / 2.0 - 1.0, Color32::from_rgb(color.0, color.1, color.2));
            let on = current == Some(color);
            let stroke = if on {
                Stroke::new(2.5, t.accent)
            } else if resp.hovered() && enabled {
                Stroke::new(1.5, t.ui_text)
            } else {
                Stroke::new(1.0, t.border)
            };
            p.circle_stroke(rect.center(), size / 2.0 - 1.0, stroke);
            if resp.on_hover_text(color.hex()).clicked() {
                app.run("node.setColor", json!({"color": color.hex()}));
            }
        }
    });
}

fn big_choice(ui: &mut egui::Ui, t: &Tokens, icon: Icon, title: &str, text: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, 10.0, if resp.hovered() { t.hover } else { t.card });
    p.rect_stroke(rect, 10.0, Stroke::new(1.0, if resp.hovered() { t.accent } else { t.border }), StrokeKind::Inside);
    let tile = Rect::from_min_size(rect.min + vec2(10.0, 10.0), vec2(44.0, 44.0));
    p.rect_filled(tile, 8.0, t.active);
    icons::paint(p, tile.shrink(10.0), icon, t.accent);
    p.text(pos2(tile.right() + 12.0, rect.top() + 14.0), egui::Align2::LEFT_TOP, title, FontId::proportional(14.0), t.ui_text);
    p.text(pos2(tile.right() + 12.0, rect.top() + 34.0), egui::Align2::LEFT_TOP, text, FontId::proportional(11.5), t.muted);
    resp
}

fn layout_panel(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    section(ui, t, app.tr("Arrange the whole map"));
    if big_choice(ui, t, Icon::Tree, app.tr("Tree"), app.tr("Branches from left to right")).on_hover_text(app.tip("layout.tree")).clicked() {
        app.run("layout.tree", json!({}));
        app.view.fit_pending = true;
    }
    ui.add_space(6.0);
    if big_choice(ui, t, Icon::Radial, app.tr("Radial"), app.tr("Ideas in a circle around the centre"))
        .on_hover_text(app.tip("layout.radial"))
        .clicked()
    {
        app.run("layout.radial", json!({}));
        app.view.fit_pending = true;
    }
    section(ui, t, app.tr("View"));
    if icon_text_button(ui, t, Icon::Fit, app.tr("Fit to window"), true).clicked() {
        app.view.fit_pending = true;
    }
    let has_sel = !app.session.selection().is_empty();
    if icon_text_button(ui, t, Icon::Collapse, app.tr("Collapse or expand branch"), has_sel).on_hover_text(app.tip("node.toggleCollapse")).clicked() {
        app.run("node.toggleCollapse", json!({}));
    }
}

fn files_panel(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    section(ui, t, app.tr("Map"));
    if icon_text_button(ui, t, Icon::Plus, app.tr("New"), true).clicked() {
        app.new_map();
    }
    if icon_text_button(ui, t, Icon::Upload, app.tr("Open or import…"), true).clicked() {
        app.requests.push(Request::Open);
    }
    if icon_text_button(ui, t, Icon::Save, app.tr("Save"), true).clicked() {
        app.save_or_ask();
    }
    if icon_text_button(ui, t, Icon::Doc, app.tr("Save as…"), true).clicked() {
        app.requests.push(Request::SaveAs);
    }
    section(ui, t, app.tr("Export as"));
    for f in ExportFormat::ALL {
        if icon_text_button(ui, t, Icon::Download, app.tr(f.label()), true).clicked() {
            app.requests.push(Request::Export(f));
        }
    }
    section(ui, t, app.tr("Opens and imports"));
    for f in ideacraft_engine::formats::Format::ALL {
        let exts: Vec<String> = f.extensions().iter().map(|e| format!(".{e}")).collect();
        ui.label(RichText::new(format!("{} · {}", app.tr(f.name()), exts.join(" "))).small().color(t.muted));
    }
}
