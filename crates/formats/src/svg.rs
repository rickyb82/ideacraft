//! SVG export: the visible map as a picture (ideas, branches, labelled links), for documents, the
//! web and agents that want to look at what they built.

use ideacraft_core::{Map, NodeId, Shape};

use crate::xml::escape;

const FONT: f32 = 14.0;
const LINE_H: f32 = 18.0;
const PAD_X: f32 = 14.0;
const PAD_Y: f32 = 9.0;
const WRAP_CHARS: usize = 28;
/// Rough average glyph width for a proportional sans-serif at [`FONT`].
const CHAR_W: f32 = 7.4;

/// Greedy word wrap at about [`WRAP_CHARS`] characters, keeping explicit line breaks.
fn wrap(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > WRAP_CHARS {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    if lines.iter().all(String::is_empty) {
        lines = vec![String::new()];
    }
    lines.truncate(12);
    lines
}

struct Box_ {
    id: NodeId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    lines: Vec<String>,
}

/// Where the line from the centre of `b` towards `(tx, ty)` leaves the box, 3 units outside it.
fn clip(b: &Box_, tx: f32, ty: f32) -> (f32, f32) {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    let (dx, dy) = (tx - cx, ty - cy);
    if dx.abs() < f32::EPSILON && dy.abs() < f32::EPSILON {
        return (cx, cy);
    }
    let sx = if dx.abs() > f32::EPSILON { (b.w / 2.0 + 3.0) / dx.abs() } else { f32::MAX };
    let sy = if dy.abs() > f32::EPSILON { (b.h / 2.0 + 3.0) / dy.abs() } else { f32::MAX };
    let k = sx.min(sy).min(1.0);
    (cx + dx * k, cy + dy * k)
}

fn num(v: f32) -> String {
    format!("{:.1}", if v.is_finite() { v } else { 0.0 })
}

/// The map as a standalone SVG document.
pub fn to_svg(map: &Map) -> String {
    let boxes: Vec<Box_> = map
        .nodes()
        .filter(|n| map.is_visible(n.id))
        .map(|n| {
            let lines = wrap(&n.label);
            let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
            let w = (longest * CHAR_W + PAD_X * 2.0).max(70.0) * if n.shape == Shape::Ellipse { 1.3 } else { 1.0 };
            let h = lines.len() as f32 * LINE_H + PAD_Y * 2.0;
            Box_ { id: n.id, x: n.pos.x - w / 2.0, y: n.pos.y - h / 2.0, w, h, lines }
        })
        .collect();
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for b in &boxes {
        x0 = x0.min(b.x);
        y0 = y0.min(b.y);
        x1 = x1.max(b.x + b.w);
        y1 = y1.max(b.y + b.h);
    }
    if boxes.is_empty() {
        (x0, y0, x1, y1) = (0.0, 0.0, 200.0, 100.0);
    }
    let margin = 40.0;
    let (vx, vy, vw, vh) = (x0 - margin, y0 - margin, x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin);
    let find = |id: NodeId| boxes.iter().find(|b| b.id == id);
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\" font-family=\"system-ui, -apple-system, 'Segoe UI', sans-serif\">\n<title>{}</title>\n<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f3f4f8\"/>\n",
        num(vx),
        num(vy),
        num(vw),
        num(vh),
        num(vw),
        num(vh),
        escape(map.title()),
        num(vx),
        num(vy),
        num(vw),
        num(vh)
    );
    out.push_str("<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"#e06a2c\"/></marker></defs>\n");
    // Tree branches.
    for b in &boxes {
        let Some(p) = map.node(b.id).and_then(|n| n.parent).and_then(find) else { continue };
        let color = map.node(b.id).map(|n| n.color.hex()).unwrap_or_default();
        let right = b.x + b.w / 2.0 >= p.x + p.w / 2.0;
        let (sx, ex) = if right { (p.x + p.w, b.x) } else { (p.x, b.x + b.w) };
        let (sy, ey) = (p.y + p.h / 2.0, b.y + b.h / 2.0);
        let bend = ((ex - sx).abs() * 0.5).max(12.0) * if right { 1.0 } else { -1.0 };
        out.push_str(&format!(
            "<path d=\"M{} {} C{} {} {} {} {} {}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.5\" stroke-opacity=\"0.9\" style=\"filter:brightness(0.8)\"/>\n",
            num(sx), num(sy), num(sx + bend), num(sy), num(ex - bend), num(ey), num(ex), num(ey)
        ));
    }
    // Free links (their labels are drawn last, above the ideas).
    let mut labels = String::new();
    for e in map.edges() {
        let (Some(a), Some(b)) = (find(e.from), find(e.to)) else { continue };
        let (ax, ay) = clip(a, b.x + b.w / 2.0, b.y + b.h / 2.0);
        let (bx, by) = clip(b, a.x + a.w / 2.0, a.y + a.h / 2.0);
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e06a2c\" stroke-width=\"1.6\" stroke-dasharray=\"7 5\" marker-end=\"url(#arrow)\"/>\n",
            num(ax), num(ay), num(bx), num(by)
        ));
        if !e.label.is_empty() {
            let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
            let w = e.label.chars().count() as f32 * 6.4 + 12.0;
            labels.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"4\" fill=\"#ffffff\"/><text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#e06a2c\" text-anchor=\"middle\">{}</text>\n",
                num(mx - w / 2.0), num(my - 8.0), num(w), num(mx), num(my + 4.0), escape(&e.label)
            ));
        }
    }
    // Ideas.
    for b in &boxes {
        let Some(n) = map.node(b.id) else { continue };
        let fill = n.color.hex();
        let shape = match n.shape {
            Shape::Ellipse => format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"",
                num(b.x + b.w / 2.0),
                num(b.y + b.h / 2.0),
                num(b.w / 2.0),
                num(b.h / 2.0)
            ),
            s => {
                let r = match s {
                    Shape::Rect => 0.0,
                    Shape::Pill => b.h / 2.0,
                    _ => 8.0,
                };
                format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"", num(b.x), num(b.y), num(b.w), num(b.h), num(r))
            }
        };
        out.push_str(&format!("{shape} fill=\"{fill}\" stroke=\"#9aa0b0\" stroke-width=\"1\"/>\n"));
        let top = b.y + b.h / 2.0 - (b.lines.len() as f32 - 1.0) * LINE_H / 2.0 + FONT * 0.35;
        for (i, line) in b.lines.iter().enumerate() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{FONT}\" fill=\"#1f222b\" text-anchor=\"middle\">{}</text>\n",
                num(b.x + b.w / 2.0),
                num(top + i as f32 * LINE_H),
                escape(line)
            ));
        }
        if n.collapsed {
            let hidden = map.descendants(n.id).len();
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#5b5bf0\"/><text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#ffffff\" text-anchor=\"middle\">{hidden}</text>\n",
                num(b.x + b.w + 12.0),
                num(b.y + b.h / 2.0),
                num(b.x + b.w + 12.0),
                num(b.y + b.h / 2.0 + 4.0)
            ));
        }
    }
    out.push_str(&labels);
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_has_every_visible_idea_and_link() {
        let map = crate::templates::find("concept-map").map(|t| t.build()).unwrap();
        let svg = to_svg(&map);
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        for n in map.nodes() {
            assert!(svg.contains(&escape(n.label.split_whitespace().next().unwrap())), "{}", n.label);
        }
        assert_eq!(svg.matches("marker-end").count(), map.edge_count());
        assert!(to_svg(&Map::default()).contains("<svg"), "an empty map is still a picture");
        assert_eq!(wrap("a b"), vec!["a b"]);
        assert!(wrap(&"word ".repeat(50)).iter().all(|l| l.chars().count() <= WRAP_CHARS));
    }
}
