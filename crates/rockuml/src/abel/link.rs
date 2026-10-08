//! A link between two entities, with what it says and how it looks (PlantUML's `Link`, `LinkArg`,
//! `LinkArrow`, and `WithLinkType`, whose only subclass `Link` is).

use std::rc::Rc;

use super::{CucaNote, EntityId, LeafType};
use crate::color::{ColorType, Colors, HColor};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType, WithLinkType};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::url::Url;
use crate::skin::visibility_modifier::VisibilityModifier;
use crate::stereo::Stereotype;
use crate::style::StyleBuilder;
use crate::text::LineLocation;

/// A link, by its place in its diagram's arena, which keeps links even once removed from the diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct LinkId(pub(crate) usize);

/// Which way the small arrow drawn next to a link's label points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkArrow {
    NoneOrSeveral,
    DirectNormal,
    Backward,
}

impl LinkArrow {
    #[must_use]
    pub(crate) fn reverse(self) -> Self {
        match self {
            Self::DirectNormal => Self::Backward,
            Self::Backward => Self::DirectNormal,
            Self::NoneOrSeveral => Self::NoneOrSeveral,
        }
    }
}

/// What a link says: its label, the quantifiers and roles at its ends, and its length in ranks.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LinkArg {
    /// `None` for a link without label, which PlantUML tells from an empty one.
    label: Option<Display>,
    quantifier1: Option<String>,
    quantifier2: Option<String>,
    role1: Option<String>,
    role2: Option<String>,
    /// A label starting with a visibility character shows it as an icon.
    visibility_modifier: Option<VisibilityModifier>,
    length: i32,
}

impl LinkArg {
    pub(crate) fn build(label: Option<Display>, length: i32) -> Self {
        Self::build_managing(label, length, true)
    }

    /// `<<stereotypes>>` in the label show in guillemets; a visibility starting it becomes an icon when
    /// `manage_visibility_modifier`.
    pub(crate) fn build_managing(
        label: Option<Display>,
        length: i32,
        manage_visibility_modifier: bool,
    ) -> Self {
        let visibility_modifier = label
            .as_ref()
            .and_then(|label| label.lines().first())
            .filter(|_| manage_visibility_modifier)
            .filter(|first| VisibilityModifier::is_visibility_character(first))
            .and_then(|first| VisibilityModifier::get_visibility_modifier(first, false));
        Self {
            visibility_modifier,
            label: label.map(|label| label.manage_guillemet(manage_visibility_modifier)),
            quantifier1: None,
            quantifier2: None,
            role1: None,
            role2: None,
            length,
        }
    }

    pub(crate) fn no_display(length: i32) -> Self {
        Self::build(None, length)
    }

