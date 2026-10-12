# MCP server

> **Last updated:** 2026-10-12

IdeaCraft speaks the [Model Context Protocol](https://modelcontextprotocol.io) so AI agents
(Claude Code, Claude Desktop and any other MCP client) can build and edit concept maps directly:
`ideacraft-cli mcp` serves MCP over stdio (newline-delimited JSON-RPC 2.0, protocol
`2025-06-18`). The server is `crates/mcp` (`ideacraft-mcp`); every edit goes through the engine's
commands, so it is validated and undoable, and a failed call never changes the map.

## Install and connect

```sh
cargo install --path apps/ideacraft-cli          # puts ideacraft-cli on your PATH
claude mcp add ideacraft -- ideacraft-cli mcp     # Claude Code
```

Claude Desktop (`claude_desktop_config.json`):

```json
{ "mcpServers": { "ideacraft": { "command": "ideacraft-cli", "args": ["mcp"] } } }
```

Start with a file, as with the other craft apps (`gridcraft-cli mcp --in …`):

```sh
ideacraft-cli mcp --in timeline.mm        # or an .opml, .md, .ideacraft.json, .xmind, .cxl …
```

`.ideacraft.json`, FreeMind/Freeplane `.mm`, `.opml` and `.md` files are **saved back in their
own format**: `save_map` without a path rewrites the file that was opened. XMind, CmapTools and
Mermaid files are read only; give `save_map` a path to save them.

Then ask, for example: *"Make a concept map of the water cycle in Italian, with labelled links
between the stages, lay it out radially and save it as ciclo.ideacraft.json and ciclo.svg."*

## Tools

| Tool | What it does |
|---|---|
| `list_templates` | The built-in templates (`language`: en, it) |
| `new_map` | New empty map, or from a `template` (with `language`, optional `title`) |
| `open_map` (also `open`, `open_file`) | Open an IdeaCraft map or import FreeMind/Freeplane, XMind, CmapTools CXL, OPML, Markdown, Mermaid |
| `import_text` | Replace the map with text in any importable format (detected when `format` is omitted) |
| `save_map` (also `save`, `save_file`) | Save; without a path, back to the opened file in its format; otherwise the extension decides (`.ideacraft.json`, `.md`, `.opml`, `.mm`, `.svg`) |
| `get_map` | Outline with each idea's `[id]` and the links, or the full JSON |
| `add_idea` / `add_ideas` | Add one idea (with colour, shape) or several children under a parent |
| `update_idea` | Text, colour, shape, collapsed, or move under another parent |
| `delete_idea` | Remove an idea and its branch |
| `link_ideas` / `unlink` | Labelled concept-map links between any two ideas |
| `layout` | `tree` or `radial` |
| `export` | `markdown`, `opml`, `freemind`, `drawio` (for draw.io / Visio), `svg` (a picture of the map) or `ideacraft` JSON |
| `list_stencils` | The shape library for diagrams (`category`: basic, flowchart, network, cloud, people; `language`) |
| `add_shape` | A diagram shape from a stencil (`kind`, e.g. `net.router`, `flow.decision`, `basic.container`) with label, position, size and `props` (ip, vlan, status…); switches to diagram mode |
| `style_link` | Line (solid/dashed/dotted), arrows (none/end/start/both), route (straight/orthogonal/curved), colour, width, label of a link |
| `align` | Align several shapes on an edge, distribute them along x or y, snap them to the grid |
| `set_mode` | `map` (mind map) or `diagram` |
| `list_commands` / `run_command` | Every engine command, e.g. `edit.undo`, `node.moveBy` |

Resources: `ideacraft://map` (the outline with ids, Markdown) and `ideacraft://map.json` (the
full map). Protocol revisions 2025-06-18, 2025-03-26 and 2024-11-05 are accepted.

For diagrams, ask for example: *"Draw the network of a branch office: internet, firewall, a core
switch, two servers in a 'Server room' container and six workstations, with IP addresses and
status, orthogonal connectors labelled with the VLAN, then export it as branch.drawio."*
`get_map` shows each shape's stencil, size, position and properties, and each link's style.

Ids are positive integers shown as `[id]` by `get_map`. Main branches get their own colour
automatically; deeper ideas take their branch's colour.

## Scope

The server edits its own map in memory and reads or writes files on the machine where it runs,
with the permissions of the user who started it. It does not drive a running desktop or web
app yet (a live control channel, as in GridCraft, is on the roadmap).
