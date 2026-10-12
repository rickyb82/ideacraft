//! The infinite canvas: draws ideas and links, and turns pointer and keyboard input into commands.

use std::sync::Arc;

use egui::epaint::CubicBezierShape;
use egui::{Align2, Color32, FontId, Galley, Key, Modifiers, Pos2, Rect, Sense, Shape as EShape, Stroke, StrokeKind, Vec2, pos2, vec2};
use ideacraft_engine::core::{Map, Node, NodeId, Point, Shape};
use serde_json::json;

use crate::IdeaApp;
use crate::theme::Tokens;

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 4.0;
const PAD: Vec2 = vec2(14.0, 9.0);
const MIN_WIDTH: f32 = 70.0;
const WRAP: f32 = 220.0;
const FONT: f32 = 15.0;

/// Camera: map point (0,0) is drawn at the canvas centre plus `pan`.
#[derive(Clone, Debug)]
pub struct View {
    pub pan: Vec2,
    pub zoom: f32,
    /// Fit the whole map in the window on the next frame.
    pub fit_pending: bool,
}

impl Default for View {
    fn default() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0, fit_pending: true }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CanvasState {
    drag: Option<Drag>,
}

#[derive(Debug)]
enum Drag {
    Pan,
    /// Moving ideas: preview offset in map units, applied as one command on release.
    Move {
        ids: Vec<NodeId>,
        offset: Vec2,
    },
    Link {
        from: NodeId,
        to: Pos2,
    },
}

struct Frame {
    origin: Pos2,
    zoom: f32,
}

impl Frame {
    fn to_screen(&self, p: Point) -> Pos2 {
        self.origin + vec2(p.x, p.y) * self.zoom
    }
    fn to_map(&self, p: Pos2) -> Point {
        let v = (p - self.origin) / self.zoom;
        Point::new(v.x, v.y)
    }
}

struct Laid {
    id: NodeId,
    rect: Rect,
    galley: Arc<Galley>,
}

fn fill(n: &Node) -> Color32 {
    Color32::from_rgb(n.color.0, n.color.1, n.color.2)
}

fn radius(shape: Shape, rect: Rect) -> f32 {
    match shape {
        Shape::Rounded => 8.0,
        Shape::Rect => 0.0,
        Shape::Pill | Shape::Ellipse => rect.height() / 2.0,
    }
}

/// Lays out every visible idea in screen space, honouring a drag preview.
fn lay_out(ui: &egui::Ui, map: &Map, f: &Frame, drag: Option<&Drag>, text: Color32) -> Vec<Laid> {
    let font = FontId::proportional(FONT * f.zoom);
    let offset = |id: NodeId| match drag {
        Some(Drag::Move { ids, offset }) if ids.contains(&id) => *offset,
        _ => Vec2::ZERO,
    };
    map.nodes()
        .filter(|n| map.is_visible(n.id))
        .map(|n| {
            let label = if n.label.is_empty() { " ".to_string() } else { n.label.clone() };
            let galley = ui.painter().layout(label, font.clone(), text, WRAP * f.zoom);
            let pad = PAD * f.zoom * if n.shape == Shape::Ellipse { 1.6 } else { 1.0 };
            let size = (galley.size() + pad * 2.0).max(vec2(MIN_WIDTH * f.zoom, 0.0));
            let o = offset(n.id);
            let centre = f.to_screen(Point::new(n.pos.x + o.x, n.pos.y + o.y));
            Laid { id: n.id, rect: Rect::from_center_size(centre, size), galley }
        })
        .collect()
}

fn hit(laid: &[Laid], p: Pos2) -> Option<NodeId> {
    laid.iter().rev().find(|l| l.rect.contains(p)).map(|l| l.id)
}

