//! The nodes of one side of a mind map (PlantUML's `Idea` and `Branch`).

use std::rc::Rc;

use crate::color::HColor;
use crate::command::CommandError;
use crate::creole::Display;
use crate::stereo::Stereotype;
use crate::style::{SName, Style, StyleBuilder, StyleSignature};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdeaShape {
    Box,
    /// `_`: the text alone.
    None,
}

impl IdeaShape {
    pub(crate) fn from_desc(shape: Option<&str>) -> Self {
        if shape == Some("_") {
            Self::None
        } else {
            Self::Box
        }
    }
}

/// Where an idea sits in its branch's arena.
pub(super) type IdeaId = usize;

pub(super) struct Idea {
    pub(super) label: Display,
    level: usize,
    parent: Option<IdeaId>,
    pub(super) children: Vec<IdeaId>,
    pub(super) shape: IdeaShape,
    pub(super) back_color: Option<HColor>,
    style_builder: Rc<StyleBuilder>,
    stereotype: Option<Stereotype>,
}

/// What a new idea is made of.
pub(super) struct IdeaSpec {
    pub(super) style_builder: Rc<StyleBuilder>,
    pub(super) back_color: Option<HColor>,
    pub(super) label: Display,
    pub(super) shape: IdeaShape,
    pub(super) stereotype: Option<Stereotype>,
}

/// One side of a mind map: the root and what grows from it on that side.
#[derive(Default)]
pub(super) struct Branch {
    ideas: Vec<Idea>,
    last: Option<IdeaId>,
}

impl Branch {
    pub(super) const ROOT: IdeaId = 0;

    pub(super) fn init_root(&mut self, spec: IdeaSpec) {
        self.ideas = vec![Idea::new(spec, 0, None)];
        self.last = Some(Self::ROOT);
    }

    pub(super) fn has_root(&self) -> bool {
        !self.ideas.is_empty()
    }

    pub(super) fn has_children(&self) -> bool {
        !self.ideas[Self::ROOT].children.is_empty()
    }

    pub(super) fn idea(&self, id: IdeaId) -> &Idea {
        &self.ideas[id]
    }

    /// Adds an idea `level` deep, below the last one or one of its ancestors.
    pub(super) fn add(&mut self, level: usize, spec: IdeaSpec) -> Result<(), CommandError> {
        let Some(last) = self.last else {
            return Err(CommandError::new("Check your indentation ?"));
        };
        let last_level = self.ideas[last].level;
        let parent = if level == last_level + 1 {
            last
        } else if level <= last_level {
            self.parent_of_last(last_level - level + 1)
        } else {
            return Err(CommandError::new("error42L"));
        };
        let id = self.ideas.len();
        self.ideas.push(Idea::new(spec, level, Some(parent)));
        self.ideas[parent].children.push(id);
        self.last = Some(id);
        Ok(())
    }

    fn parent_of_last(&self, generations: usize) -> IdeaId {
        let mut result = self.last.expect("there is a last idea");
        for _ in 0..generations {
            result = self.ideas[result]
                .parent
                .expect("the levels are consistent");
        }
        result
    }

    /// The idea's style, with the rules its ancestors pass down (`getStyle`).
    pub(super) fn style(&self, id: IdeaId) -> Style {
        let idea = &self.ideas[id];
        let ancestors = std::iter::successors(idea.parent, |&ancestor| self.ideas[ancestor].parent)
            .map(|ancestor| self.style_signature(ancestor, idea.level));
        idea.style_builder
            .merged_style_of_tree_node(&self.style_signature(id, idea.level), ancestors)
            .expect("the default skin styles mind map nodes")
    }

    /// The style of the links to the idea's children (`getStyleArrow`).
    pub(super) fn style_arrow(&self, id: IdeaId) -> Style {
        let idea = &self.ideas[id];
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::MindmapDiagram,
            SName::Arrow,
        ])
        .with_stereotype_labels(idea.stereotype.as_ref())
        .with_level(level_of(idea.level))
        .get_merged_style(&idea.style_builder)
    }

    /// The selectors an idea answers to, at `level` (`getDefaultStyleDefinitionNode`).
    fn style_signature(&self, id: IdeaId, level: usize) -> StyleSignature {
        let idea = &self.ideas[id];
        let mut names = vec![
            SName::Root,
            SName::Element,
            SName::MindmapDiagram,
            SName::Node,
        ];
        if level == 0 {
            names.push(SName::RootNode);
        } else if idea.children.is_empty() {
            names.push(SName::LeafNode);
        }
        if idea.shape == IdeaShape::None {
            names.push(SName::Boxless);
        }
        StyleSignature::of(&names)
            .with_stereotype_labels(idea.stereotype.as_ref())
            .with_level(level_of(level))
    }
}

fn level_of(level: usize) -> i32 {
    i32::try_from(level).expect("mind maps are not that deep")
}

impl Idea {
    fn new(spec: IdeaSpec, level: usize, parent: Option<IdeaId>) -> Self {
        Self {
            label: spec.label,
            level,
            parent,
            children: Vec::new(),
            shape: spec.shape,
            back_color: spec.back_color,
            style_builder: spec.style_builder,
            stereotype: spec.stereotype,
        }
    }
}
