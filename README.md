# IdeaCraft

**Design your own digital map.** IdeaCraft is an open-source, Rust-native canvas for dynamic
concept maps and mind maps — a free, fast alternative to closed whiteboard and mind-mapping apps.
It belongs to the *craft* family of open creative tools (GridCraft, PdfCraft, …) and shares their
conventions: everything is a command, nothing crashes, and every file stays readable.

> Status: **0.1, early preview.** The canvas, templates, import/export and the command engine
> work; see [ROADMAP.md](ROADMAP.md) for what comes next.

## What it does

- **Infinite canvas** — pan, zoom around the pointer, fit to window, light and dark themes.
- **Ideas and structure** — a parent/child tree per map (Tab adds a child, Enter a sibling),
  free labelled links between any two ideas (Shift+drag, or the Link tool), collapsible branches.
- **Dynamic layouts** — tree and radial layouts, re-run any time; drag an idea up or down and
  re-layout to reorder it.
- **Styling** — node fills from an original palette and four shapes (rounded, rect, ellipse, pill).
- **12 built-in templates** — Brainstorming, Concept map, Project plan, SWOT analysis, Decision
  making, Meeting notes, Book summary, Cause and effect, Weekly planner, Product roadmap,
  Customer journey, Lesson plan. All are original and defined in code.
- **Import** — FreeMind / Freeplane (`.mm`), XMind (`.xmind`, current and legacy), CmapTools
  (`.cxl`), OPML, Markdown / indented outlines, Mermaid (`mindmap`, simple `flowchart`).
  Open a file or drop it on the window.
- **Export** — Markdown outline, OPML, FreeMind `.mm`, and IdeaCraft's own plain-JSON format
  (`.ideacraft.json`).
- **Undo/redo** for every change; the map is restored if a command ever fails.
- **Headless CLI** for scripts and agents: run any command, convert between formats, list
  templates.

## Run it

```sh
cargo run --release -p ideacraft                       # opens the template gallery
cargo run --release -p ideacraft -- --template swot    # start from a template
cargo run --release -p ideacraft -- notes.mm           # open or import a file
```

Linux needs the usual windowing libraries (X11 or Wayland, `libxkbcommon`).

### Keyboard

| Key | Action |
|---|---|
| Double-click | new idea (empty canvas) / edit text (idea) |
| Tab / Enter | add child / add sibling, then type |
| F2 | edit text |
| Delete | delete the idea and its branch |
| Space | collapse / expand branch |
| Shift+drag | link two ideas |
| Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z | undo, redo |
| Cmd/Ctrl+L, Cmd/Ctrl+Shift+L | tree layout, radial layout |
| Cmd/Ctrl+0 | fit the map in the window |
| Cmd/Ctrl+O, S, Shift+S, N | open/import, save, save as, new |

### Headless

```sh
cargo run -p ideacraft-cli -- templates
cargo run -p ideacraft-cli -- commands
cargo run -p ideacraft-cli -- convert plan.xmind plan.ideacraft.json
cargo run -p ideacraft-cli -- run --template brainstorm \
    --cmd 'node.addChild={"id":1,"label":"Ask a teacher"}' --cmd 'layout.radial' --out ideas.md
```

### Look at the UI without a window

```sh
cargo run -p ideacraft-ui-egui --example snapshot -- --template swot out.png [--dark]
```

## Develop

```sh
cargo xtask ci        # fmt, clippy -D warnings, tests, layer check
```

Read [CLAUDE.md](CLAUDE.md) (rules for contributors and agents) and
[docs/architecture.md](docs/architecture.md) first.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Every
asset is original; see [ATTRIBUTION.md](ATTRIBUTION.md).