fn fit(view: &mut View, laid_map: &Map, canvas: Rect) {
    let pts: Vec<Point> = laid_map.nodes().filter(|n| laid_map.is_visible(n.id)).map(|n| n.pos).collect();
    view.fit_pending = false;
    if pts.is_empty() {
        view.pan = Vec2::ZERO;
        view.zoom = 1.0;
        return;
    }
    let (mut lo, mut hi) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
    for p in &pts {
        lo = lo.min(pos2(p.x, p.y));
        hi = hi.max(pos2(p.x, p.y));
    }
    let bounds = Rect::from_min_max(lo, hi).expand2(vec2(140.0, 60.0));
    let zoom = (canvas.width() / bounds.width()).min(canvas.height() / bounds.height()).clamp(MIN_ZOOM, 1.5);
    view.zoom = if zoom.is_finite() { zoom } else { 1.0 };
    view.pan = -bounds.center().to_vec2() * view.zoom;
}

/// A branch curve from parent `a` to child `b`: it leaves and enters horizontally when the child
/// sits beside its parent, vertically when it sits (nearly) above or below it (radial layouts).
fn edge_points(a: Rect, b: Rect) -> [Pos2; 4] {
    let d = b.center() - a.center();
    let gap_x = d.x.abs() - (a.width() + b.width()) / 2.0;
    if gap_x > 0.0 && d.x.abs() >= 0.4 * d.y.abs() {
        let right = d.x >= 0.0;
        let (s, e) = if right { (a.right_center(), b.left_center()) } else { (a.left_center(), b.right_center()) };
        let bend = ((e.x - s.x).abs() * 0.5).max(12.0) * if right { 1.0 } else { -1.0 };
        [s, s + vec2(bend, 0.0), e - vec2(bend, 0.0), e]
    } else {
        let down = d.y >= 0.0;
        let (s, e) = if down { (a.center_bottom(), b.center_top()) } else { (a.center_top(), b.center_bottom()) };
        let bend = ((e.y - s.y).abs() * 0.5).max(12.0) * if down { 1.0 } else { -1.0 };
        [s, s + vec2(0.0, bend), e - vec2(0.0, bend), e]
    }
}

