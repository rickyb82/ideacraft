//! The command catalog. Ids follow the app's menus (`node.addChild`, `layout.tree`) plus
//! primitives (`node.move`, `selection.set`).

use ideacraft_core::layout::{self, LEAF_GAP, LEVEL_GAP, Layout};
use ideacraft_core::{Map, NodeId, Point, Rgb, Shape};
use ideacraft_formats::templates::Lang;
use serde_json::{Value, json};

use crate::params as p;
use crate::{Error, Result, Session};

/// A command's metadata: what menus, palettes, the CLI and `--help` show.
#[derive(Clone, Copy, Debug)]
pub struct CommandInfo {
    pub id: &'static str,
    pub label: &'static str,
    /// Menu path, `Menu › Item`.
    pub menu: &'static str,
    /// Default keyboard shortcut, empty when none (`Cmd` is ⌘ on macOS, Ctrl elsewhere).
    pub shortcut: &'static str,
    /// Parameters, documented for the CLI and automation.
    pub params: &'static str,
    /// Changes the map, so it is an undo step.
    pub undoable: bool,
}

const fn c(id: &'static str, label: &'static str, menu: &'static str, shortcut: &'static str, params: &'static str, undoable: bool) -> CommandInfo {
    CommandInfo { id, label, menu, shortcut, params, undoable }
}

pub const COMMANDS: &[CommandInfo] = &[
    c("map.new", "New Map", "File › New", "Cmd+N", "{title?}", false),
    c(
        "map.newFromTemplate",
        "New from Template",
        "File › New from Template",
        "",
        "{template: brainstorm|concept-map|project-plan|swot|decision|meeting|book|fishbone|weekly|roadmap|journey|lesson, language?: en|it}",
        false,
    ),
    c("map.templates", "Templates", "File › New from Template", "", "{language?: en|it} → [{id, name, category, description}]", false),
    c(
        "map.importText",
        "Import",
        "File › Import",
        "",
        "{text, format?: mm|xmind|cxl|opml|md|mermaid|ideacraft} (detected when absent) → {format}",
        false,
    ),
    c("map.export", "Export", "File › Export", "", "{format: markdown|opml|freemind|svg|ideacraft} → {text}", false),
    c("map.setTitle", "Rename Map", "File › Rename", "", "{title}", true),
    c("map.info", "Map Info", "File › Info", "", "{} → {title, nodes, links, selection}", false),
    c("node.add", "Add Idea", "Insert › Idea", "", "{label?, x?, y?, parent?} → {id}", true),
    c("node.addChild", "Add Child Idea", "Insert › Child", "Tab", "{id?, label?} (default: the selection) → {id}", true),
    c("node.addSibling", "Add Sibling Idea", "Insert › Sibling", "Enter", "{id?, label?} → {id}", true),
    c("node.delete", "Delete", "Edit › Delete", "Delete", "{id? | ids?} (removes the whole branch)", true),
    c("node.setLabel", "Edit Text", "Edit › Edit Text", "F2", "{id?, label}", true),
    c("node.move", "Move", "Arrange › Move", "", "{id?, x, y}", true),
    c("node.moveBy", "Nudge", "Arrange › Nudge", "", "{id? | ids?, dx, dy}", true),
    c("node.setColor", "Fill Colour", "Format › Fill Colour", "", "{id? | ids?, color: \"#rrggbb\"}", true),
    c("node.setShape", "Shape", "Format › Shape", "", "{id? | ids?, shape: rounded|rect|ellipse|pill}", true),
    c("node.toggleCollapse", "Collapse/Expand Branch", "View › Collapse/Expand", "Space", "{id? | ids?, collapsed?: bool}", true),
    c("node.setParent", "Attach To", "Arrange › Attach To", "", "{id?, parent: id | null}", true),
    c("edge.add", "Link Ideas", "Insert › Link", "", "{from, to, label?} → {id}", true),
    c("edge.delete", "Delete Link", "Edit › Delete Link", "", "{id}", true),
    c("edge.setLabel", "Link Text", "Edit › Link Text", "", "{id, label}", true),
    c("layout.tree", "Tree Layout", "Arrange › Tree Layout", "Cmd+L", "{}", true),
    c("layout.radial", "Radial Layout", "Arrange › Radial Layout", "Cmd+Shift+L", "{}", true),
    c("selection.set", "Select", "Edit › Select", "", "{ids: [id…]}", false),
    c("selection.all", "Select All", "Edit › Select All", "Cmd+A", "{}", false),
    c("selection.clear", "Select None", "Edit › Select None", "Escape", "{}", false),
    c("edit.undo", "Undo", "Edit › Undo", "Cmd+Z", "{}", false),
    c("edit.redo", "Redo", "Edit › Redo", "Cmd+Shift+Z", "{}", false),
];

pub fn command(id: &str) -> Option<&'static CommandInfo> {
    COMMANDS.iter().find(|c| c.id == id)
}

