//! IdeaCraft's MCP server (layer L6): the Model Context Protocol over stdio (newline-delimited
//! JSON-RPC 2.0), so AI agents can create, read, edit, lay out and export concept maps.
//!
//! Every edit goes through the engine's commands ([`Session::execute`]), so it is validated, can
//! be undone (`run_command` with `edit.undo`) and never leaves a half-applied map.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::io::{BufRead, Write};

use ideacraft_engine::core::{Map, NodeId, stencil};
use ideacraft_engine::formats::{self, templates};
use ideacraft_engine::{COMMANDS, Session};
use serde_json::{Value, json};

/// MCP protocol revision this server prefers.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Revisions it also speaks; a client asking for another gets [`PROTOCOL_VERSION`].
pub const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// The open map as an outline with ids (`resources/read`).
pub const MAP_URI: &str = "ideacraft://map";
/// The open map as IdeaCraft JSON (`resources/read`).
pub const MAP_JSON_URI: &str = "ideacraft://map.json";
/// Longest request line read (bytes); longer lines are answered with an error.
const MAX_LINE: usize = 32 * 1024 * 1024;

const INSTRUCTIONS: &str = "IdeaCraft edits one map at a time: a mind/concept map (ideas in a tree plus labelled links) or, in diagram mode, a free diagram \
(network diagrams, flowcharts, architectures) made of stencil shapes (list_stencils: routers, switches, servers, clouds, flowchart symbols…) with sizes, \
properties (ip, vlan, status…) and styled connectors (style_link: orthogonal/dashed/arrows). Use add_shape for diagrams, add_idea/add_ideas for maps, \
align to tidy shapes, export drawio to hand the result to draw.io/Visio and svg to look at it. IdeaCraft edits one concept map at a time. Start with new_map (optionally from a template) or open_map, then add ideas \
with add_idea/add_ideas (children hang under a parent id), connect any two ideas with link_ideas (a labelled concept-map link), \
rearrange with layout, and read the result with get_map (an outline with ids) or export (svg to look at it). Save with save_map. \
run_command reaches every engine command (list_commands), including edit.undo and edit.redo.";

/// The server: one open map, and the file it came from (path and the format it is saved in).
#[derive(Default)]
pub struct Server {
    session: Session,
    file: Option<(String, &'static str)>,
}

/// The format a file name asks for when saving: `ideacraft`, `markdown`, `opml`, `freemind` or `svg`.
fn format_for_path(path: &str) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    let by_ext = [
        (".ideacraft.json", "ideacraft"),
        (".json", "ideacraft"),
        (".md", "markdown"),
        (".markdown", "markdown"),
        (".txt", "markdown"),
        (".opml", "opml"),
        (".mm", "freemind"),
        (".svg", "svg"),
        (".drawio", "drawio"),
    ];
    by_ext.iter().find(|(e, _)| lower.ends_with(e)).map(|(_, f)| *f)
}

type ToolResult = Result<Value, String>;