fn draw(app: &IdeaApp, ui: &egui::Ui, painter: &egui::Painter, f: &Frame, laid: &[Laid], t: &Tokens, canvas: Rect) {
    let map = app.session.map();
    // Dot grid.
    let step = 32.0 * f.zoom;
    if step > 8.0 {
        let start = f.origin + vec2(((canvas.left() - f.origin.x) / step).floor() * step, ((canvas.top() - f.origin.y) / step).floor() * step);
        let mut y = start.y;
        while y < canvas.bottom() {
            let mut x = start.x;
            while x < canvas.right() {
                painter.circle_filled(pos2(x, y), 1.2, t.grid_dot);
                x += step;
            }
            y += step;
        }
    }
    let rect_of = |id: NodeId| laid.iter().find(|l| l.id == id).map(|l| l.rect);
    // Tree branches.
    for l in laid {
        let parent = map.node(l.id).and_then(|n| n.parent).and_then(rect_of);
        if let Some(p) = parent {
            let color = map.node(l.id).map(fill).unwrap_or(t.branch);
            let stroke = Stroke::new((2.5 * f.zoom).max(1.0), color.lerp_to_gamma(t.branch, 0.45));
            painter.add(CubicBezierShape::from_points_stroke(edge_points(p, l.rect), false, Color32::TRANSPARENT, stroke));
        }
    }
    // Free links: dashed, with an arrow and an optional label.
    let link_font = FontId::proportional(12.0 * f.zoom);
    for e in map.edges() {
        let (Some(a), Some(b)) = (rect_of(e.from), rect_of(e.to)) else { continue };
        let (s, end) = (a.center(), b.center());
        let dir = (end - s).normalized();
        let tip = clip(b, s, end);
        let start = clip(a, end, s);
        let stroke = Stroke::new((1.6 * f.zoom).max(1.0), t.link);
        painter.extend(EShape::dashed_line(&[start, tip], stroke, 7.0 * f.zoom, 5.0 * f.zoom));
        let side = vec2(-dir.y, dir.x) * 5.0 * f.zoom;
        let back = tip - dir * 10.0 * f.zoom;
        painter.add(EShape::convex_polygon(vec![tip, back + side, back - side], t.link, Stroke::NONE));
        if !e.label.is_empty() {
            let mid = start + (tip - start) / 2.0;
            let g = painter.layout_no_wrap(e.label.clone(), link_font.clone(), t.link);
            let r = Rect::from_center_size(mid, g.size() + vec2(8.0, 4.0) * f.zoom);
            painter.rect_filled(r, 4.0 * f.zoom, t.link_label_bg);
            painter.galley(r.min + vec2(4.0, 2.0) * f.zoom, g, t.link);
        }
    }
    // Ideas.
    let selected = app.session.selection();
    for l in laid {
        let Some(n) = map.node(l.id) else { continue };
        let r = l.rect;
        let border = if selected.contains(&n.id) { Stroke::new(2.5, t.selection) } else { Stroke::new(1.0, t.node_border) };
        if n.shape == Shape::Ellipse {
            painter.add(EShape::ellipse_filled(r.center(), r.size() / 2.0, fill(n)));
            painter.add(EShape::ellipse_stroke(r.center(), r.size() / 2.0, border));
        } else {
            let rr = radius(n.shape, r) * if n.shape == Shape::Rounded { f.zoom } else { 1.0 };
            if selected.contains(&n.id) {
                painter.rect_filled(r.expand(5.0), rr + 5.0, t.selection.gamma_multiply(0.18));
            }
            painter.rect_filled(r.translate(vec2(0.0, 3.0 * f.zoom)).expand(1.0), rr, Color32::from_black_alpha(14));
            painter.rect_filled(r.translate(vec2(0.0, 1.0 * f.zoom)), rr, Color32::from_black_alpha(16));
            painter.rect_filled(r, rr, fill(n));
            painter.rect_stroke(r, rr, border, StrokeKind::Inside);
        }
        let editing = app.editing.as_ref().is_some_and(|(id, _)| *id == n.id);
        if !editing {
            painter.galley(r.center() - l.galley.size() / 2.0, l.galley.clone(), t.text);
        }
        let hidden = if n.collapsed { map.descendants(n.id).len() } else { 0 };
        if hidden > 0 {
            let badge = r.right_center() + vec2(12.0 * f.zoom, 0.0);
            painter.circle_filled(badge, 10.0 * f.zoom, t.selection);
            painter.text(badge, Align2::CENTER_CENTER, format!("{hidden}"), FontId::proportional(11.0 * f.zoom), Color32::WHITE);
        }
    }
    if let Some(Drag::Link { from, to }) = &app.canvas.drag
        && let Some(a) = rect_of(*from)
    {
        painter.line_segment([a.center(), *to], Stroke::new(2.0, t.link));
    }
    let _ = ui;
}

/// Where the segment from `outside` to `centre` enters `r`.
fn clip(r: Rect, outside: Pos2, centre: Pos2) -> Pos2 {
    let d = outside - centre;
    if d.x.abs() < f32::EPSILON && d.y.abs() < f32::EPSILON {
        return centre;
    }
    let sx = if d.x.abs() > f32::EPSILON { (r.width() / 2.0) / d.x.abs() } else { f32::MAX };
    let sy = if d.y.abs() > f32::EPSILON { (r.height() / 2.0) / d.y.abs() } else { f32::MAX };
    centre + d * sx.min(sy).min(1.0)
}

