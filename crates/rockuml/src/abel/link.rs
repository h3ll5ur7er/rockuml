//! A link between two entities, with what it says and how it looks (PlantUML's `Link`, `LinkArg`,
//! `LinkArrow`, and `WithLinkType`, whose only subclass `Link` is).

use std::rc::Rc;

use super::{CucaNote, EntityId, EntityPosition, LeafType};
use crate::color::{ColorType, Colors, HColor};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::url::Url;
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
    labeldistance: Option<String>,
    labelangle: Option<String>,
    kal1: Option<String>,
    kal2: Option<String>,
    length: i32,
}

impl LinkArg {
    /// `<<stereotypes>>` in the label show in guillemets.
    pub(crate) fn build(label: Option<Display>, length: i32) -> Self {
        Self {
            label: label.map(|label| label.manage_guillemet()),
            quantifier1: None,
            quantifier2: None,
            role1: None,
            role2: None,
            labeldistance: None,
            labelangle: None,
            kal1: None,
            kal2: None,
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

    #[must_use]
    pub(crate) fn with_kal(self, kal1: Option<String>, kal2: Option<String>) -> Self {
        Self { kal1, kal2, ..self }
    }

    #[must_use]
    pub(crate) fn with_distance_angle(
        self,
        labeldistance: Option<String>,
        labelangle: Option<String>,
    ) -> Self {
        Self {
            labeldistance,
            labelangle,
            ..self
        }
    }

    /// The same words for the link drawn from its other end.
    #[must_use]
    pub(crate) fn get_inv(&self) -> Self {
        Self {
            quantifier1: self.quantifier2.clone(),
            quantifier2: self.quantifier1.clone(),
            kal1: self.kal2.clone(),
            kal2: self.kal1.clone(),
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

    pub(crate) fn get_labeldistance(&self) -> Option<&str> {
        self.labeldistance.as_deref()
    }

    pub(crate) fn get_labelangle(&self) -> Option<&str> {
        self.labelangle.as_deref()
    }

    pub(crate) fn get_kal1(&self) -> Option<&str> {
        self.kal1.as_deref()
    }

    pub(crate) fn get_kal2(&self) -> Option<&str> {
        self.kal2.as_deref()
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
pub(crate) struct Link {
    id: LinkId,
    uid: String,
    location: Option<LineLocation>,
    style_builder: Rc<StyleBuilder>,
    cl1: EntityId,
    cl2: EntityId,
    link_type: LinkType,
    link_arg: LinkArg,
    port1: Option<String>,
    port2: Option<String>,
    /// `hidden` in the arrow's style.
    hidden: bool,
    single: bool,
    use_node_style: bool,
    colors: Colors,
    /// The colours of the parallel lines after the first.
    supplementary: Vec<Colors>,
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
    #[expect(clippy::too_many_arguments, reason = "PlantUML's constructor")]
    pub(crate) fn new(
        id: LinkId,
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
            id,
            uid,
            location,
            style_builder,
            cl1,
            cl2,
            link_type,
            link_arg,
            port1: None,
            port2: None,
            hidden: false,
            single: false,
            use_node_style: false,
            colors: Colors::default(),
            supplementary: Vec::new(),
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

    /// The same link from the other end, under the next id and uid; the diagram numbers it.
    pub(crate) fn get_inv(&self, id: LinkId, uid: String) -> Self {
        let mut result = Self::new(
            id,
            uid,
            self.location.clone(),
            self.style_builder.clone(),
            self.cl2,
            self.cl1,
            self.link_type.get_inversed(),
            self.link_arg.get_inv(),
        );
        result.inverted = !self.inverted;
        result.port1.clone_from(&self.port2);
        result.port2.clone_from(&self.port1);
        result.url.clone_from(&self.url);
        result.stereotype.clone_from(&self.stereotype);
        result.link_arrow = self.link_arrow;
        result
    }

    pub(crate) fn id(&self) -> LinkId {
        self.id
    }

    /// `lnk` and the diagram's counter, like `lnk12`.
    pub(crate) fn get_uid(&self) -> &str {
        &self.uid
    }

    pub(crate) fn get_location(&self) -> Option<&LineLocation> {
        self.location.as_ref()
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

    pub(crate) fn get_port_name1(&self) -> Option<&str> {
        self.port1.as_deref()
    }

    pub(crate) fn get_port_name2(&self) -> Option<&str> {
        self.port2.as_deref()
    }

    /// Only the diagram sets ports, as the entities learn their names too.
    pub(crate) fn set_ports(&mut self, port1: Option<String>, port2: Option<String>) {
        self.port1 = port1;
        self.port2 = port2;
    }

    /// Opale links and links sharing a tail draw no decoration of their own.
    pub(crate) fn get_type(&self) -> LinkType {
        if self.opale || self.sametail.is_some() {
            return LinkType::new(LinkDecor::None, LinkDecor::None);
        }
        self.link_type
    }

    /// The type without the decorations at ends that are groups with something inside.
    pub(crate) fn get_type_patch_cluster(&self, diagram: &CucaDiagram) -> LinkType {
        let is_really_group = |id: EntityId| {
            let entity = diagram.entity(id);
            entity.is_group() && entity.groups(diagram).len() + entity.leafs(diagram).len() > 0
        };
        let mut result = self.get_type();
        if is_really_group(self.cl1) {
            result = result.without_decors2();
        }
        if is_really_group(self.cl2) {
            result = result.without_decors1();
        }
        result
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

    pub(crate) fn get_link_arg(&self) -> &LinkArg {
        &self.link_arg
    }

    pub(crate) fn get_label(&self) -> Option<&Display> {
        self.link_arg.get_label()
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

    pub(crate) fn get_labeldistance(&self) -> Option<&str> {
        self.link_arg.get_labeldistance()
    }

    pub(crate) fn get_labelangle(&self) -> Option<&str> {
        self.link_arg.get_labelangle()
    }

    pub(crate) fn has_kal1(&self) -> bool {
        self.link_arg.get_kal1().is_some_and(|kal| !kal.is_empty())
    }

    pub(crate) fn has_kal2(&self) -> bool {
        self.link_arg.get_kal2().is_some_and(|kal| !kal.is_empty())
    }

    pub(crate) fn is_auto_link_of_a_group(&self, diagram: &CucaDiagram) -> bool {
        diagram.entity(self.cl1).is_group()
            && diagram.entity(self.cl2).is_group()
            && self.cl1 == self.cl2
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

    pub(crate) fn has_entry_point(&self, diagram: &CucaDiagram) -> bool {
        let on_border = |id: EntityId| {
            let entity = diagram.entity(id);
            !entity.is_group() && entity.get_entity_position() != EntityPosition::Normal
        };
        on_border(self.cl1) || on_border(self.cl2)
    }

    pub(crate) fn has_two_entry_points_same_container(&self, diagram: &CucaDiagram) -> bool {
        let (entity1, entity2) = (diagram.entity(self.cl1), diagram.entity(self.cl2));
        !entity1.is_group()
            && !entity2.is_group()
            && entity1.get_entity_position() != EntityPosition::Normal
            && entity2.get_entity_position() != EntityPosition::Normal
            && entity1.get_parent_container(diagram) == entity2.get_parent_container(diagram)
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

    pub(crate) fn does_touch(&self, other: &Link) -> bool {
        self.contains(other.cl1) || self.contains(other.cl2)
    }

    pub(crate) fn is_autolink(&self) -> bool {
        self.cl1 == self.cl2
    }

    // What follows is PlantUML's `WithLinkType`.

    pub(crate) fn get_specific_color(&self) -> Option<&HColor> {
        self.colors.get(ColorType::Line)
    }

    /// The colour of the first line (`i` 0), or of one more parallel line.
    pub(crate) fn set_specific_color(&mut self, specific_color: HColor, i: usize) {
        let colors = self.colors.with(ColorType::Line, Some(specific_color));
        if i == 0 {
            self.colors = colors;
        } else {
            self.supplementary.push(colors);
        }
    }

    pub(crate) fn get_supplementary_colors(&self) -> &[Colors] {
        &self.supplementary
    }

    pub(crate) fn get_colors(&self) -> &Colors {
        &self.colors
    }

    pub(crate) fn set_colors(&mut self, colors: Colors) {
        self.colors = colors;
    }

    pub(crate) fn go_dashed(&mut self) {
        self.link_type = self.link_type.go_dashed();
    }

    pub(crate) fn go_dotted(&mut self) {
        self.link_type = self.link_type.go_dotted();
    }

    pub(crate) fn go_thickness(&mut self, thickness: f64) {
        self.link_type = self.link_type.go_thickness(thickness);
    }

    pub(crate) fn go_bold(&mut self) {
        self.link_type = self.link_type.go_bold();
    }

    pub(crate) fn go_hidden(&mut self) {
        self.hidden = true;
    }

    pub(crate) fn go_norank(&mut self) {
        self.constraint = false;
    }

    pub(crate) fn go_single(&mut self) {
        self.single = true;
    }

    pub(crate) fn is_single(&self) -> bool {
        self.single
    }

    pub(crate) fn go_node_style(&mut self) {
        self.use_node_style = true;
    }

    pub(crate) fn use_node_style(&self) -> bool {
        self.use_node_style
    }

    /// `dashed,#red;bold`: the style in brackets in an arrow, `;` separating parallel lines.
    pub(crate) fn apply_style(&mut self, arrow_style: Option<&str>) {
        let Some(arrow_style) = arrow_style else {
            return;
        };
        for (i, style) in tokens(arrow_style, ';').enumerate() {
            self.apply_one_style(style, i);
        }
    }

    fn apply_one_style(&mut self, arrow_style: &str, i: usize) {
        for s in tokens(arrow_style, ',') {
            if s.eq_ignore_ascii_case("dashed") {
                self.go_dashed();
            } else if s.eq_ignore_ascii_case("bold") {
                self.go_bold();
            } else if s.eq_ignore_ascii_case("dotted") {
                self.go_dotted();
            } else if s.eq_ignore_ascii_case("hidden") {
                self.go_hidden();
            } else if s.eq_ignore_ascii_case("single") {
                self.go_single();
            } else if s.eq_ignore_ascii_case("plain") {
                // A plain line is the default.
            } else if s.eq_ignore_ascii_case("node") {
                self.go_node_style();
            } else if s.eq_ignore_ascii_case("norank") {
                self.go_norank();
            } else if let Some(thickness) = s.strip_prefix("thickness=") {
                self.go_thickness(thickness.parse().unwrap_or_default());
            } else {
                self.set_specific_color(HColor::parse_or_white(s), i);
            }
        }
    }
}

/// `StringTokenizer`: the non-empty pieces between delimiters.
fn tokens(text: &str, delimiter: char) -> impl Iterator<Item = &str> {
    text.split(delimiter).filter(|token| !token.is_empty())
}