    #[must_use]
    pub(crate) fn with_quantifier(
        self,
        quantifier1: Option<String>,
        quantifier2: Option<String>,
    ) -> Self {
        Self {
            quantifier1,
            quantifier2,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn with_role(self, role1: Option<String>, role2: Option<String>) -> Self {
        Self {
            role1,
            role2,
            ..self
        }
    }

    /// The same words for the link drawn from its other end.
    #[must_use]
    pub(crate) fn get_inv(&self) -> Self {
        Self {
            quantifier1: self.quantifier2.clone(),
            quantifier2: self.quantifier1.clone(),
            role1: self.role2.clone(),
            role2: self.role1.clone(),
            ..self.clone()
        }
    }

    pub(crate) fn get_label(&self) -> Option<&Display> {
        self.label.as_ref()
    }

    pub(crate) fn get_length(&self) -> i32 {
        self.length
    }

    pub(crate) fn set_length(&mut self, length: i32) {
        self.length = length;
    }

    pub(crate) fn get_quantifier1(&self) -> Option<&str> {
        self.quantifier1.as_deref()
    }

    pub(crate) fn get_quantifier2(&self) -> Option<&str> {
        self.quantifier2.as_deref()
    }

    pub(crate) fn get_visibility_modifier(&self) -> Option<VisibilityModifier> {
        self.visibility_modifier
    }

    pub(crate) fn get_role1(&self) -> Option<&str> {
        self.role1.as_deref()
    }

    pub(crate) fn get_role2(&self) -> Option<&str> {
        self.role2.as_deref()
    }
}

#[expect(
    clippy::struct_field_names,
    clippy::struct_excessive_bools,
    reason = "PlantUML's fields, each an independent setting"
)]
#[derive(Clone)]
pub(crate) struct Link {
    uid: String,
    location: Option<LineLocation>,
    style_builder: Rc<StyleBuilder>,
    cl1: EntityId,
    cl2: EntityId,
    link_type: LinkType,
    link_arg: LinkArg,
    /// `hidden` in the arrow's style.
    hidden: bool,
    single: bool,
    colors: Colors,
    pub note: Option<CucaNote>,
    invis: bool,
    pub weight: f64,
    pub constraint: bool,
    inverted: bool,
    pub link_arrow: LinkArrow,
    pub opale: bool,
    pub horizontal_solitary: bool,
    pub sametail: Option<String>,
    pub stereotype: Option<Stereotype>,
    pub url: Option<Url>,
    pub code_line: Option<LineLocation>,
}

impl Link {
    /// Only diagrams create links, numbering them as they go.
    ///
    /// # Panics
    ///
    /// If the link is not at least one rank long.
    pub(crate) fn new(
        uid: String,
        location: Option<LineLocation>,
        style_builder: Rc<StyleBuilder>,
        cl1: EntityId,
        cl2: EntityId,
        link_type: LinkType,
        link_arg: LinkArg,
    ) -> Self {
        assert!(
            link_arg.get_length() >= 1,
            "links are at least one rank long"
        );
        Self {
            uid,
            location,
            style_builder,
            cl1,
            cl2,
            link_type,
            link_arg,
            hidden: false,
            single: false,
            colors: Colors::default(),
            note: None,
            invis: false,
            weight: 1.0,
            constraint: true,
            inverted: false,
            link_arrow: LinkArrow::NoneOrSeveral,
            opale: false,
            horizontal_solitary: false,
            sametail: None,
            stereotype: None,
            url: None,
            code_line: None,
        }
    }

    /// The same link from the other end, under the next uid; the diagram numbers it.
    pub(crate) fn get_inv(&self, uid: String) -> Self {
        let mut result = Self::new(
            uid,
            self.location.clone(),
            self.style_builder.clone(),
            self.cl2,
            self.cl1,
            self.link_type.get_inversed(),
            self.link_arg.get_inv(),
        );
        result.inverted = !self.inverted;
        result.url.clone_from(&self.url);
        result.stereotype.clone_from(&self.stereotype);
        result.link_arrow = self.link_arrow;
        result
    }

    /// `lnk` and the diagram's counter, like `lnk12`.
    pub(crate) fn get_uid(&self) -> &str {
        &self.uid
    }

    pub(crate) fn get_style_builder(&self) -> &Rc<StyleBuilder> {
        &self.style_builder
    }

    pub(crate) fn get_entity1(&self) -> EntityId {
        self.cl1
    }

    pub(crate) fn get_entity2(&self) -> EntityId {
        self.cl2
    }

    /// Opale links and links sharing a tail draw no decoration of their own.
    pub(crate) fn get_type(&self) -> LinkType {
        if self.opale || self.sametail.is_some() {
            return LinkType::new(LinkDecor::None, LinkDecor::None);
        }
        self.link_type
    }

    pub(crate) fn is_invis(&self) -> bool {
        self.link_type.is_invisible() || self.invis
    }

    pub(crate) fn set_invis(&mut self, invis: bool) {
        self.invis = invis;
    }