fn node_pos(map: &Map, id: NodeId) -> Result<Point> {
    map.node(id).map(|n| n.pos).ok_or(Error::Model(ideacraft_core::Error::NoNode(id.0)))
}

/// Where a new child of `parent` goes: right of it, under its lowest child.
fn child_slot(map: &Map, parent: NodeId) -> Result<Point> {
    let at = node_pos(map, parent)?;
    let lowest = map.children(parent).iter().filter_map(|k| map.node(*k)).map(|n| n.pos).max_by(|a, b| a.y.total_cmp(&b.y));
    Ok(match lowest {
        Some(l) => Point::new(l.x, l.y + LEAF_GAP),
        None => Point::new(at.x + LEVEL_GAP, at.y),
    })
}

/// The optional `language` parameter of template commands.
fn language(prm: &Value) -> Result<Lang> {
    match p::opt_str(prm, "language")? {
        None => Ok(Lang::En),
        Some(code) => Lang::parse(code).ok_or_else(|| Error::Param("`language` must be en or it".into())),
    }
}

fn added(s: &mut Session, id: NodeId) -> Value {
    s.selection = vec![id];
    json!({ "id": id.0 })
}

pub(crate) fn run(s: &mut Session, id: &str, prm: &Value) -> Result<Value> {
    match id {
        "map.new" => {
            let title = p::opt_str(prm, "title")?.unwrap_or("Untitled map");
            s.replace_map(Map::new(title));
            Ok(json!({}))
        }
        "map.newFromTemplate" => {
            let id = p::str(prm, "template")?;
            let t = ideacraft_formats::templates::find(id)
                .ok_or_else(|| Error::Param(format!("no template {:?}", id.chars().take(40).collect::<String>())))?;
            s.replace_map(t.build_in(language(prm)?));
            Ok(json!({}))
        }
        "map.templates" => {
            let lang = language(prm)?;
            Ok(Value::Array(
                ideacraft_formats::templates::ALL
                    .iter()
                    .map(|t| json!({"id": t.id, "name": t.name(lang), "category": t.category(lang), "description": t.description(lang)}))
                    .collect(),
            ))
        }
        "map.importText" => {
            let text = p::str(prm, "text")?;
            let format = match p::opt_str(prm, "format")? {
                Some(f) => ideacraft_formats::Format::parse(f)
                    .ok_or_else(|| Error::Param(format!("unknown format {:?}", f.chars().take(24).collect::<String>())))?,
                None => ideacraft_formats::sniff(text),
            };
            let map = ideacraft_formats::import_text(text, format)?;
            s.replace_map(map);
            Ok(json!({"format": format.name()}))
        }
        "map.export" => {
            let text = match p::str(prm, "format")?.to_ascii_lowercase().as_str() {
                "markdown" | "md" => ideacraft_formats::to_markdown(&s.map),
                "opml" => ideacraft_formats::to_opml(&s.map),
                "freemind" | "mm" => ideacraft_formats::to_freemind(&s.map),
                "ideacraft" | "json" => s.map.to_json(),
                "svg" => ideacraft_formats::to_svg(&s.map),
                _ => return Err(Error::Param("`format` must be markdown, opml, freemind, svg or ideacraft".into())),
            };
            Ok(json!({"text": text}))
        }
        "map.setTitle" => {
            s.map.set_title(p::str(prm, "title")?);
            Ok(json!({}))
        }
        "map.info" => Ok(json!({
            "title": s.map.title(),
            "nodes": s.map.node_count(),
            "links": s.map.edge_count(),
            "selection": s.selection.iter().map(|n| n.0).collect::<Vec<_>>(),
        })),
        "node.add" => {
            let parent = p::opt_node(prm, "parent")?;
            let pos = match (p::opt_point(prm)?, parent) {
                (Some(at), _) => at,
                (None, Some(par)) => child_slot(&s.map, par)?,
                (None, None) => Point::new(0.0, s.map.nodes().map(|n| n.pos.y + 100.0).fold(0.0, f32::max)),
            };
            let label = p::opt_str(prm, "label")?.unwrap_or("Idea");
            let new = s.map.add_node(label, pos, parent)?;
            Ok(added(s, new))
        }
        "node.addChild" => {
            let parent = p::target(prm, &s.selection)?;
            let pos = child_slot(&s.map, parent)?;
            s.map.set_collapsed(parent, false)?;
            let new = s.map.add_node(p::opt_str(prm, "label")?.unwrap_or("Idea"), pos, Some(parent))?;
            Ok(added(s, new))
        }
        "node.addSibling" => {
            let of = p::target(prm, &s.selection)?;
            let label = p::opt_str(prm, "label")?.unwrap_or("Idea");
            let at = node_pos(&s.map, of)?;
            let parent = s.map.node(of).and_then(|n| n.parent);
            // Just below `of`, pushing later siblings down so the order stays as seen.
            let pos = Point::new(at.x, at.y + LEAF_GAP / 2.0);
            if let Some(par) = parent {
                for sib in s.map.children(par) {
                    let sp = node_pos(&s.map, sib)?;
                    if sib != of && sp.y > at.y {
                        s.map.set_pos(sib, Point::new(sp.x, sp.y + LEAF_GAP))?;
                    }
                }
            }
            let pos = if parent.is_some() { pos } else { Point::new(at.x, at.y + 100.0) };
            let new = s.map.add_node(label, pos, parent)?;
            Ok(added(s, new))
        }
        "node.delete" => {
            let mut removed = 0;
            for t in p::targets(prm, &s.selection)? {
                if s.map.node(t).is_some() {
                    removed += s.map.remove_node(t)?.len();
                } else if prm.get("ids").is_some() || prm.get("id").is_some() {
                    return Err(ideacraft_core::Error::NoNode(t.0).into());
                }
            }
            Ok(json!({ "removed": removed }))
        }
        "node.setLabel" => {
            let t = p::target(prm, &s.selection)?;
            s.map.set_label(t, p::str(prm, "label")?)?;
            Ok(json!({}))
        }
        "node.move" => {
            let t = p::target(prm, &s.selection)?;
            let at = p::opt_point(prm)?.ok_or_else(|| Error::Param("give `x` and `y`".into()))?;
            s.map.set_pos(t, at)?;
            Ok(json!({}))
        }
        "node.moveBy" => {
            let dx = p::opt_f32(prm, "dx")?.unwrap_or(0.0);
            let dy = p::opt_f32(prm, "dy")?.unwrap_or(0.0);
            for t in p::targets(prm, &s.selection)? {
                let at = node_pos(&s.map, t)?;
                s.map.set_pos(t, Point::new(at.x + dx, at.y + dy))?;
            }
            Ok(json!({}))
        }
        "node.setColor" => {
            let color = Rgb::parse(p::str(prm, "color")?)?;
            for t in p::targets(prm, &s.selection)? {
                s.map.set_color(t, color)?;
            }
            Ok(json!({}))
        }
        "node.setShape" => {
            let name = p::str(prm, "shape")?;
            let shape = Shape::parse(name).ok_or_else(|| Error::Param("`shape` must be rounded, rect, ellipse or pill".into()))?;
            for t in p::targets(prm, &s.selection)? {
                s.map.set_shape(t, shape)?;
            }
            Ok(json!({}))
        }
        "node.toggleCollapse" => {
            let force = prm.get("collapsed").and_then(Value::as_bool);
            for t in p::targets(prm, &s.selection)? {
                let now = s.map.node(t).map(|n| n.collapsed).ok_or(ideacraft_core::Error::NoNode(t.0))?;
                s.map.set_collapsed(t, force.unwrap_or(!now))?;
            }
            // Selected nodes that just disappeared fall back to the collapsed branch's root.
            let map = &s.map;
            s.selection.retain(|n| map.is_visible(*n));
            Ok(json!({}))
        }
        "node.setParent" => {
            let t = p::target(prm, &s.selection)?;
            let parent = match prm.get("parent") {
                Some(Value::Null) => None,
                Some(_) => Some(p::node(prm, "parent")?),
                None => return Err(Error::Param("`parent` must be a node id or null".into())),
            };
            s.map.set_parent(t, parent)?;
            Ok(json!({}))
        }
        "edge.add" => {
            let e = s.map.connect(p::node(prm, "from")?, p::node(prm, "to")?, p::opt_str(prm, "label")?.unwrap_or(""))?;
            Ok(json!({ "id": e.0 }))
        }
        "edge.delete" => {
            s.map.disconnect(p::edge(prm, "id")?)?;
            Ok(json!({}))
        }
        "edge.setLabel" => {
            s.map.set_edge_label(p::edge(prm, "id")?, p::str(prm, "label")?)?;
            Ok(json!({}))
        }
        "layout.tree" => {
            layout::apply(&mut s.map, Layout::Tree);
            Ok(json!({}))
        }
        "layout.radial" => {
            layout::apply(&mut s.map, Layout::Radial);
            Ok(json!({}))
        }
        "selection.set" => {
            let ids = p::targets(prm, &[])?;
            if let Some(bad) = ids.iter().find(|n| s.map.node(**n).is_none()) {
                return Err(ideacraft_core::Error::NoNode(bad.0).into());
            }
            s.selection = ids;
            Ok(json!({}))
        }
        "selection.all" => {
            let map = &s.map;
            s.selection = map.nodes().map(|n| n.id).filter(|n| map.is_visible(*n)).collect();
            Ok(json!({}))
        }
        "selection.clear" => {
            s.selection.clear();
            Ok(json!({}))
        }
        "edit.undo" => s.undo_step().map(|()| json!({})),
        "edit.redo" => s.redo_step().map(|()| json!({})),
        other => Err(Error::UnknownCommand(other.to_string())),
    }
}
