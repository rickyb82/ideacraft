# IdeaCraft roadmap

> **Last updated:** 2026-10-12 · **Stage:** 0.1 preview

IdeaCraft aims to be the open, fast, scriptable place to think in maps: concept maps, mind maps
and idea boards, with the polish of commercial canvas tools and none of the lock-in.

## Now (0.1 — done)

- Workspace in the craft layout: `core` → `formats` → `engine` → `ui-egui` → apps.
- Canvas: ideas, tree branches, labelled links, collapse, pan/zoom/fit, light/dark.
- Tree and radial layouts; node colours and shapes.
- 12 original templates with a preview gallery.
- Import FreeMind/Freeplane, XMind, CmapTools CXL, OPML, Markdown, Mermaid; export Markdown,
  OPML, FreeMind; native `.ideacraft.json`.
- Command engine with undo/redo, panic guard, command-sequence fuzzing; headless CLI;
  offscreen snapshot renderer.
- Canvas-tool interface (top bar, left rail with panels, floating selection bar, zoom), icons
  drawn in code, light/dark themes.
- Web app (WebAssembly, trunk) published to GitHub Pages; browser open/save/export, state kept
  in the browser.
- English and Italian interface and templates.

## Next (0.2)

- Marquee selection, multi-select drag, keyboard navigation between ideas (arrow keys).
- Balanced mind-map layout (branches on both sides), org-chart and timeline layouts.
- Text formatting in ideas (bold, lists), notes, links to URLs, icons/emoji, images.
- Free shapes and sticky notes outside the tree (board mode).
- Themes: whole-map colour schemes, fonts from `storytold/craft-fonts`.
- More interface languages.

## Later

- Real-time collaboration (CRDT), comments.
- Presentation mode (walk through branches), PNG/SVG/PDF export.
- MCP server and a local control channel for agents, like the other craft apps.
- More importers: MindManager-style XML, GraphML, Obsidian canvas.

## Progress log

| Date | Change |
|---|---|
| 2026-10-12 | 0.1: skeleton, canvas, layouts, 12 templates, 6 import formats, 3 export formats, CLI |
| 2026-10-12 | Italian interface and templates; new canvas-tool interface; web app on GitHub Pages |
