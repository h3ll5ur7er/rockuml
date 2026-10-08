//! The nodes of a work breakdown (PlantUML's `WElement`).

use std::cell::Cell;
use std::rc::Rc;

use crate::color::HColor;
use crate::creole::Display;
use crate::diagram::mindmap::IdeaShape;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::stereo::Stereotype;
use crate::style::{SName, Style, StyleBuilder, StyleSignature};

/// Where an element sits in the diagram's arena.
pub(super) type ElementId = usize;

/// Which way an element hangs from its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Left,
    Right,
}

pub(super) struct WElement {
    pub(super) back_color: Option<HColor>,
    pub(super) label: Display,
    pub(super) level: usize,
    pub(super) stereotype: Option<Stereotype>,
    parent: Option<ElementId>,
    pub(super) style_builder: Rc<StyleBuilder>,
    pub(super) children_left: Vec<ElementId>,
    pub(super) children_right: Vec<ElementId>,
    pub(super) shape: IdeaShape,
    /// Where the element was last drawn, relative to the diagram, which links are drawn between.
    geometry: Cell<Option<(UTranslate, XDimension2D)>>,
}

/// What a new element is made of.
pub(super) struct ElementSpec {
    pub(super) back_color: Option<HColor>,
    pub(super) label: Display,
    pub(super) stereotype: Option<Stereotype>,
    pub(super) style_builder: Rc<StyleBuilder>,
    pub(super) shape: IdeaShape,
}

/// The elements of a work breakdown, the root first.
#[derive(Default)]
pub(super) struct Elements {
    elements: Vec<WElement>,
}

impl Elements {
    pub(super) const ROOT: ElementId = 0;

    pub(super) fn has_root(&self) -> bool {
        !self.elements.is_empty()
    }

    pub(super) fn init_root(&mut self, spec: ElementSpec) {
        self.elements.push(WElement::new(spec, 0, None));
    }

    pub(super) fn get(&self, id: ElementId) -> &WElement {
        &self.elements[id]
    }

    pub(super) fn parent(&self, id: ElementId) -> Option<ElementId> {
        self.elements[id].parent
    }

    /// A child of `parent`; a first-level element to the left also leads the right ones, whose list the
    /// root's layout reads (`createElement`).
    pub(super) fn create_element(
        &mut self,
        parent: ElementId,
        level: usize,
        spec: ElementSpec,
        direction: Direction,
    ) -> ElementId {
        let id = self.elements.len();
        self.elements.push(WElement::new(spec, level, Some(parent)));
        let parent = &mut self.elements[parent];
        if direction == Direction::Left && level == 1 {
            parent.children_right.insert(0, id);
        }
        if direction == Direction::Left {
            parent.children_left.push(id);
        } else {
            parent.children_right.push(id);
        }
        id
    }

    /// The element's style, with the rules its ancestors pass down (`getStyle`).
    pub(super) fn style(&self, id: ElementId) -> Style {
        let element = &self.elements[id];
        let ancestors =
            std::iter::successors(element.parent, |&ancestor| self.elements[ancestor].parent)
                .map(|ancestor| self.style_signature(ancestor, element.level));
        element
            .style_builder
            .merged_style_of_tree_node(&self.style_signature(id, element.level), ancestors)
            .expect("the default skin styles work breakdown nodes")
    }

    /// The selectors an element answers to, at `level` (`getDefaultStyleDefinitionNode`).
    fn style_signature(&self, id: ElementId, level: usize) -> StyleSignature {
        let element = &self.elements[id];
        let mut names = vec![SName::Root, SName::Element, SName::WbsDiagram, SName::Node];
        if level == 0 {
            names.push(SName::RootNode);
        } else if element.is_leaf() {
            names.push(SName::LeafNode);
        }
        if element.shape == IdeaShape::None {
            names.push(SName::Boxless);
        }
        StyleSignature::of(&names)
            .with_stereotype_labels(element.stereotype.as_ref())
            .with_level(super::level_of(level))
    }
}

impl WElement {
    fn new(spec: ElementSpec, level: usize, parent: Option<ElementId>) -> Self {
        Self {
            back_color: spec.back_color,
            label: spec.label,
            level,
            stereotype: spec.stereotype,
            parent,
            style_builder: spec.style_builder,
            children_left: Vec::new(),
            children_right: Vec::new(),
            shape: spec.shape,
            geometry: Cell::new(None),
        }
    }

    pub(super) fn is_leaf(&self) -> bool {
        self.children_left.is_empty() && self.children_right.is_empty()
    }

    pub(super) fn set_geometry(&self, position: UTranslate, dimension: XDimension2D) {
        self.geometry.set(Some((position, dimension)));
    }

    pub(super) fn geometry(&self) -> Option<(UTranslate, XDimension2D)> {
        self.geometry.get()
    }
}
