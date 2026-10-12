//! The template gallery: a card per template with a drawn preview.

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use ideacraft_engine::core::Map;
use ideacraft_engine::formats::templates;
use serde_json::json;

use crate::IdeaApp;
use crate::theme::Tokens;

const CARD: Vec2 = vec2(230.0, 190.0);
const THUMB_H: f32 = 120.0;

/// Draws a miniature of `map` inside `rect`.
pub fn thumbnail(painter: &egui::Painter, map: &Map, rect: Rect, t: &Tokens) {
    painter.rect_filled(rect, 6.0, t.canvas);
    let nodes: Vec<_> = map.nodes().filter(|n| map.is_visible(n.id)).collect();
    if nodes.is_empty() {
        return;
    }
    let (mut lo, mut hi) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
    for n in &nodes {
        lo = lo.min(pos2(n.pos.x, n.pos.y));
        hi = hi.max(pos2(n.pos.x, n.pos.y));
    }
    let bounds = Rect::from_min_max(lo, hi).expand2(vec2(80.0, 30.0));
    let inner = rect.shrink(8.0);
    let scale = (inner.width() / bounds.width()).min(inner.height() / bounds.height());
    if !scale.is_finite() {
        return;
    }
    let at = |x: f32, y: f32| -> Pos2 { inner.center() + (pos2(x, y) - bounds.center()) * scale };
    let node_at = |id| map.node(id).map(|n| at(n.pos.x, n.pos.y));
    for n in &nodes {
        if let (Some(a), Some(b)) = (n.parent.and_then(node_at), node_at(n.id)) {
            painter.line_segment([a, b], Stroke::new(1.0, t.branch));
        }
    }
    for e in map.edges() {
        if let (Some(a), Some(b)) = (node_at(e.from), node_at(e.to)) {
            painter.extend(egui::Shape::dashed_line(&[a, b], Stroke::new(1.0, t.link), 3.0, 2.0));
        }
    }
    let size = vec2(110.0, 30.0) * scale;
    for n in &nodes {
        let r = Rect::from_center_size(at(n.pos.x, n.pos.y), size.max(vec2(10.0, 4.0)));
        painter.rect_filled(r, r.height() / 3.0, Color32::from_rgb(n.color.0, n.color.1, n.color.2));
        painter.rect_stroke(r, r.height() / 3.0, Stroke::new(0.5, t.node_border), StrokeKind::Inside);
    }
}

pub fn show(app: &mut IdeaApp, ctx: &egui::Context, t: &Tokens) {
    if !app.gallery_open {
        return;
    }
    let lang = app.language.templates();
    if app.template_previews.len() != templates::ALL.len() || app.previews_language != app.language {
        app.template_previews = templates::ALL.iter().map(|t| t.build_in(lang)).collect();
        app.previews_language = app.language;
    }
    let mut open = true;
    let mut chosen = None;
    egui::Window::new(app.tr("New from template")).open(&mut open).collapsible(false).default_size([760.0, 560.0]).show(ctx, |ui| {
        ui.label(RichText::new(app.tr("Pick a starting point. Everything stays editable.")).color(t.muted));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                for (i, tpl) in templates::ALL.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(CARD, Sense::click());
                    let hover = resp.hovered();
                    let p = ui.painter();
                    p.rect_filled(rect, 10.0, ui.visuals().extreme_bg_color);
                    p.rect_stroke(
                        rect,
                        10.0,
                        Stroke::new(if hover { 2.0 } else { 1.0 }, if hover { t.selection } else { t.node_border }),
                        StrokeKind::Inside,
                    );
                    let thumb = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(CARD.x - 16.0, THUMB_H));
                    if let Some(map) = app.template_previews.get(i) {
                        thumbnail(p, map, thumb, t);
                    }
                    let text = ui.visuals().strong_text_color();
                    p.text(
                        pos2(rect.left() + 12.0, thumb.bottom() + 8.0),
                        egui::Align2::LEFT_TOP,
                        tpl.name(lang),
                        egui::FontId::proportional(15.0),
                        text,
                    );
                    p.text(
                        pos2(rect.left() + 12.0, thumb.bottom() + 30.0),
                        egui::Align2::LEFT_TOP,
                        tpl.category(lang),
                        egui::FontId::proportional(12.0),
                        t.muted,
                    );
                    if resp.on_hover_text(tpl.description(lang)).clicked() {
                        chosen = Some(tpl.id);
                    }
                }
            });
        });
    });
    if let Some(id) = chosen {
        app.run("map.newFromTemplate", json!({"template": id, "language": app.language.code()}));
        app.path = None;
        app.view.fit_pending = true;
        open = false;
    }
    app.gallery_open = open;
}
