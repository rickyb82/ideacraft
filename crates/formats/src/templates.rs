//! Built-in templates: original content and palettes, defined here in code.
//!
//! Each template is an indented outline (two spaces per level), a palette for its first-level
//! branches, a layout and optional labelled links between ideas (matched by label).

use ideacraft_core::layout::{self, Layout};
use ideacraft_core::{Map, Point, Rgb, Shape};

use crate::build::{self, Link, Topic};
use crate::markdown::{indent, rank, tree};

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// For galleries: a short category.
    pub category: &'static str,
    outline: &'static str,
    links: &'static [(&'static str, &'static str, &'static str)],
    palette: &'static [Rgb],
    root: Rgb,
    root_shape: Shape,
    leaf_shape: Shape,
    layout: Layout,
}

const fn rgb(hex: u32) -> Rgb {
    Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const PASTEL: &[Rgb] = &[rgb(0xa9cdff), rgb(0xa8ead2), rgb(0xd0c2fc), rgb(0xffc0a6), rgb(0xf7b5cf), rgb(0xd6f0a0), rgb(0xffde85)];
const OCEAN: &[Rgb] = &[rgb(0x9fd8f0), rgb(0xa9e4dc), rgb(0xb9cdf5), rgb(0xc8eef7), rgb(0x9cc9e8)];
const SUNSET: &[Rgb] = &[rgb(0xffcf9e), rgb(0xffb3a3), rgb(0xf6a9c3), rgb(0xe2b4ef), rgb(0xffe39b)];
const FOREST: &[Rgb] = &[rgb(0xb8e3b0), rgb(0xd8ecaa), rgb(0xa9d8c3), rgb(0xe6dcae), rgb(0xc4e7cf)];
const SWOT: &[Rgb] = &[rgb(0xb3e8b8), rgb(0xffbfb3), rgb(0xa9cdff), rgb(0xffd99a)];

pub const ALL: &[Template] = &[
    Template {
        id: "brainstorm",
        name: "Brainstorming",
        description: "A central question with idea clusters around it, laid out in a circle.",
        category: "Ideas",
        outline: "What could we try next?
  Quick wins
    Small experiment
    Ask our users
  Big bets
    New audience
    Partnership
  Wild ideas
    Do the opposite
    Combine two products
  Questions
    What would make this easy?
    What is stopping us?
  Inspiration
    Other industries
    Nature",
        links: &[("Ask our users", "New audience", "could reveal")],
        palette: PASTEL,
        root: rgb(0xffde85),
        root_shape: Shape::Ellipse,
        leaf_shape: Shape::Pill,
        layout: Layout::Radial,
    },
    Template {
        id: "concept-map",
        name: "Concept map",
        description: "A study concept map: concepts joined by labelled links (the water cycle as an example).",
        category: "Learning",
        outline: "Water cycle
  Evaporation
    Sun heats water
  Condensation
    Clouds form
  Precipitation
    Rain
    Snow
  Collection
    Rivers
    Oceans
    Groundwater",
        links: &[
            ("Evaporation", "Condensation", "leads to"),
            ("Condensation", "Precipitation", "leads to"),
            ("Precipitation", "Collection", "feeds"),
            ("Collection", "Evaporation", "starts again with"),
        ],
        palette: OCEAN,
        root: rgb(0xffffff),
        root_shape: Shape::Ellipse,
        leaf_shape: Shape::Rounded,
        layout: Layout::Radial,
    },
    Template {
        id: "project-plan",
        name: "Project plan",
        description: "Goals, scope, timeline, team, budget and risks of a project.",
        category: "Work",
        outline: "Project
  Goals
    Main outcome
    How we measure success
  Scope
    In scope
    Out of scope
  Timeline
    Kick-off
    Milestone 1
    Milestone 2
    Launch
  Team
    Owner
    Contributors
    Stakeholders
  Budget
    People
    Tools
  Risks
    Risk and mitigation",
        links: &[("Milestone 2", "Risk and mitigation", "watch")],
        palette: PASTEL,
        root: rgb(0xffde85),
        root_shape: Shape::Pill,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
    Template {
        id: "swot",
        name: "SWOT analysis",
        description: "Strengths, weaknesses, opportunities and threats, each in its own colour.",
        category: "Strategy",
        outline: "SWOT
  Strengths
    What we do best
    Unique resources
  Weaknesses
    What to improve
    Where we lack skills
  Opportunities
    Trends to use
    Unmet needs
  Threats
    Competition
    Changes outside our control",
        links: &[("What we do best", "Unmet needs", "can serve")],
        palette: SWOT,
        root: rgb(0xffffff),
        root_shape: Shape::Ellipse,
        leaf_shape: Shape::Rounded,
        layout: Layout::Radial,
    },
    Template {
        id: "decision",
        name: "Decision making",
        description: "Compare options side by side with pros, cons and cost.",
        category: "Strategy",
        outline: "Decision to make
  Option A
    Pros
    Cons
    Cost
  Option B
    Pros
    Cons
    Cost
  Option C
    Pros
    Cons
    Cost
  Criteria
    Must have
    Nice to have",
        links: &[],
        palette: SUNSET,
        root: rgb(0xffde85),
        root_shape: Shape::Pill,
        leaf_shape: Shape::Pill,
        layout: Layout::Tree,
    },
    Template {
        id: "meeting",
        name: "Meeting notes",
        description: "Agenda, discussion, decisions and action items with owners.",
        category: "Work",
        outline: "Meeting
  Agenda
    Topic 1
    Topic 2
  Attendees
  Discussion
    Key points
    Open questions
  Decisions
  Action items
    Task, owner, date
  Next meeting",
        links: &[("Open questions", "Next meeting", "carry over to")],
        palette: OCEAN,
        root: rgb(0xc8eef7),
        root_shape: Shape::Pill,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
    Template {
        id: "book",
        name: "Book summary",
        description: "Characters, plot, themes and favourite quotes of a book.",
        category: "Learning",
        outline: "Book title
  Author
  Characters
    Main character
    Supporting characters
  Plot
    Beginning
    Turning point
    Ending
  Themes
  Quotes
  My opinion",
        links: &[("Main character", "Turning point", "changes at")],
        palette: SUNSET,
        root: rgb(0xffe39b),
        root_shape: Shape::Rect,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
    Template {
        id: "fishbone",
        name: "Cause and effect",
        description: "Find the root causes of a problem, grouped by category.",
        category: "Strategy",
        outline: "Problem
  People
    Training
    Workload
  Process
    Unclear steps
    Handoffs
  Tools
    Outdated software
  Materials
    Supply delays
  Environment
    Noise
  Measurement
    Wrong metric",
        links: &[("Workload", "Handoffs", "worsens")],
        palette: FOREST,
        root: rgb(0xffb3a3),
        root_shape: Shape::Rect,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
    Template {
        id: "weekly",
        name: "Weekly planner",
        description: "One branch per day, with a focus for the week.",
        category: "Personal",
        outline: "This week
  Focus
  Monday
  Tuesday
  Wednesday
  Thursday
  Friday
  Weekend
    Rest
    Family and friends",
        links: &[],
        palette: PASTEL,
        root: rgb(0xffde85),
        root_shape: Shape::Ellipse,
        leaf_shape: Shape::Pill,
        layout: Layout::Radial,
    },
    Template {
        id: "roadmap",
        name: "Product roadmap",
        description: "Now, next and later, with dependencies between items.",
        category: "Work",
        outline: "Roadmap
  Now
    Fix onboarding
    Faster search
  Next
    Mobile app
    Sharing
  Later
    Offline mode
    Integrations",
        links: &[("Faster search", "Offline mode", "needed for"), ("Sharing", "Integrations", "opens")],
        palette: OCEAN,
        root: rgb(0xd0c2fc),
        root_shape: Shape::Pill,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
    Template {
        id: "journey",
        name: "Customer journey",
        description: "The stages a customer goes through, with what they do and feel.",
        category: "Strategy",
        outline: "Customer journey
  Discover
    Sees a post
  Consider
    Compares options
  Buy
    Checks out
  Use
    First success
  Recommend
    Tells a friend",
        links: &[("Discover", "Consider", "then"), ("Consider", "Buy", "then"), ("Buy", "Use", "then"), ("Use", "Recommend", "then")],
        palette: SUNSET,
        root: rgb(0xffffff),
        root_shape: Shape::Pill,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
    Template {
        id: "lesson",
        name: "Lesson plan",
        description: "Objectives, materials, activities and assessment for a class.",
        category: "Learning",
        outline: "Lesson
  Objectives
    Students will be able to…
  Materials
  Activities
    Warm-up
    Main activity
    Wrap-up
  Assessment
  Homework",
        links: &[("Objectives", "Assessment", "checked by")],
        palette: FOREST,
        root: rgb(0xd6f0a0),
        root_shape: Shape::Ellipse,
        leaf_shape: Shape::Rounded,
        layout: Layout::Tree,
    },
];

pub fn find(id: &str) -> Option<&'static Template> {
    ALL.iter().find(|t| t.id.eq_ignore_ascii_case(id.trim()))
}

impl Template {
    /// A fresh map from this template.
    pub fn build(&self) -> Map {
        let mut widths = Vec::new();
        let items: Vec<(usize, Topic)> =
            self.outline.lines().filter(|l| !l.trim().is_empty()).map(|l| (rank(&mut widths, indent(l)), Topic::new(l.trim()))).collect();
        let mut roots = tree(items);
        for root in &mut roots {
            root.color = Some(self.root);
            root.shape = Some(self.root_shape);
            root.pos = None;
            for (i, branch) in root.children.iter_mut().enumerate() {
                let color = self.palette.get(i % self.palette.len().max(1)).copied().unwrap_or(Rgb::WHITE);
                let mut stack = vec![&mut *branch];
                while let Some(t) = stack.pop() {
                    t.color = Some(color);
                    t.shape = Some(if t.children.is_empty() { self.leaf_shape } else { Shape::Rounded });
                    stack.extend(t.children.iter_mut());
                }
            }
        }
        let links: Vec<Link> = self.links.iter().map(|(a, b, l)| Link { from: (*a).into(), to: (*b).into(), label: (*l).into() }).collect();
        let title = roots.first().map(|r| r.label.clone()).unwrap_or_else(|| self.name.into());
        let mut map = build::build(&title, roots, &links);
        map.set_title(self.name);
        if let Some(root) = map.roots().first().copied() {
            let _ = map.set_pos(root, Point::default());
        }
        layout::apply(&mut map, self.layout);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_builds_with_its_links() {
        for (i, t) in ALL.iter().enumerate() {
            assert!(ALL.iter().take(i).all(|o| o.id != t.id), "duplicate {}", t.id);
            let m = t.build();
            assert!(m.node_count() >= 8, "{} is too small", t.id);
            assert_eq!(m.roots().len(), 1, "{}", t.id);
            assert_eq!(m.edge_count(), t.links.len(), "{}: a link names a missing idea", t.id);
            assert_eq!(Map::from_json(&m.to_json()).as_ref(), Ok(&m));
        }
        assert!(find("SWOT").is_some());
        assert!(find("nope").is_none());
    }
}
