use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::limits::{MAX_COORD, MAX_DEPTH, MAX_EDGES, MAX_LABEL_CHARS, MAX_NODES};
use crate::{Error, Result, Rgb};

/// Current `.ideacraft.json` format version.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EdgeId(pub u64);

/// A point in map coordinates (logical pixels at 100% zoom; y grows downwards).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Finite and inside ±[`MAX_COORD`]; NaN becomes 0.
    pub fn clamped(self) -> Self {
        let c = |v: f32| {
            if v.is_finite() {
                v.clamp(-MAX_COORD, MAX_COORD)
            } else if v.is_nan() {
                0.0
            } else {
                v.signum() * MAX_COORD
            }
        };
        Self { x: c(self.x), y: c(self.y) }
    }
}

/// How a node is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Rounded,
    Rect,
    Ellipse,
    Pill,
}

impl Shape {
    pub const ALL: [Self; 4] = [Self::Rounded, Self::Rect, Self::Ellipse, Self::Pill];

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name().eq_ignore_ascii_case(name.trim()))
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Rounded => "rounded",
            Self::Rect => "rect",
            Self::Ellipse => "ellipse",
            Self::Pill => "pill",
        }
    }
}

/// A concept: a labelled box on the canvas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    /// Centre of the node.
    pub pos: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeId>,
    #[serde(default)]
    pub color: Rgb,
    #[serde(default)]
    pub shape: Shape,
    /// Children (and everything under them) are hidden.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub collapsed: bool,
}

/// A free, optionally labelled link between two concepts ("causes", "is part of"…).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
}

/// A concept map.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    title: String,
    nodes: BTreeMap<NodeId, Node>,
    edges: BTreeMap<EdgeId, Edge>,
    next_id: u64,
}

impl Default for Map {
    fn default() -> Self {
        Self::new("Untitled map")
    }
}

/// The file layout: plain lists, so files diff well.
#[derive(Serialize, Deserialize)]
struct File {
    format: String,
    version: u32,
    title: String,
    /// Next free id, so ids of deleted ideas are never reused.
    #[serde(default, rename = "nextId")]
    next_id: u64,
    #[serde(default)]
    nodes: Vec<Node>,
    #[serde(default)]
    edges: Vec<Edge>,
}

fn clean_label(label: &str) -> String {
    label.chars().filter(|c| *c == '\n' || !c.is_control()).take(MAX_LABEL_CHARS).collect()
}

impl Map {
    pub fn new(title: &str) -> Self {
        Self { title: clean_label(title), nodes: BTreeMap::new(), edges: BTreeMap::new(), next_id: 1 }
    }

    /// A small built-in example (original content), used by `--sample` and tests.
    pub fn sample() -> Self {
        let mut map = Self::new("IdeaCraft");
        let build = |map: &mut Self| -> Result<()> {
            let root = map.add_node("IdeaCraft", Point::new(0.0, 0.0), None)?;
            map.set_color(root, Rgb(0xff, 0xe0, 0x8a))?;
            map.set_shape(root, Shape::Pill)?;
            let branches = [
                ("Canvas", Rgb(0xb3, 0xd4, 0xff), &["Infinite pan & zoom", "Drag to move", "Double-click to add"][..]),
                ("Structure", Rgb(0xb1, 0xec, 0xd8), &["Parent / child tree", "Free links", "Collapse branches"][..]),
                ("Layouts", Rgb(0xd4, 0xc4, 0xfb), &["Tree", "Radial"][..]),
                ("Open", Rgb(0xff, 0xc2, 0xa8), &["Plain JSON files", "Every action is a command", "CLI"][..]),
            ];
            let mut first_leaf = None;
            let mut last_leaf = None;
            for (i, (label, color, leaves)) in branches.iter().enumerate() {
                let branch = map.add_node(label, Point::new(220.0, i as f32 * 120.0), Some(root))?;
                map.set_color(branch, *color)?;
                for (j, leaf) in leaves.iter().enumerate() {
                    let id = map.add_node(leaf, Point::new(460.0, i as f32 * 120.0 + j as f32 * 40.0), Some(branch))?;
                    map.set_color(id, *color)?;
                    first_leaf.get_or_insert(id);
                    last_leaf = Some(id);
                }
            }
            if let (Some(a), Some(b)) = (first_leaf, last_leaf) {
                map.connect(a, b, "saved in")?;
            }
            crate::layout::apply(map, crate::layout::Layout::Tree);
            Ok(())
        };
        // The sample is tiny and well-formed; should it ever fail, an empty map is still valid.
        if build(&mut map).is_err() {
            map = Self::new("IdeaCraft");
        }
        map
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = clean_label(title);
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.values()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node> {
        self.nodes.get_mut(&id).ok_or(Error::NoNode(id.0))
    }

    fn fresh_id(&mut self) -> Result<u64> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(Error::Full("ids"))?;
        Ok(id)
    }

