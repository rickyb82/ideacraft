//! Italian interface labels.

pub(super) fn translate(text: &str) -> Option<&'static str> {
    Some(match text {
        "Map title" => "Titolo della mappa",
        "New" => "Nuova",
        "Templates…" => "Modelli…",
        "New map from a template" => "Nuova mappa da un modello",
        "Open…" => "Apri…",
        "Open a map or import FreeMind, Freeplane, XMind, CmapTools, OPML, Markdown or Mermaid (Cmd+O)" => {
            "Apri una mappa o importa FreeMind, Freeplane, XMind, CmapTools, OPML, Markdown o Mermaid (Cmd+O)"
        }
        "Save" => "Salva",
        "Save (Cmd+S)" => "Salva (Cmd+S)",
        "Export" => "Esporta",
        "+ Idea" => "+ Idea",
        "+ Child" => "+ Sottoidea",
        "+ Sibling" => "+ Stesso livello",
        "Delete" => "Elimina",
        "Collapse" => "Comprimi",
        "Link" => "Collega",
        "Drag from one idea to another to link them (or hold Shift while dragging)" => {
            "Trascina da un’idea all’altra per collegarle (oppure tieni premuto Maiusc mentre trascini)"
        }
        "Tree" => "Albero",
        "Radial" => "Radiale",
        "Fit" => "Adatta",
        "Fit the map in the window (Cmd+0)" => "Adatta la mappa alla finestra (Cmd+0)",
        "Undo" => "Annulla",
        "Redo" => "Ripeti",
        "Fill" => "Riempimento",
        "Shape" => "Forma",
        "Language" => "Lingua",
        "rounded" => "arrotondata",
        "rect" => "rettangolo",
        "ellipse" => "ellisse",
        "pill" => "pillola",
        "{ideas} ideas · {links} links · {selected} selected" => "{ideas} idee · {links} collegamenti · {selected} selezionate",
        "Double-click: new idea / edit · Tab: child · Enter: sibling · Shift+drag: link · Space: collapse" => {
            "Doppio clic: nuova idea / modifica · Tab: sottoidea · Invio: stesso livello · Maiusc+trascina: collega · Spazio: comprimi"
        }
        "New from template" => "Nuova da modello",
        "Pick a starting point. Everything stays editable." => "Scegli da dove partire. Tutto resta modificabile.",
        "Markdown outline (.md)" => "Schema Markdown (.md)",
        "Opened {name}" => "Aperto {name}",
        "Imported {name} ({format})" => "Importato {name} ({format})",
        "Couldn't open {name}: {error}" => "Impossibile aprire {name}: {error}",
        "file too large" => "file troppo grande",
        "Exported {name}" => "Esportato {name}",
        "Couldn't export {name}: {error}" => "Impossibile esportare {name}: {error}",
        "Saved {name}" => "Salvato {name}",
        "Couldn't save {name}: {error}" => "Impossibile salvare {name}: {error}",
        "Untitled map" => "Mappa senza titolo",
        "Idea" => "Idea",
        "Maps and outlines" => "Mappe e schemi",
        "All files" => "Tutti i file",
        "IdeaCraft map" => "Mappa IdeaCraft",
        "Markdown / text outline" => "Markdown / schema di testo",
        "select a node or give `id`" => "Seleziona prima un’idea",
        "New Map" => "Nuova mappa",
        "Add Idea" => "Aggiungi idea",
        "Add Child Idea" => "Aggiungi sottoidea",
        "Add Sibling Idea" => "Aggiungi idea allo stesso livello",
        "Collapse/Expand Branch" => "Comprimi/espandi ramo",
        "Tree Layout" => "Layout ad albero",
        "Radial Layout" => "Layout radiale",
        _ => return None,
    })
}