fn tools() -> Value {
    let id = json!({"type": "integer", "minimum": 1});
    let color = json!({"type": "string", "description": "Fill colour, #rrggbb"});
    let shape = json!({"type": "string", "enum": ["rounded", "rect", "ellipse", "pill"]});
    let language = json!({"type": "string", "enum": ["en", "it"], "description": "Template language (default en)"});
    let template_ids: Vec<&str> = templates::ALL.iter().map(|t| t.id).collect();
    json!([
        {"name": "list_templates", "description": "List the built-in templates (id, name, category, description).",
         "inputSchema": {"type": "object", "properties": {"language": language}}},
        {"name": "new_map", "description": "Start a new map, empty or from a template. Replaces the open map.",
         "inputSchema": {"type": "object", "properties": {"title": {"type": "string"}, "template": {"type": "string", "enum": template_ids}, "language": language}}},
        {"name": "open_map", "description": "Open a file: an IdeaCraft map or an import (FreeMind/Freeplane .mm, XMind .xmind, CmapTools .cxl, OPML, Markdown, Mermaid). .ideacraft.json, .mm, .opml and .md files are saved back in their own format.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
        {"name": "import_text", "description": "Replace the map with text in another format (Markdown outline, OPML, FreeMind XML, Mermaid mindmap/flowchart, CXL, XMind content.json). The format is detected when omitted.",
         "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}, "format": {"type": "string", "enum": ["md", "opml", "mm", "mermaid", "cxl", "xmind", "ideacraft"]}}, "required": ["text"]}},
        {"name": "save_map", "description": "Save the map. The format follows the extension (.ideacraft.json, .md, .opml, .mm, .drawio, .svg) unless given. Without path, saves back to the file it was opened from (a .mm stays FreeMind, an .opml stays OPML) or last saved to.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "format": {"type": "string", "enum": ["ideacraft", "markdown", "opml", "freemind", "drawio", "svg"]}}}},
        {"name": "get_map", "description": "Read the map: title, counts, an outline with each idea's [id], and the links. format=json returns the full IdeaCraft JSON (positions, colours).",
         "inputSchema": {"type": "object", "properties": {"format": {"type": "string", "enum": ["outline", "json"]}}}},
        {"name": "add_idea", "description": "Add one idea, optionally as a child of `parent`. Returns its id.",
         "inputSchema": {"type": "object", "properties": {"label": {"type": "string"}, "parent": id, "color": color, "shape": shape}, "required": ["label"]}},
        {"name": "add_ideas", "description": "Add several child ideas under `parent` (or as new roots without it). Returns their ids in order.",
         "inputSchema": {"type": "object", "properties": {"parent": id, "labels": {"type": "array", "items": {"type": "string"}, "maxItems": 500}}, "required": ["labels"]}},
        {"name": "update_idea", "description": "Change an idea: text, colour, shape, collapsed, or move it under another parent (null makes it a root).",
         "inputSchema": {"type": "object", "properties": {"id": id, "label": {"type": "string"}, "color": color, "shape": shape, "collapsed": {"type": "boolean"}, "parent": {"type": ["integer", "null"]}}, "required": ["id"]}},
        {"name": "delete_idea", "description": "Delete an idea with its whole branch and the links touching it.",
         "inputSchema": {"type": "object", "properties": {"id": id}, "required": ["id"]}},
        {"name": "link_ideas", "description": "Link two ideas with an optional label (the linking phrase of a concept map, e.g. \"causes\"). Returns the link id.",
         "inputSchema": {"type": "object", "properties": {"from": id, "to": id, "label": {"type": "string"}}, "required": ["from", "to"]}},
        {"name": "unlink", "description": "Delete a link by its id.",
         "inputSchema": {"type": "object", "properties": {"id": id}, "required": ["id"]}},
        {"name": "layout", "description": "Arrange the whole map: tree (left to right) or radial (rings around each root).",
         "inputSchema": {"type": "object", "properties": {"kind": {"type": "string", "enum": ["tree", "radial"]}}, "required": ["kind"]}},
        {"name": "export", "description": "The map as text: markdown, opml, freemind (.mm XML), drawio (for draw.io / Visio), svg (a picture of the map) or ideacraft JSON.",
         "inputSchema": {"type": "object", "properties": {"format": {"type": "string", "enum": ["markdown", "opml", "freemind", "drawio", "svg", "ideacraft"]}}, "required": ["format"]}},
        {"name": "list_stencils", "description": "The shape library for diagrams: id, name, category (basic shapes, flowchart, network, cloud & servers, people & places), shape and default size.",
         "inputSchema": {"type": "object", "properties": {"language": language, "category": {"type": "string", "description": "Filter: basic, flowchart, network, cloud, people"}}}},
        {"name": "add_shape", "description": "Add a diagram shape from a stencil (see list_stencils), e.g. net.router, flow.decision, basic.container. Positions are map units (centre); shapes are about 110×90. Switches the map to diagram mode. Returns the id.",
         "inputSchema": {"type": "object", "properties": {"kind": {"type": "string"}, "label": {"type": "string"}, "language": language, "x": {"type": "number"}, "y": {"type": "number"}, "w": {"type": "number"}, "h": {"type": "number"},
             "props": {"type": "object", "additionalProperties": {"type": "string"}, "description": "Properties such as ip, vlan, model, owner, status (up|warning|down shows a light)"}}, "required": ["kind"]}},
        {"name": "style_link", "description": "Style a link: line solid|dashed|dotted, arrows none|end|start|both, route straight|orthogonal|curved, colour #rrggbb (null for default), width in px, and a new label.",
         "inputSchema": {"type": "object", "properties": {"id": id, "line": {"type": "string", "enum": ["solid", "dashed", "dotted"]}, "arrows": {"type": "string", "enum": ["none", "end", "start", "both"]},
             "route": {"type": "string", "enum": ["straight", "orthogonal", "curved"]}, "color": {"type": ["string", "null"]}, "width": {"type": "number"}, "label": {"type": "string"}}, "required": ["id"]}},
        {"name": "align", "description": "Tidy shapes: align several ids on an edge (left|right|top|bottom|centerX|centerY), distribute them along x or y, or snap them to the grid.",
         "inputSchema": {"type": "object", "properties": {"ids": {"type": "array", "items": id}, "edge": {"type": "string", "enum": ["left", "right", "top", "bottom", "centerX", "centerY"]},
             "distribute": {"type": "string", "enum": ["x", "y"]}, "snap": {"type": "boolean"}}, "required": ["ids"]}},
        {"name": "set_mode", "description": "Switch the map between mind-map mode (tree layouts, concept links) and diagram mode (free shapes, snapping, styled connectors).",
         "inputSchema": {"type": "object", "properties": {"mode": {"type": "string", "enum": ["map", "diagram"]}}, "required": ["mode"]}},
        {"name": "list_commands", "description": "Every engine command with its parameters, for run_command.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "run_command", "description": "Run any engine command by id (see list_commands), e.g. edit.undo, node.moveBy, selection.set.",
         "inputSchema": {"type": "object", "properties": {"command": {"type": "string"}, "params": {"type": "object"}}, "required": ["command"]}}
    ])
}

fn str_arg<'a>(args: &'a Value, name: &str) -> Option<&'a str> {
    args.get(name).and_then(Value::as_str)
}

fn id_arg(args: &Value, name: &str) -> Result<u64, String> {
    args.get(name).and_then(Value::as_u64).ok_or_else(|| format!("`{name}` must be an idea id (a positive integer)"))
}

/// Outline with ids, then links: what an agent needs to keep editing.
pub fn outline(map: &Map) -> String {
    let mut out = format!("# {}\n{} ideas, {} links\n\n", map.title(), map.node_count(), map.edge_count());
    let mut stack: Vec<(NodeId, usize)> = map.roots().into_iter().rev().map(|r| (r, 0)).collect();
    let mut seen = 0usize;
    while let Some((id, depth)) = stack.pop() {
        seen += 1;
        if seen > ideacraft_engine::core::limits::MAX_NODES {
            break;
        }
        let Some(n) = map.node(id) else { continue };
        let folded = if n.collapsed { " (collapsed)" } else { "" };
        let mut extra = Vec::new();
        if let Some(k) = &n.kind {
            extra.push(k.clone());
        }
        if n.shape != ideacraft_engine::core::Shape::Rounded {
            extra.push(n.shape.name().to_string());
        }
        if let Some(s) = n.size {
            extra.push(format!("{:.0}×{:.0} at ({:.0}, {:.0})", s.w, s.h, n.pos.x, n.pos.y));
        }
        for (k, v) in &n.props {
            extra.push(format!("{k}={v}"));
        }
        let extra = if extra.is_empty() { String::new() } else { format!(" {{{}}}", extra.join(", ")) };
        out.push_str(&format!("{}- [{}] {}{folded}{extra}\n", "  ".repeat(depth.min(64)), n.id.0, n.label.replace('\n', " ")));
        for kid in map.children(id).into_iter().rev() {
            stack.push((kid, depth + 1));
        }
    }
    if map.edge_count() > 0 {
        out.push_str("\nLinks:\n");
        for e in map.edges() {
            let label = if e.label.is_empty() { String::new() } else { format!(" \"{}\"", e.label) };
            let st = e.style;
            let style = if st == Default::default() {
                String::new()
            } else {
                format!(" ({}, {}, arrows {})", st.line.name(), st.route.name(), st.arrows.name())
            };
            out.push_str(&format!("- link {}: [{}] → [{}]{label}{style}\n", e.id.0, e.from.0, e.to.0));
        }
    }
    out
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn map(&self) -> &Map {
        self.session.map()
    }

    fn exec(&mut self, command: &str, params: Value) -> ToolResult {
        self.session.execute(command, &params).map_err(|e| format!("{command}: {e}"))
    }

    fn render(&self, format: &str) -> Result<String, String> {
        let map = self.session.map();
        Ok(match format {
            "ideacraft" | "json" => map.to_json(),
            "markdown" | "md" => formats::to_markdown(map),
            "opml" => formats::to_opml(map),
            "freemind" | "mm" => formats::to_freemind(map),
            "svg" => formats::to_svg(map),
            "drawio" => formats::to_drawio(map),
            other => return Err(format!("unknown format {other:?}")),
        })
    }

    /// Runs one tool.
    pub fn call(&mut self, name: &str, args: &Value) -> ToolResult {
        match name {
            "list_templates" => {
                let language = str_arg(args, "language").unwrap_or("en");
                self.exec("map.templates", json!({"language": language}))
            }
            "new_map" => {
                if let Some(t) = str_arg(args, "template") {
                    self.exec("map.newFromTemplate", json!({"template": t, "language": str_arg(args, "language").unwrap_or("en")}))?;
                    if let Some(title) = str_arg(args, "title") {
                        self.exec("map.setTitle", json!({"title": title}))?;
                        self.session.mark_saved();
                    }
                } else {
                    self.exec("map.new", json!({"title": str_arg(args, "title").unwrap_or("Untitled map")}))?;
                }
                self.file = None;
                Ok(json!({"outline": outline(self.map())}))
            }
            "open_map" | "open" | "open_file" | "open_document" => {
                let path = str_arg(args, "path").or_else(|| str_arg(args, "file")).ok_or("`path` is required")?;
                let format = self.open_file(path)?;
                Ok(json!({"format": format, "path": path, "saves_back": self.file.is_some(), "outline": outline(self.map())}))
            }
            "import_text" => {
                let text = str_arg(args, "text").ok_or("`text` is required")?;
                let mut params = json!({"text": text});
                if let Some(f) = str_arg(args, "format") {
                    params["format"] = json!(f);
                }
                let r = self.exec("map.importText", params)?;
                self.file = None;
                Ok(json!({"format": r["format"], "outline": outline(self.map())}))
            }
            "save_map" | "save" | "save_file" | "save_document" => {
                let (path, source_format) = match str_arg(args, "path").or_else(|| str_arg(args, "file")) {
                    Some(p) => (p.to_string(), None),
                    None => self.file.clone().map(|(p, f)| (p, Some(f))).ok_or("`path` is required (the map has no file to save back to)")?,
                };
                let format = match str_arg(args, "format") {
                    Some(f) => f.to_string(),
                    None => source_format.or_else(|| format_for_path(&path)).unwrap_or("ideacraft").to_string(),
                };
                let text = self.render(&format)?;
                std::fs::write(&path, text).map_err(|e| format!("{path}: {e}"))?;
                if format != "svg" {
                    self.session.mark_saved();
                    if let Some(f) = format_for_path(&path).filter(|f| *f == format.as_str() || (format == "json" && *f == "ideacraft")) {
                        self.file = Some((path.clone(), f));
                    }
                }
                Ok(json!({"saved": path, "format": format}))
            }
            "get_map" => match str_arg(args, "format").unwrap_or("outline") {
                "json" => Ok(json!({"json": self.map().to_json()})),
                _ => Ok(json!({"outline": outline(self.map())})),
            },
            "list_stencils" => {
                let italian = str_arg(args, "language") == Some("it");
                let category = str_arg(args, "category").map(|c| c.to_ascii_lowercase());
                Ok(Value::Array(
                    stencil::ALL
                        .iter()
                        .filter(|s| category.as_deref().is_none_or(|c| s.id.starts_with(c) || s.category.name().to_ascii_lowercase().starts_with(c)))
                        .map(|s| json!({"id": s.id, "name": s.name(italian), "category": if italian { s.category.name_it() } else { s.category.name() }, "shape": s.shape.name(), "w": s.size.w, "h": s.size.h}))
                        .collect(),
                ))
            }
            "add_shape" => {
                let mut params = json!({"kind": str_arg(args, "kind").ok_or("`kind` is required")?});
                for key in ["label", "x", "y", "props", "language"] {
                    if let Some(v) = args.get(key).filter(|v| !v.is_null()) {
                        params[key] = v.clone();
                    }
                }
                let id = self.exec("node.addShape", params)?["id"].as_u64().ok_or("no id returned")?;
                if let (Some(w), Some(h)) = (args.get("w").and_then(Value::as_f64), args.get("h").and_then(Value::as_f64)) {
                    self.exec("node.resize", json!({"id": id, "w": w, "h": h}))?;
                }
                Ok(json!({"id": id}))
            }
            "style_link" => {
                let id = id_arg(args, "id")?;
                let mut params = json!({"id": id});
                for key in ["line", "arrows", "route", "color", "width"] {
                    if let Some(v) = args.get(key) {
                        params[key] = v.clone();
                    }
                }
                if params.as_object().is_some_and(|o| o.len() > 1) {
                    self.exec("edge.setStyle", params)?;
                }
                if let Some(label) = str_arg(args, "label") {
                    self.exec("edge.setLabel", json!({"id": id, "label": label}))?;
                }
                Ok(json!({"id": id}))
            }
            "align" => {
                let ids = args.get("ids").cloned().ok_or("`ids` is required")?;
                if let Some(edge) = str_arg(args, "edge") {
                    self.exec("arrange.align", json!({"ids": ids, "edge": edge}))?;
                }
                if let Some(axis) = str_arg(args, "distribute") {
                    self.exec("arrange.distribute", json!({"ids": ids, "axis": axis}))?;
                }
                if args.get("snap").and_then(Value::as_bool) == Some(true) {
                    self.exec("arrange.snap", json!({"ids": ids}))?;
                }
                Ok(json!({"outline": outline(self.map())}))
            }
            "set_mode" => self.exec("map.setMode", json!({"mode": str_arg(args, "mode").unwrap_or_default()})),
            "add_idea" => {
                let label = str_arg(args, "label").ok_or("`label` is required")?;
                let mut params = json!({"label": label});
                if args.get("parent").is_some_and(|p| !p.is_null()) {
                    params["parent"] = json!(id_arg(args, "parent")?);
                }
                let id = self.exec("node.add", params)?["id"].as_u64().ok_or("no id returned")?;
                self.style(id, args)?;
                Ok(json!({"id": id}))
            }
            "add_ideas" => {
                let labels = args.get("labels").and_then(Value::as_array).ok_or("`labels` must be a list of strings")?;
                let parent = if args.get("parent").is_some_and(|p| !p.is_null()) { Some(id_arg(args, "parent")?) } else { None };
                let mut ids = Vec::new();
                for label in labels.iter().take(500) {
                    let label = label.as_str().ok_or("`labels` must be a list of strings")?;
                    let mut params = json!({"label": label});
                    if let Some(p) = parent {
                        params["parent"] = json!(p);
                    }
                    ids.push(self.exec("node.add", params)?["id"].clone());
                }
                Ok(json!({"ids": ids}))
            }
            "update_idea" => {
                let id = id_arg(args, "id")?;
                if let Some(label) = str_arg(args, "label") {
                    self.exec("node.setLabel", json!({"id": id, "label": label}))?;
                }
                if let Some(c) = args.get("collapsed").and_then(Value::as_bool) {
                    self.exec("node.toggleCollapse", json!({"id": id, "collapsed": c}))?;
                }
                if let Some(parent) = args.get("parent") {
                    self.exec("node.setParent", json!({"id": id, "parent": parent}))?;
                }
                self.style(id, args)?;
                Ok(json!({"id": id}))
            }
            "delete_idea" => self.exec("node.delete", json!({"id": id_arg(args, "id")?})),
            "link_ideas" => self
                .exec("edge.add", json!({"from": id_arg(args, "from")?, "to": id_arg(args, "to")?, "label": str_arg(args, "label").unwrap_or("")})),
            "unlink" => self.exec("edge.delete", json!({"id": id_arg(args, "id")?})),
            "layout" => match str_arg(args, "kind") {
                Some("tree") => self.exec("layout.tree", json!({})),
                Some("radial") => self.exec("layout.radial", json!({})),
                _ => Err("`kind` must be tree or radial".into()),
            },
            "export" => {
                let format = str_arg(args, "format").ok_or("`format` is required")?;
                Ok(json!({"format": format, "text": self.render(format)?}))
            }
            "list_commands" => {
                Ok(Value::Array(COMMANDS.iter().map(|c| json!({"id": c.id, "label": c.label, "params": c.params, "undoable": c.undoable})).collect()))
            }
            "run_command" => {
                let command = str_arg(args, "command").ok_or("`command` is required")?;
                let params = args.get("params").cloned().unwrap_or_else(|| json!({}));
                self.exec(command, params)
            }
            other => Err(format!("unknown tool {other:?}")),
        }
    }

    /// Opens a map or imports a file. Maps, FreeMind/Freeplane, OPML and Markdown files are then
    /// saved back to the same file in the same format; other imports need a path to save.
    pub fn open_file(&mut self, path: &str) -> Result<&'static str, String> {
        let len = std::fs::metadata(path).map_err(|e| format!("{path}: {e}"))?.len();
        if len > formats::MAX_FILE_BYTES as u64 {
            return Err(format!("{path}: larger than {} MB", formats::MAX_FILE_BYTES / (1024 * 1024)));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let (map, format) = formats::import_bytes(path, &bytes).map_err(|e| format!("{path}: {e}"))?;
        self.session.replace_map(map);
        let saves_as = match format {
            formats::Format::IdeaCraft => Some("ideacraft"),
            formats::Format::FreeMind => Some("freemind"),
            formats::Format::Opml => Some("opml"),
            formats::Format::Markdown => Some("markdown"),
            _ => None,
        };
        self.file = saves_as.map(|f| (path.to_string(), f));
        Ok(format.name())
    }

    fn style(&mut self, id: u64, args: &Value) -> Result<(), String> {
        if let Some(c) = str_arg(args, "color") {
            self.exec("node.setColor", json!({"id": id, "color": c}))?;
        }
        if let Some(s) = str_arg(args, "shape") {
            self.exec("node.setShape", json!({"id": id, "shape": s}))?;
        }
        Ok(())
    }

    /// Handles one JSON-RPC message; `None` for notifications.
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned()?;
        let method = msg.get("method").and_then(Value::as_str).unwrap_or_default();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).filter(|v| SUPPORTED_VERSIONS.contains(v)).unwrap_or(PROTOCOL_VERSION),
                "capabilities": {"tools": {"listChanged": false}, "resources": {"listChanged": false, "subscribe": false}},
                "serverInfo": {"name": "ideacraft", "title": "IdeaCraft", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                Ok(match self.call(name, &args) {
                    Ok(v) => {
                        let text = match &v {
                            Value::Object(o) if o.len() == 1 && o.values().all(Value::is_string) => {
                                o.values().filter_map(Value::as_str).collect::<String>()
                            }
                            other => serde_json::to_string_pretty(other).unwrap_or_default(),
                        };
                        let structured = if v.is_object() { v } else { json!({"result": v}) };
                        json!({"content": [{"type": "text", "text": text}], "structuredContent": structured, "isError": false})
                    }
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                })
            }
            "resources/list" => Ok(json!({"resources": [
                {"uri": MAP_URI, "name": "map", "title": "Open map (outline)", "description": "Title, ideas with their [id] and links", "mimeType": "text/markdown"},
                {"uri": MAP_JSON_URI, "name": "map.json", "title": "Open map (IdeaCraft JSON)", "description": "The full map: positions, colours, shapes, links", "mimeType": "application/json"},
            ]})),
            "resources/templates/list" => Ok(json!({"resourceTemplates": []})),
            "resources/read" => match params.get("uri").and_then(Value::as_str) {
                Some(MAP_URI) => Ok(json!({"contents": [{"uri": MAP_URI, "mimeType": "text/markdown", "text": outline(self.map())}]})),
                Some(MAP_JSON_URI) => Ok(json!({"contents": [{"uri": MAP_JSON_URI, "mimeType": "application/json", "text": self.map().to_json()}]})),
                _ => Err((-32002, "resource not found".to_string())),
            },
            "prompts/list" => Ok(json!({"prompts": []})),
            _ => Err((-32601, format!("method not found: {}", method.chars().take(64).collect::<String>()))),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, message)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
        })
    }

    /// Serves until `input` ends: one JSON-RPC message per line in, one response per line out.
    pub fn serve(&mut self, input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let reply = if line.len() > MAX_LINE {
                Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "request too large"}}))
            } else {
                match serde_json::from_str::<Value>(&line) {
                    Ok(Value::Array(batch)) => {
                        let replies: Vec<Value> = batch.iter().filter_map(|m| self.handle(m)).collect();
                        (!replies.is_empty()).then_some(Value::Array(replies))
                    }
                    Ok(msg) => self.handle(&msg),
                    Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
                }
            };
            if let Some(r) = reply {
                writeln!(output, "{r}")?;
                output.flush()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(s: &mut Server, name: &str, args: Value) -> Value {
        let r = s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}})).unwrap();
        assert_eq!(r["result"]["isError"], false, "{name}: {r}");
        r["result"]["structuredContent"].clone()
    }

    #[test]
    fn handshake_and_tool_list() {
        let mut s = Server::new();
        let init = s.handle(&json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}})).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "ideacraft");
        assert!(s.handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none(), "notifications get no reply");
        let list = s.handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).unwrap();
        let tools = list["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 20);
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
            assert!(t["description"].as_str().is_some_and(|d| !d.is_empty()));
        }
        let missing = s.handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "nope"})).unwrap();
        assert_eq!(missing["error"]["code"], -32601);
    }

    #[test]
    fn an_agent_builds_a_concept_map() {
        let mut s = Server::new();
        call(&mut s, "new_map", json!({"title": "Photosynthesis"}));
        let root = call(&mut s, "add_idea", json!({"label": "Photosynthesis", "shape": "pill", "color": "#ffe08a"}))["id"].as_u64().unwrap();
        let ids = call(&mut s, "add_ideas", json!({"parent": root, "labels": ["Light", "Water", "Carbon dioxide", "Glucose"]}))["ids"].clone();
        let (light, glucose) = (ids[0].as_u64().unwrap(), ids[3].as_u64().unwrap());
        call(&mut s, "link_ideas", json!({"from": light, "to": glucose, "label": "powers the making of"}));
        call(&mut s, "update_idea", json!({"id": glucose, "label": "Glucose (sugar)", "color": "#b1ecd8"}));
        call(&mut s, "layout", json!({"kind": "radial"}));
        let outline = call(&mut s, "get_map", json!({}))["outline"].as_str().unwrap().to_string();
        assert!(outline.contains(&format!("- [{root}] Photosynthesis")), "{outline}");
        assert!(outline.contains("  - ") && outline.contains("powers the making of"));
        let svg = call(&mut s, "export", json!({"format": "svg"}))["text"].as_str().unwrap().to_string();
        assert!(svg.contains("Glucose (sugar)"));
        call(&mut s, "run_command", json!({"command": "edit.undo"}));
        assert!(s.map().nodes().all(|n| n.label != "Glucose (sugar)") || s.map().node_count() == 5);

        let dir = std::env::temp_dir().join(format!("ideacraft-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("photo.ideacraft.json");
        call(&mut s, "save_map", json!({"path": file.to_str().unwrap()}));
        call(&mut s, "save_map", json!({"path": dir.join("photo.md").to_str().unwrap()}));
        let mut t = Server::new();
        let opened = call(&mut t, "open_map", json!({"path": dir.join("photo.md").to_str().unwrap()}));
        assert!(opened["outline"].as_str().unwrap().contains("Carbon dioxide"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn templates_imports_and_errors() {
        let mut s = Server::new();
        let list = call(&mut s, "list_templates", json!({"language": "it"}));
        assert!(list["result"].as_array().unwrap().iter().any(|t| t["name"] == "Analisi SWOT"));
        call(&mut s, "new_map", json!({"template": "swot", "language": "it"}));
        assert_eq!(s.map().title(), "Analisi SWOT");
        let r = call(&mut s, "import_text", json!({"text": "mindmap\n  root((Trip))\n    Pack\n    Route"}));
        assert_eq!(r["format"], "Mermaid");
        assert_eq!(s.map().node_count(), 3);
        for (name, args) in [
            ("add_idea", json!({})),
            ("delete_idea", json!({"id": 9999})),
            ("link_ideas", json!({"from": "x"})),
            ("layout", json!({"kind": "spiral"})),
            ("open_map", json!({"path": "/definitely/not/here.mm"})),
            ("save_map", json!({})),
            ("nope", json!({})),
        ] {
            let r = s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}})).unwrap();
            assert_eq!(r["result"]["isError"], true, "{name} should fail cleanly: {r}");
        }
        assert_eq!(s.map().node_count(), 3, "failed tools change nothing");
    }

    #[test]
    fn an_agent_draws_a_network_diagram() {
        let mut s = Server::new();
        call(&mut s, "new_map", json!({"title": "Branch office"}));
        let stencils = call(&mut s, "list_stencils", json!({"category": "network", "language": "it"}));
        assert!(stencils["result"].as_array().unwrap().iter().all(|st| st["id"].as_str().unwrap().starts_with("net.")));
        let fw = call(
            &mut s,
            "add_shape",
            json!({"kind": "net.firewall", "label": "FW", "x": 0, "y": 0, "props": {"ip": "10.0.0.1", "status": "up"}}),
        )["id"]
            .as_u64()
            .unwrap();
        let sw = call(&mut s, "add_shape", json!({"kind": "net.switch", "label": "SW", "x": 300, "y": 0}))["id"].as_u64().unwrap();
        let pc = call(&mut s, "add_shape", json!({"kind": "net.pc", "x": 600, "y": 40, "w": 140, "h": 100}))["id"].as_u64().unwrap();
        let link = call(&mut s, "link_ideas", json!({"from": fw, "to": sw}))["id"].as_u64().unwrap();
        call(
            &mut s,
            "style_link",
            json!({"id": link, "route": "orthogonal", "line": "dashed", "arrows": "both", "color": "#ff0000", "width": 2, "label": "1 Gbit"}),
        );
        let out = call(&mut s, "align", json!({"ids": [fw, sw, pc], "edge": "top", "distribute": "x", "snap": true}))["outline"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(out.contains("net.firewall") && out.contains("ip=10.0.0.1") && out.contains("dashed, orthogonal, arrows both"), "{out}");
        assert_eq!(s.map().mode(), ideacraft_engine::core::Mode::Diagram);
        let drawio = call(&mut s, "export", json!({"format": "drawio"}))["text"].as_str().unwrap().to_string();
        assert!(drawio.contains("<mxfile") && drawio.contains("orthogonalEdgeStyle"));
        let r = call(&mut s, "import_text", json!({"text": drawio}));
        assert_eq!(r["format"], "draw.io / diagrams.net");
        assert_eq!(s.map().node_count(), 3);
        call(&mut s, "set_mode", json!({"mode": "map"}));
        assert_eq!(s.map().mode(), ideacraft_engine::core::Mode::Map);
        for args in [json!({"kind": "net.nope"}), json!({})] {
            let r =
                s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "add_shape", "arguments": args}})).unwrap();
            assert_eq!(r["result"]["isError"], true);
        }
    }

    #[test]
    fn opml_and_freemind_files_open_edit_and_save_back_in_their_format() {
        let dir = std::env::temp_dir().join(format!("ideacraft-mcp-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mm = dir.join("timeline.mm");
        std::fs::write(&mm, "<map version=\"1.0.1\">\n<node TEXT=\"Company 2007 → today\">\n<node TEXT=\"2007 · Start\" POSITION=\"right\">\n<node TEXT=\"Founded\"/>\n</node>\n<node TEXT=\"2008-2010\" POSITION=\"left\"/>\n</node>\n</map>\n").unwrap();
        let opml = dir.join("timeline.opml");
        std::fs::write(&opml, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\"><head><title>Timeline</title></head><body>\n<outline text=\"Company\"><outline text=\"2007\"/></outline></body></opml>").unwrap();
        for (path, root_label) in [(&mm, "Company 2007 → today"), (&opml, "Company")] {
            let mut s = Server::new();
            let opened = call(&mut s, "open_file", json!({"path": path.to_str().unwrap()}));
            assert_eq!(opened["saves_back"], true);
            let root = s.map().roots()[0];
            assert_eq!(s.map().node(root).unwrap().label, root_label);
            call(&mut s, "add_idea", json!({"label": "Added by an agent", "parent": root.0}));
            call(&mut s, "save", json!({}));
            let mut t = Server::new();
            t.open_file(path.to_str().unwrap()).unwrap();
            assert!(t.map().nodes().any(|n| n.label == "Added by an agent"), "{path:?} saved back");
            let text = std::fs::read_to_string(path).unwrap();
            let expected = if path.extension().unwrap() == "mm" { "<map" } else { "<opml" };
            assert!(text.trim_start().starts_with(expected) || text.contains(expected), "{path:?} keeps its format");
        }
        let res = Server::new().handle(&json!({"jsonrpc": "2.0", "id": 9, "method": "resources/read", "params": {"uri": MAP_URI}})).unwrap();
        assert!(res["result"]["contents"][0]["text"].as_str().unwrap().starts_with("# "));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn serve_speaks_line_delimited_json_rpc() {
        let input = concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            "not json\n",
            "\n",
            "[{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}]\n",
        );
        let mut out = Vec::new();
        Server::new().serve(input.as_bytes(), &mut out).unwrap();
        let lines: Vec<Value> = String::from_utf8(out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["id"], 1);
        assert_eq!(lines[1]["error"]["code"], -32700);
        assert_eq!(lines[2][0]["id"], 2);
    }
}