    /// Adds a node; `parent` makes it a child in the tree.
    pub fn add_node(&mut self, label: &str, pos: Point, parent: Option<NodeId>) -> Result<NodeId> {
        if self.nodes.len() >= MAX_NODES {
            return Err(Error::Full("nodes"));
        }
        if let Some(p) = parent {
            self.node(p).ok_or(Error::NoNode(p.0))?;
        }
        let id = NodeId(self.fresh_id()?);
        // A new main branch (child of a root) gets the next branch colour; deeper ideas take
        // their parent's colour, so each branch reads as one colour.
        let color = match parent.and_then(|p| self.node(p)) {
            Some(p) if p.parent.is_none() => {
                let branches = self.nodes.values().filter(|n| n.parent == Some(p.id)).count();
                Rgb::BRANCHES.get(branches % Rgb::BRANCHES.len()).copied().unwrap_or_default()
            }
            Some(p) => p.color,
            None => Rgb::default(),
        };
        self.nodes.insert(id, Node { id, label: clean_label(label), pos: pos.clamped(), parent, color, shape: Shape::default(), collapsed: false });
        Ok(id)
    }

    /// Removes a node with its whole branch and every link touching them; returns the removed ids.
    pub fn remove_node(&mut self, id: NodeId) -> Result<Vec<NodeId>> {
        self.node(id).ok_or(Error::NoNode(id.0))?;
        let mut gone = vec![id];
        gone.extend(self.descendants(id));
        let set: BTreeSet<NodeId> = gone.iter().copied().collect();
        for n in &gone {
            self.nodes.remove(n);
        }
        self.edges.retain(|_, e| !set.contains(&e.from) && !set.contains(&e.to));
        Ok(gone)
    }

    pub fn set_label(&mut self, id: NodeId, label: &str) -> Result<()> {
        self.node_mut(id)?.label = clean_label(label);
        Ok(())
    }

    pub fn set_pos(&mut self, id: NodeId, pos: Point) -> Result<()> {
        self.node_mut(id)?.pos = pos.clamped();
        Ok(())
    }

    pub fn set_color(&mut self, id: NodeId, color: Rgb) -> Result<()> {
        self.node_mut(id)?.color = color;
        Ok(())
    }

    pub fn set_shape(&mut self, id: NodeId, shape: Shape) -> Result<()> {
        self.node_mut(id)?.shape = shape;
        Ok(())
    }

    pub fn set_collapsed(&mut self, id: NodeId, collapsed: bool) -> Result<()> {
        self.node_mut(id)?.collapsed = collapsed;
        Ok(())
    }

    /// Moves a node (with its branch) under `parent`, or makes it a root with `None`.
    pub fn set_parent(&mut self, id: NodeId, parent: Option<NodeId>) -> Result<()> {
        self.node(id).ok_or(Error::NoNode(id.0))?;
        if let Some(p) = parent {
            self.node(p).ok_or(Error::NoNode(p.0))?;
            if p == id || self.ancestors(p).contains(&id) {
                return Err(Error::Cycle);
            }
        }
        self.node_mut(id)?.parent = parent;
        Ok(())
    }

    /// Links two nodes; linking a pair that is already linked returns the existing link.
    pub fn connect(&mut self, from: NodeId, to: NodeId, label: &str) -> Result<EdgeId> {
        if from == to {
            return Err(Error::SelfLink);
        }
        self.node(from).ok_or(Error::NoNode(from.0))?;
        self.node(to).ok_or(Error::NoNode(to.0))?;
        if let Some(e) = self.edges.values().find(|e| (e.from, e.to) == (from, to) || (e.from, e.to) == (to, from)) {
            return Ok(e.id);
        }
        if self.edges.len() >= MAX_EDGES {
            return Err(Error::Full("links"));
        }
        let id = EdgeId(self.fresh_id()?);
        self.edges.insert(id, Edge { id, from, to, label: clean_label(label) });
        Ok(id)
    }

