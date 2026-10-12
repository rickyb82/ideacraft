# Architecture

> **Last updated:** 2026-10-12

| Crate | Layer | Role |
|---|---|---|
| `crates/core` (`ideacraft-core`) | L0 | The map model: ideas (`Node`), tree parents, free links (`Edge`), colours, shapes, layouts, limits and the `.ideacraft.json` format with lenient, repairing reads. |
| `crates/formats` (`ideacraft-formats`) | L2 | Import (FreeMind/Freeplane, XMind, CmapTools CXL, OPML, Markdown, Mermaid), export (Markdown, OPML, FreeMind) and the built-in templates. A bounded XML tree sits on `quick-xml`; XMind zips are read with `zip`. |
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

## Imports

| Format | What is read |
|---|---|
| FreeMind / Freeplane `.mm` | node text (and rich HTML text), folding, background colour, arrow links with labels |
| XMind `.xmind` | first sheet: topics, folding, fill colour, relationships with titles; `content.json` and legacy `content.xml` |
| CmapTools `.cxl` | concepts with positions and colours; linking phrases become labelled links |
| OPML | nested outlines |
| Markdown / text | headings, nested bullets and numbered lists, plain indented lines, `A → B (label)` links |
| Mermaid | `mindmap` (indentation, shapes) and simple `flowchart`/`graph` (unlabelled edges form the tree, labelled ones become links) |