/// Canvas shortcuts. Runs before any widget so Tab adds a child instead of moving focus.
pub(crate) fn keyboard(app: &mut IdeaApp, ui: &egui::Ui) {
    if app.editing.is_some() || ui.ctx().memory(|m| m.focused().is_some()) {
        return;
    }
    let pressed = |m: Modifiers, k: Key| ui.ctx().input_mut(|i| i.consume_key(m, k));
    let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
    if pressed(shift_cmd, Key::Z) || pressed(Modifiers::COMMAND, Key::Y) {
        app.run("edit.redo", json!({}));
    } else if pressed(Modifiers::COMMAND, Key::Z) {
        app.run("edit.undo", json!({}));
    } else if pressed(shift_cmd, Key::L) {
        app.run("layout.radial", json!({}));
    } else if pressed(Modifiers::COMMAND, Key::L) {
        app.run("layout.tree", json!({}));
    } else if pressed(Modifiers::COMMAND, Key::A) {
        app.run("selection.all", json!({}));
    } else if pressed(Modifiers::COMMAND, Key::Num0) {
        app.view.fit_pending = true;
    } else if pressed(Modifiers::COMMAND, Key::O) {
        app.requests.push(crate::Request::Open);
    } else if pressed(shift_cmd, Key::S) {
        app.requests.push(crate::Request::SaveAs);
    } else if pressed(Modifiers::COMMAND, Key::S) {
        if app.path.is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            app.save();
        } else {
            app.requests.push(crate::Request::SaveAs);
        }
    } else if pressed(Modifiers::COMMAND, Key::N) {
        app.new_map();
    } else if app.session.selection().is_empty() {
        let _ = pressed(Modifiers::NONE, Key::Tab);
        if pressed(Modifiers::NONE, Key::Escape) {
            app.link_mode = false;
        }
    } else if pressed(Modifiers::NONE, Key::Tab) {
        app.run_and_edit("node.addChild", json!({}));
    } else if pressed(Modifiers::NONE, Key::Enter) {
        app.run_and_edit("node.addSibling", json!({}));
    } else if pressed(Modifiers::NONE, Key::Delete) || pressed(Modifiers::NONE, Key::Backspace) {
        app.run("node.delete", json!({}));
    } else if pressed(Modifiers::NONE, Key::Space) {
        app.run("node.toggleCollapse", json!({}));
    } else if pressed(Modifiers::NONE, Key::F2) {
        if let Some(id) = app.session.selection().first().copied() {
            app.begin_edit(id);
        }
    } else if pressed(Modifiers::NONE, Key::Escape) {
        app.run("selection.clear", json!({}));
    }
}

fn select(app: &mut IdeaApp, id: NodeId, additive: bool) {
    let mut ids: Vec<u64> = if additive { app.session.selection().iter().map(|n| n.0).collect() } else { Vec::new() };
    if let Some(i) = ids.iter().position(|n| *n == id.0) {
        ids.remove(i);
    } else {
        ids.push(id.0);
    }
    app.run("selection.set", json!({"ids": ids}));
}

fn text_editor(app: &mut IdeaApp, ui: &egui::Ui, laid: &[Laid], f: &Frame) {
    let Some((id, mut draft)) = app.editing.clone() else { return };
    let Some(rect) = laid.iter().find(|l| l.id == id).map(|l| l.rect) else {
        app.editing = None;
        return;
    };
    let width = rect.width().max(160.0 * f.zoom);
    let mut done = false;
    let mut cancel = false;
    egui::Area::new(egui::Id::new("idea-editor")).fixed_pos(pos2(rect.center().x - width / 2.0, rect.top())).order(egui::Order::Foreground).show(
        ui.ctx(),
        |ui| {
            let enter = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            cancel = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            let edit = egui::TextEdit::multiline(&mut draft)
                .desired_width(width)
                .desired_rows(1)
                .font(FontId::proportional(FONT * f.zoom))
                .horizontal_align(egui::Align::Center)
                .hint_text(app.tr("Idea"));
            let resp = ui.add(edit);
            if !resp.has_focus() && !resp.lost_focus() {
                resp.request_focus();
            }
            done = enter || resp.lost_focus();
        },
    );
    if cancel {
        app.editing = None;
    } else {
        app.editing = Some((id, draft));
        if done {
            app.commit_edit();
        }
    }
}