    pub(crate) fn is_between(&self, cl1: EntityId, cl2: EntityId) -> bool {
        (cl1 == self.cl1 && cl2 == self.cl2) || (cl1 == self.cl2 && cl2 == self.cl1)
    }

    pub(crate) fn get_label(&self) -> Option<&Display> {
        self.link_arg.get_label()
    }

    /// The visibility the label starts with, drawn as an icon before it.
    pub(crate) fn get_visibility_modifier(&self) -> Option<VisibilityModifier> {
        self.link_arg.get_visibility_modifier()
    }

    pub(crate) fn get_length(&self) -> i32 {
        self.link_arg.get_length()
    }

    pub(crate) fn set_length(&mut self, length: i32) {
        self.link_arg.set_length(length);
    }

    pub(crate) fn get_quantifier1(&self) -> Option<&str> {
        self.link_arg.get_quantifier1()
    }

    pub(crate) fn get_quantifier2(&self) -> Option<&str> {
        self.link_arg.get_quantifier2()
    }

    pub(crate) fn get_role1(&self) -> Option<&str> {
        self.link_arg.get_role1()
    }

    pub(crate) fn get_role2(&self) -> Option<&str> {
        self.link_arg.get_role2()
    }

    pub(crate) fn contains_type(&self, leaf_type: LeafType, diagram: &CucaDiagram) -> bool {
        diagram.entity(self.cl1).get_leaf_type() == Some(leaf_type)
            || diagram.entity(self.cl2).get_leaf_type() == Some(leaf_type)
    }

    pub(crate) fn contains(&self, entity: EntityId) -> bool {
        self.cl1 == entity || self.cl2 == entity
    }

    /// # Panics
    ///
    /// If the link does not touch `entity`.
    pub(crate) fn get_other(&self, entity: EntityId) -> EntityId {
        if self.cl1 == entity {
            self.cl2
        } else if self.cl2 == entity {
            self.cl1
        } else {
            panic!("{} does not touch {entity:?}", self.uid)
        }
    }

    /// The arrow next to the label, as seen from the link's own direction.
    pub(crate) fn get_link_arrow(&self) -> LinkArrow {
        if self.inverted {
            self.link_arrow.reverse()
        } else {
            self.link_arrow
        }
    }

    pub(crate) fn is_inverted(&self) -> bool {
        self.inverted
    }

    pub(crate) fn is_hidden(&self, diagram: &CucaDiagram) -> bool {
        self.hidden
            || diagram.entity(self.cl1).is_hidden(diagram)
            || diagram.entity(self.cl2).is_hidden(diagram)
    }

    pub(crate) fn is_removed(&self, diagram: &CucaDiagram) -> bool {
        if self
            .stereotype
            .as_ref()
            .is_some_and(|stereotype| diagram.is_stereotype_removed(stereotype))
        {
            return true;
        }
        diagram.entity(self.cl1).is_removed(diagram) || diagram.entity(self.cl2).is_removed(diagram)
    }

    pub(crate) fn same_connections(&self, other: &Link) -> bool {
        self.is_between(other.cl1, other.cl2)
    }

    pub(crate) fn get_colors(&self) -> &Colors {
        &self.colors
    }

    pub(crate) fn set_colors(&mut self, colors: Colors) {
        self.colors = colors;
    }

    pub(crate) fn is_single(&self) -> bool {
        self.single
    }
}

impl WithLinkType for Link {
    fn link_type_mut(&mut self) -> &mut LinkType {
        &mut self.link_type
    }

    /// Only Graphviz layouts draw the parallel lines later colours ask for.
    fn set_specific_color(&mut self, color: HColor, i: usize) {
        if i == 0 {
            self.colors = self.colors.with(ColorType::Line, Some(color));
        }
    }

    fn go_hidden(&mut self) {
        self.hidden = true;
    }

    fn go_single(&mut self) {
        self.single = true;
    }

    fn go_norank(&mut self) {
        self.constraint = false;
    }
}
