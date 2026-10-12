# Architecture

> **Last updated:** 2026-10-12

| Crate | Layer | Role |
|---|---|---|
| `crates/core` (`ideacraft-core`) | L0 | The map model: ideas (`Node`: tree parent, colour, 14 shapes, optional size, stencil kind, properties, z-order), links (`Edge` with `EdgeStyle`: line, arrows, route, colour, width), map `Mode` (map or diagram), layouts, connector routing (`route.rs`, shared by canvas, SVG and draw.io), the stencil library (`stencil.rs`: 45 original glyphs as strokes on a unit square), limits and the `.ideacraft.json` format with lenient, repairing reads. |
| `crates/formats` (`ideacraft-formats`) | L2 | Import (FreeMind/Freeplane, XMind, CmapTools CXL, OPML, Markdown, Mermaid, draw.io plain or deflate-compressed), export (Markdown, OPML, FreeMind, draw.io, SVG), the outline templates and the diagram templates built in code (`diagrams.rs`). A bounded XML tree sits on `quick-xml`; XMind zips are read with `zip`; compressed draw.io diagrams with `flate2`. |
| `crates/engine` (`ideacraft-engine`) | L5 | `Session` (map, selection, undo/redo) and the command catalog. `Session::execute` validates, runs, records undo and restores the map on failure or panic. |
| `crates/ui-egui` (`ideacraft-ui-egui`) | L6 | egui front end: top bar and status bar (`chrome.rs`), left rail and panels (`sidebar.rs`), canvas with the floating selection bar and zoom (`canvas.rs`), icons drawn in code (`icons.rs`), theme, i18n (English, Italian); `examples/snapshot.rs` renders offscreen. |
| `crates/mcp` (`ideacraft-mcp`) | L6 | Model Context Protocol server over stdio: 16 tools on top of the engine commands ([mcp.md](mcp.md)). |
| `apps/ideacraft` | L7 | Desktop app (eframe), file dialogs (`rfd`), drag-and-drop, saved language. |
| `apps/ideacraft-web` | L7 | Browser app (eframe on wasm32, built with trunk): file picker, downloads, state in browser storage, `?lang=` and `?template=`. |
| `apps/ideacraft-cli` | L7 | Headless: commands, templates, convert, outline, and `mcp` (the MCP server). |
| `xtask` | — | `cargo xtask ci` / `layers`. |

## The map

Each idea has an optional parent, so a map is a forest of trees; layouts, collapsing and keyboard
editing follow the trees. Free links join any two ideas and carry an optional label — the linking
phrase of a concept map ("causes", "is part of"). Positions are map coordinates (y down); layouts
keep each root where it is and place everything below it.

## Files

`.ideacraft.json` is pretty-printed JSON (`format`, `version`, `title`, `nextId`, `nodes`,
`edges`). Readers repair what they can — duplicate ids, dangling parents and links, parent
cycles, out-of-range coordinates, over-long labels — and only refuse unreadable or newer files.

## Diagram mode

A map's `mode` is `map` (tree layouts, concept-map look) or `diagram`. In diagram mode the canvas
snaps to a 20-unit grid, drag on empty space is a marquee, a single selected shape shows corner
resize handles, and links draw with their own style (map mode draws default-styled links in the
dashed concept-map look). Shapes added from the library get the stencil's shape, size, colour and
`kind`; the stencil's glyph is drawn above the label by the canvas (`canvas.rs`) and the SVG export
from the same unit-square strokes. Containers (`Shape::Container`) draw first and are hit last, so
shapes placed on them stay clickable. Properties live on the node (`props`); the `status` property
drives the indicator light (`Node::status_light`).

## Imports

| Format | What is read |
|---|---|
| FreeMind / Freeplane `.mm` | node text (and rich HTML text), folding, background colour, arrow links with labels |
| XMind `.xmind` | first sheet: topics, folding, fill colour, relationships with titles; `content.json` and legacy `content.xml` |
| CmapTools `.cxl` | concepts with positions and colours; linking phrases become labelled links |
| OPML | nested outlines |
| Markdown / text | headings, nested bullets and numbered lists, plain indented lines, `A → B (label)` links |
| Mermaid | `mindmap` (indentation, shapes) and simple `flowchart`/`graph` (unlabelled edges form the tree, labelled ones become links) |
| draw.io `.drawio` | first diagram: vertices (geometry, shape from the style, fill colour, swimlanes as containers), edges (source/target, label, dashed, arrows, orthogonal/curved, stroke colour and width); IdeaCraft stencils round-trip through an `ideacraftKind` style key |