pub fn show(app: &mut IdeaApp, ui: &mut egui::Ui, t: &Tokens) {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let canvas = resp.rect;
    if app.view.fit_pending {
        fit(&mut app.view, app.session.map(), canvas);
    }
    // Zoom (pinch or Cmd+wheel) around the pointer; plain wheel pans.
    if resp.hovered() {
        let (zoom_delta, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
        if (zoom_delta - 1.0).abs() > f32::EPSILON {
            let pointer = resp.hover_pos().unwrap_or(canvas.center());
            let new_zoom = (app.view.zoom * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
            let anchor = pointer - canvas.center() - app.view.pan;
            app.view.pan -= anchor * (new_zoom / app.view.zoom - 1.0);
            app.view.zoom = new_zoom;
        } else {
            app.view.pan += scroll;
        }
    }
    let f = Frame { origin: canvas.center() + app.view.pan, zoom: app.view.zoom };
    let mut laid = lay_out(ui, app.session.map(), &f, app.canvas.drag.as_ref(), t.text);
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let shift = ui.input(|i| i.modifiers.shift);
    let additive = ui.input(|i| i.modifiers.command || i.modifiers.shift);

    if resp.drag_started() {
        app.commit_edit();
        let on = pointer.and_then(|p| hit(&laid, p));
        app.canvas.drag = match on {
            Some(id) if shift || app.link_mode => Some(Drag::Link { from: id, to: pointer.unwrap_or_default() }),
            Some(id) => {
                if !app.session.selection().contains(&id) {
                    select(app, id, false);
                }
                Some(Drag::Move { ids: app.session.selection().to_vec(), offset: Vec2::ZERO })
            }
            None => Some(Drag::Pan),
        };
    }
    if resp.dragged() {
        let delta = resp.drag_delta();
        match &mut app.canvas.drag {
            Some(Drag::Pan) => app.view.pan += delta,
            Some(Drag::Move { offset, .. }) => *offset += delta / app.view.zoom,
            Some(Drag::Link { to, .. }) => *to = pointer.unwrap_or(*to),
            None => {}
        }
    }
    if resp.drag_stopped() {
        match app.canvas.drag.take() {
            Some(Drag::Move { ids, offset }) if offset != Vec2::ZERO => {
                app.run("node.moveBy", json!({"ids": ids.iter().map(|n| n.0).collect::<Vec<_>>(), "dx": offset.x, "dy": offset.y}));
            }
            Some(Drag::Link { from, to }) => match hit(&laid, to) {
                Some(target) if target != from => {
                    app.run("edge.add", json!({"from": from.0, "to": target.0}));
                }
                _ => {}
            },
            _ => {}
        }
        laid = lay_out(ui, app.session.map(), &f, None, t.text);
    }
    if resp.double_clicked() {
        match pointer.and_then(|p| hit(&laid, p)) {
            Some(id) => app.begin_edit(id),
            None => {
                let at = f.to_map(pointer.unwrap_or(canvas.center()));
                app.run_and_edit("node.add", json!({"x": at.x, "y": at.y}));
            }
        }
        laid = lay_out(ui, app.session.map(), &f, None, t.text);
    } else if resp.clicked() {
        app.commit_edit();
        match pointer.and_then(|p| hit(&laid, p)) {
            Some(id) => select(app, id, additive),
            None => {
                app.run("selection.clear", json!({}));
            }
        }
    }
    // A command may have added or removed ideas.
    let laid = if app.editing.is_some() || resp.changed() { lay_out(ui, app.session.map(), &f, app.canvas.drag.as_ref(), t.text) } else { laid };
    draw(app, ui, &painter, &f, &laid, t, canvas);
    text_editor(app, ui, &laid, &f);
    overlays(app, ui, canvas, &laid, t);
}

fn card_frame(t: &Tokens) -> egui::Frame {
    egui::Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(6, 4))
        .shadow(egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if t.dark { 90 } else { 28 }) })
}