    pub fn disconnect(&mut self, id: EdgeId) -> Result<Edge> {
        self.edges.remove(&id).ok_or(Error::NoEdge(id.0))
    }

    pub fn set_edge_label(&mut self, id: EdgeId, label: &str) -> Result<()> {
        self.edges.get_mut(&id).ok_or(Error::NoEdge(id.0))?.label = clean_label(label);
        Ok(())
    }

    /// Nodes without a parent, in id order.
    pub fn roots(&self) -> Vec<NodeId> {
        self.nodes.values().filter(|n| n.parent.is_none()).map(|n| n.id).collect()
    }

    /// Direct children, top to bottom on the canvas (ties by creation order), so dragging a child
    /// up or down and re-running a layout reorders it.
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        let mut kids: Vec<&Node> = self.nodes.values().filter(|n| n.parent == Some(id)).collect();
        kids.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y).then(a.id.cmp(&b.id)));
        kids.into_iter().map(|n| n.id).collect()
    }

    /// Parent, grandparent… nearest first; stops at [`MAX_DEPTH`] or a cycle.
    pub fn ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut cur = self.node(id).and_then(|n| n.parent);
        while let Some(p) = cur {
            if out.len() >= MAX_DEPTH || out.contains(&p) || p == id {
                break;
            }
            out.push(p);
            cur = self.node(p).and_then(|n| n.parent);
        }
        out
    }

    /// Everything below a node, breadth first.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut children_of: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
        for n in self.nodes.values() {
            if let Some(p) = n.parent {
                children_of.entry(p).or_default().push(n.id);
            }
        }
        let mut seen = BTreeSet::from([id]);
        let mut out = Vec::new();
        let mut i = 0;
        let mut frontier = vec![id];
        while let Some(cur) = frontier.get(i).copied() {
            i += 1;
            for kid in children_of.get(&cur).into_iter().flatten() {
                if seen.insert(*kid) {
                    out.push(*kid);
                    frontier.push(*kid);
                }
            }
        }
        out
    }

    /// Shown on the canvas: no ancestor is collapsed.
    pub fn is_visible(&self, id: NodeId) -> bool {
        self.node(id).is_some() && self.ancestors(id).iter().all(|a| self.node(*a).is_some_and(|n| !n.collapsed))
    }

    pub fn to_json(&self) -> String {
        let file = File {
            format: "ideacraft".into(),
            version: FORMAT_VERSION,
            title: self.title.clone(),
            next_id: self.next_id,
            nodes: self.nodes.values().cloned().collect(),
            edges: self.edges.values().cloned().collect(),
        };
        serde_json::to_string_pretty(&file).unwrap_or_default()
    }

    /// Reads a map leniently: duplicates, dangling parents and links, parent cycles and
    /// out-of-range values are repaired; only an unreadable file or a newer format is an error.
    pub fn from_json(text: &str) -> Result<Self> {
        let file: File = serde_json::from_str(text).map_err(|e| Error::Format(e.to_string()))?;
        if file.format != "ideacraft" {
            return Err(Error::Format(format!("not an IdeaCraft map ({:?})", file.format.chars().take(32).collect::<String>())));
        }
        if file.version > FORMAT_VERSION {
            return Err(Error::Format(format!("made by a newer IdeaCraft (format {})", file.version)));
        }
        let mut map = Self::new(&file.title);
        let file_next = file.next_id;
        let mut max_id = 0u64;
        for mut n in file.nodes.into_iter().take(MAX_NODES) {
            if map.nodes.contains_key(&n.id) {
                continue;
            }
            n.label = clean_label(&n.label);
            n.pos = n.pos.clamped();
            max_id = max_id.max(n.id.0);
            map.nodes.insert(n.id, n);
        }
        let ids: Vec<NodeId> = map.nodes.keys().copied().collect();
        for id in &ids {
            let parent = map.node(*id).and_then(|n| n.parent);
            if let Some(p) = parent
                && (!map.nodes.contains_key(&p) || p == *id)
                && let Ok(n) = map.node_mut(*id)
            {
                n.parent = None;
            }
        }
        // Break parent cycles: walk up from each node; a walk that comes back cuts its last step.
        for id in &ids {
            let mut seen = BTreeSet::from([*id]);
            let mut cur = *id;
            let mut steps = 0;
            while let Some(p) = map.node(cur).and_then(|n| n.parent) {
                steps += 1;
                if !seen.insert(p) || steps > MAX_DEPTH {
                    if let Ok(n) = map.node_mut(cur) {
                        n.parent = None;
                    }
                    break;
                }
                cur = p;
            }
        }
        let mut pairs = BTreeSet::new();
        for mut e in file.edges.into_iter().take(MAX_EDGES) {
            let pair = (e.from.min(e.to), e.from.max(e.to));
            if e.from == e.to || !map.nodes.contains_key(&e.from) || !map.nodes.contains_key(&e.to) || map.edges.contains_key(&e.id) {
                continue;
            }
            if map.nodes.contains_key(&NodeId(e.id.0)) || !pairs.insert(pair) {
                continue;
            }
            e.label = clean_label(&e.label);
            max_id = max_id.max(e.id.0);
            map.edges.insert(e.id, e);
        }
        map.next_id = max_id.saturating_add(1).max(file_next).max(1);
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_tree_with_links() {
        let mut m = Map::new("t");
        let a = m.add_node("A", Point::new(0.0, 0.0), None).unwrap();
        let b = m.add_node("B", Point::new(10.0, 5.0), Some(a)).unwrap();
        let c = m.add_node("C", Point::new(10.0, -5.0), Some(a)).unwrap();
        let d = m.add_node("D", Point::new(20.0, 0.0), Some(b)).unwrap();
        assert_eq!(m.children(a), vec![c, b], "children run top to bottom");
        assert_eq!(m.ancestors(d), vec![b, a]);
        assert_eq!(m.descendants(a), vec![b, c, d]);
        let e = m.connect(c, d, "relates").unwrap();
        assert_eq!(m.connect(d, c, "again").unwrap(), e, "a pair is linked once");
        assert_eq!(m.connect(c, c, ""), Err(Error::SelfLink));
        assert_eq!(m.set_parent(a, Some(d)), Err(Error::Cycle));
        m.set_collapsed(b, true).unwrap();
        assert!(!m.is_visible(d));
        assert!(m.is_visible(b));
        assert_eq!(m.remove_node(b).unwrap(), vec![b, d]);
        assert_eq!(m.edge_count(), 0, "links to removed nodes go with them");
        assert_eq!(m.node_count(), 2);
    }

    #[test]
    fn json_round_trips_and_repairs_hostile_files() {
        let m = Map::sample();
        assert!(m.node_count() > 10);
        assert_eq!(Map::from_json(&m.to_json()).unwrap(), m);

        let hostile = r#"{"format":"ideacraft","version":1,"title":"x",
            "nodes":[{"id":1,"label":"a","pos":{"x":1e30,"y":0},"parent":2},
                     {"id":2,"label":"b","pos":{"x":0,"y":0},"parent":1},
                     {"id":2,"label":"dup","pos":{"x":0,"y":0}},
                     {"id":3,"label":"c","pos":{"x":0,"y":0},"parent":99}],
            "edges":[{"id":4,"from":1,"to":1},{"id":5,"from":1,"to":42},{"id":6,"from":1,"to":3},{"id":7,"from":3,"to":1}]}"#;
        let m = Map::from_json(hostile).unwrap();
        assert_eq!(m.node_count(), 3);
        assert_eq!(m.edge_count(), 1);
        assert!(m.node(NodeId(1)).unwrap().pos.x <= crate::limits::MAX_COORD);
        assert_eq!(m.node(NodeId(3)).unwrap().parent, None);
        assert!(m.roots().len() >= 2, "the 1↔2 cycle is broken");
        let mut m = m;
        let fresh = m.add_node("new", Point::default(), None).unwrap();
        assert!(fresh.0 > 6, "new ids never collide with loaded ones");

        assert!(Map::from_json("{").is_err());
        assert!(Map::from_json(r#"{"format":"other","version":1,"title":""}"#).is_err());
        assert!(Map::from_json(r#"{"format":"ideacraft","version":99,"title":""}"#).is_err());
    }

    #[test]
    fn labels_and_positions_are_sanitised() {
        let mut m = Map::new("t");
        let a = m.add_node(&"x".repeat(MAX_LABEL_CHARS + 50), Point::new(f32::NAN, f32::INFINITY), None).unwrap();
        let n = m.node(a).unwrap();
        assert_eq!(n.label.chars().count(), MAX_LABEL_CHARS);
        assert_eq!(n.pos, Point::new(0.0, MAX_COORD));
        m.set_label(a, "two\nlines\u{7}").unwrap();
        assert_eq!(m.node(a).unwrap().label, "two\nlines");
    }
}
