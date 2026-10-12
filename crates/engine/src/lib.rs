//! IdeaCraft's engine (layer L5): a [`Session`] owns the map, the selection and the undo history,
//! and everything a user can do is a command run through [`Session::execute`]. The UI, the CLI
//! and future automation (MCP, control channel) all reach the same commands.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod cmd;
mod params;

use std::panic::{AssertUnwindSafe, catch_unwind};

pub use cmd::{COMMANDS, CommandInfo, command};
pub use ideacraft_core as core;
use ideacraft_core::{Map, NodeId};
pub use ideacraft_formats as formats;
use serde_json::Value;

/// Most undo steps kept.
pub const UNDO_LIMIT: usize = 200;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Model(#[from] ideacraft_core::Error),
    #[error("unknown command {0:?}")]
    UnknownCommand(String),
    #[error("{0}")]
    Param(String),
    #[error(transparent)]
    Import(#[from] ideacraft_formats::Error),
    #[error("nothing to {0}")]
    Nothing(&'static str),
    #[error("internal error in {0}; the map was restored")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// An open map and its editing state.
#[derive(Debug, Default)]
pub struct Session {
    map: Map,
    selection: Vec<NodeId>,
    undo: Vec<(Map, Vec<NodeId>)>,
    redo: Vec<(Map, Vec<NodeId>)>,
    dirty: bool,
}

impl Session {
    pub fn new(map: Map) -> Self {
        Self { map, ..Self::default() }
    }

    pub fn map(&self) -> &Map {
        &self.map
    }

    pub fn selection(&self) -> &[NodeId] {
        &self.selection
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Changed since it was opened or last saved.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Marks the map as unsaved (an imported file that was never saved as a map).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Runs a command. A command that fails leaves the map and selection as they were; a command
    /// that changes the map becomes one undo step. Panics are caught and the map restored (a
    /// safety net: commands must not panic in the first place).
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        let info = command(id).ok_or_else(|| Error::UnknownCommand(id.chars().take(64).collect()))?;
        let before = (self.map.clone(), self.selection.clone());
        let outcome = catch_unwind(AssertUnwindSafe(|| cmd::run(self, info.id, params)));
        match outcome {
            Ok(Ok(value)) => {
                self.selection.retain(|n| self.map.node(*n).is_some());
                if info.undoable && self.map != before.0 {
                    self.undo.push(before);
                    if self.undo.len() > UNDO_LIMIT {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                    self.dirty = true;
                }
                Ok(value)
            }
            Ok(Err(e)) => {
                (self.map, self.selection) = before;
                Err(e)
            }
            Err(_) => {
                (self.map, self.selection) = before;
                Err(Error::Internal(info.id.to_string()))
            }
        }
    }

    fn undo_step(&mut self) -> Result<()> {
        let (map, sel) = self.undo.pop().ok_or(Error::Nothing("undo"))?;
        let current = (std::mem::replace(&mut self.map, map), std::mem::replace(&mut self.selection, sel));
        self.redo.push(current);
        self.dirty = true;
        Ok(())
    }

    fn redo_step(&mut self) -> Result<()> {
        let (map, sel) = self.redo.pop().ok_or(Error::Nothing("redo"))?;
        let current = (std::mem::replace(&mut self.map, map), std::mem::replace(&mut self.selection, sel));
        self.undo.push(current);
        self.dirty = true;
        Ok(())
    }

    /// Replaces the whole map (opening a file): history is cleared.
    pub fn replace_map(&mut self, map: Map) {
        *self = Self::new(map);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn every_command_survives_empty_params() {
        for c in COMMANDS {
            let mut s = Session::new(Map::sample());
            let _ = s.execute(c.id, &json!({}));
            let _ = s.execute(c.id, &Value::Null);
            let _ = s.execute(c.id, &json!({"id": -1, "ids": "x", "x": "nan", "label": 7, "color": [], "parent": {}}));
            let mut empty = Session::default();
            let _ = empty.execute(c.id, &json!({}));
        }
    }

    #[test]
    fn commands_are_unique_and_documented() {
        for (i, c) in COMMANDS.iter().enumerate() {
            assert!(COMMANDS.iter().take(i).all(|o| o.id != c.id), "duplicate {}", c.id);
            assert!(!c.label.is_empty() && !c.menu.is_empty(), "{} needs a label and menu", c.id);
        }
    }

    #[test]
    fn undo_redo_and_failed_commands() {
        let mut s = Session::default();
        let id = s.execute("node.add", &json!({"label": "Root"})).unwrap()["id"].as_u64().unwrap();
        assert_eq!(s.selection(), [NodeId(id)]);
        s.execute("node.addChild", &json!({})).unwrap();
        assert_eq!(s.map().node_count(), 2);
        assert!(s.is_dirty());
        assert!(s.execute("node.setColor", &json!({"color": "#zzz"})).is_err());
        assert_eq!(s.map().node_count(), 2, "a failed command changes nothing");
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.map().node_count(), 1);
        s.execute("edit.redo", &json!({})).unwrap();
        assert_eq!(s.map().node_count(), 2);
        s.execute("selection.set", &json!({"ids": [id]})).unwrap();
        s.execute("node.delete", &json!({})).unwrap();
        assert_eq!(s.map().node_count(), 0, "deleting removes the branch");
        assert!(s.execute("edit.redo", &json!({})).is_err());
        assert!(s.execute("no.such", &json!({})).is_err());

        s.execute("map.newFromTemplate", &json!({"template": "swot"})).unwrap();
        assert_eq!(s.map().title(), "SWOT analysis");
        let md = s.execute("map.export", &json!({"format": "markdown"})).unwrap()["text"].as_str().unwrap().to_string();
        let r = s.execute("map.importText", &json!({"text": md})).unwrap();
        assert_eq!(r["format"], "Markdown / text outline");
        assert!(s.map().node_count() > 8);
        assert!(s.execute("map.newFromTemplate", &json!({"template": "x"})).is_err());
        s.execute("map.newFromTemplate", &json!({"template": "swot", "language": "it"})).unwrap();
        assert_eq!(s.map().title(), "Analisi SWOT");
        assert!(s.map().nodes().any(|n| n.label == "Punti di forza"));
        assert!(s.execute("map.newFromTemplate", &json!({"template": "swot", "language": "xx"})).is_err());
    }
}