/// Floating controls over the canvas: the selection bar, zoom, and the empty-map invitation.
fn overlays(app: &mut IdeaApp, ui: &egui::Ui, canvas: Rect, laid: &[Laid], t: &Tokens) {
    use crate::icons::Icon;
    use crate::widgets::{divider, icon_button, primary_button};
    let ctx = ui.ctx().clone();
    // Selection bar.
    // Above the first selected idea (below it near the top edge), kept inside the canvas.
    let anchor = app.session.selection().first().and_then(|id| laid.iter().find(|l| l.id == *id)).map(|l| l.rect);
    if let (Some(r), None, None) = (anchor, &app.editing, &app.canvas.drag) {
        let above = r.top() - 58.0 > canvas.top() + 8.0;
        let y = if above { r.top() - 12.0 } else { r.bottom() + 12.0 };
        let x = r.center().x.clamp(canvas.left() + 190.0, (canvas.right() - 190.0).max(canvas.left() + 190.0));
        let pivot = if above { Align2::CENTER_BOTTOM } else { Align2::CENTER_TOP };
        egui::Area::new(egui::Id::new("selection-bar"))
            .order(egui::Order::Foreground)
            .pivot(pivot)
            .fixed_pos(pos2(x, y.clamp(canvas.top() + 8.0, canvas.bottom() - 8.0)))
            .show(&ctx, |ui| {
                card_frame(t).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let palette = icon_button(ui, t, Icon::Palette, app.tr("Colour"), true, false);
                        egui::Popup::menu(&palette).show(|ui| {
                            ui.set_max_width(170.0);
                            crate::sidebar::swatches(app, ui, t, true, 28.0);
                        });
                        let shape = icon_button(ui, t, Icon::Shape, app.tr("Shape"), true, false);
                        egui::Popup::menu(&shape).show(|ui| {
                            for s in Shape::ALL {
                                if ui.button(app.tr(s.name())).clicked() {
                                    app.run("node.setShape", json!({"shape": s.name()}));
                                    ui.close();
                                }
                            }
                        });
                        divider(ui, t);
                        if icon_button(ui, t, Icon::Child, &app.tip("node.addChild"), true, false).clicked() {
                            app.run_and_edit("node.addChild", json!({}));
                        }
                        if icon_button(ui, t, Icon::Sibling, &app.tip("node.addSibling"), true, false).clicked() {
                            app.run_and_edit("node.addSibling", json!({}));
                        }
                        if icon_button(ui, t, Icon::Collapse, &app.tip("node.toggleCollapse"), true, false).clicked() {
                            app.run("node.toggleCollapse", json!({}));
                        }
                        let link_tip = app.tr("Drag from one idea to another to link them (or hold Shift while dragging)").to_string();
                        if icon_button(ui, t, Icon::Link, &link_tip, true, app.link_mode).clicked() {
                            app.link_mode = !app.link_mode;
                        }
                        divider(ui, t);
                        if icon_button(ui, t, Icon::Trash, &app.tip("node.delete"), true, false).clicked() {
                            app.run("node.delete", json!({}));
                        }
                    });
                });
            });
    }
    // Zoom.
    egui::Area::new(egui::Id::new("zoom-bar"))
        .order(egui::Order::Foreground)
        .pivot(Align2::RIGHT_BOTTOM)
        .fixed_pos(canvas.right_bottom() - vec2(14.0, 14.0))
        .show(&ctx, |ui| {
            card_frame(t).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let zoom = |app: &mut IdeaApp, k: f32| app.view.zoom = (app.view.zoom * k).clamp(MIN_ZOOM, MAX_ZOOM);
                    if icon_button(ui, t, Icon::ZoomOut, app.tr("Zoom out"), true, false).clicked() {
                        zoom(app, 1.0 / 1.2);
                    }
                    ui.label(egui::RichText::new(format!("{:.0}%", app.view.zoom * 100.0)).color(t.ui_text).small());
                    if icon_button(ui, t, Icon::ZoomIn, app.tr("Zoom in"), true, false).clicked() {
                        zoom(app, 1.2);
                    }
                    if icon_button(ui, t, Icon::Fit, app.tr("Fit to window"), true, false).clicked() {
                        app.view.fit_pending = true;
                    }
                });
            });
        });
    // Empty map.
    if app.session.map().node_count() == 0 && app.editing.is_none() {
        egui::Area::new(egui::Id::new("empty-map")).order(egui::Order::Middle).pivot(Align2::CENTER_CENTER).fixed_pos(canvas.center()).show(
            &ctx,
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(app.tr("Double-click anywhere to add an idea")).size(18.0).color(t.muted))
                            .selectable(false),
                    );
                    ui.add_space(6.0);
                    ui.add(egui::Label::new(egui::RichText::new(app.tr("or start from a template")).color(t.muted)).selectable(false));
                    ui.add_space(10.0);
                    if primary_button(ui, t, Some(Icon::Templates), app.tr("Choose a template")).clicked() {
                        app.sidebar = Some(crate::SidePanel::Templates);
                    }
                });
            },
        );
    }
}
