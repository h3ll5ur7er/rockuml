//! A group laid out as a cluster around its nodes and sub-clusters (PlantUML's `Cluster`, without what only
//! Graphviz layouts need).

use std::cell::Cell;

use super::frontier_calculator::FrontierCalculator;
use super::image::{EntityImageState, get_state_description, get_style_state};
use super::rounded_container::RoundedContainer;
use super::{Bibliotekon, ClusterDecoration, ClusterHeader, ColorSequence, MARGIN};
use crate::abel::{Entity, EntityId, EntityPosition, GroupType};
use crate::color::{ColorType, HColor};
use crate::decoration::symbol::{USymbol, USymbols};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{RectangleArea, UTranslate, XDimension2D, XPoint2D};
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::stereo::Stereotype;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// A cluster, by its place in its layout's [`super::Bibliotekon`]; the root cluster is the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClusterId(pub(crate) usize);

impl ClusterId {
    pub(crate) const ROOT: Self = Self(0);
}

pub(crate) struct Cluster {
    parent: Option<ClusterId>,
    group: EntityId,
    /// The leaves laid out right inside, in the order they were added.
    nodes: Vec<EntityId>,
    color: i32,
    /// `None` for the root cluster, which has no title.
    header: Option<ClusterHeader>,
    rectangle_area: Cell<Option<RectangleArea>>,
    /// Where the title goes, once the layout placed the cluster.
    xy_title: Cell<Option<XPoint2D>>,
}

impl Cluster {
    pub(super) fn new(
        parent: Option<ClusterId>,
        group: EntityId,
        cluster_header: Option<ClusterHeader>,
        color_sequence: &mut ColorSequence,
    ) -> Self {
        let color = color_sequence.get_value();
        // Graphviz layouts also number the title and the notes of the cluster.
        for _ in 0..3 {
            color_sequence.get_value();
        }
        Self {
            parent,
            group,
            nodes: Vec::new(),
            color,
            header: cluster_header,
            rectangle_area: Cell::new(None),
            xy_title: Cell::new(None),
        }
    }

    pub(crate) fn get_parent_cluster(&self) -> Option<ClusterId> {
        self.parent
    }

    pub(crate) fn get_group(&self) -> EntityId {
        self.group
    }

    pub(super) fn add_node(&mut self, leaf: EntityId) {
        self.nodes.push(leaf);
    }

    /// `cluster6`: the name of the cluster's subgraph in the layout.
    pub(crate) fn get_cluster_id(&self) -> String {
        format!("cluster{}", self.color)
    }

    fn header(&self) -> &ClusterHeader {
        self.header
            .as_ref()
            .expect("only the root cluster has no header, and it is never drawn")
    }

    pub(crate) fn get_title_and_attribute_width(&self, diagram: &CucaDiagram) -> i32 {
        let minimum_width = self
            .get_style(diagram)
            .value(PName::MinimumWidth)
            .as_double();
        self.header()
            .get_title_and_attribute_width()
            .max(minimum_width.ceil() as i32)
    }

    pub(crate) fn get_title_and_attribute_height(&self) -> i32 {
        self.header().get_title_and_attribute_height()
    }

    pub(crate) fn is_label(&self, diagram: &CucaDiagram) -> bool {
        self.get_title_and_attribute_height() > 0 || self.get_title_and_attribute_width(diagram) > 0
    }

    pub(crate) fn get_title_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.header()
            .get_title()
            .calculate_dimension(string_bounder)
    }

    pub(crate) fn set_title_position(&self, position: XPoint2D) {
        self.xy_title.set(Some(position));
    }

    /// The area between two opposite corners.
    pub(crate) fn set_position(&self, min: XPoint2D, max: XPoint2D) {
        self.rectangle_area
            .set(Some(RectangleArea::build(min, max)));
    }

    /// # Panics
    ///
    /// Before the layout placed the cluster.
    pub(crate) fn get_rectangle_area(&self) -> RectangleArea {
        self.rectangle_area
            .get()
            .expect("the layout places clusters before they are drawn")
    }

    pub(crate) fn draw_u(&self, ug: &UGraphic, diagram: &CucaDiagram, bibliotekon: &Bibliotekon) {
        let group = diagram.entity(self.group);
        if group.is_hidden(diagram) {
            return;
        }
        let full_name = group.get_name(diagram);
        if !full_name.starts_with("##") {
            ug.draw(&UShape::Comment(format!("cluster {full_name}")));
        }
        let style = self.get_style(diagram);
        let border_color = group
            .colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let mut rounded = style.value(PName::RoundCorner).as_double();
        if diagram.skin().strict_uml_style() {
            rounded = 0.0;
        }
        let diagonal_corner = style.value(PName::DiagonalCorner).as_double();

        let mut u_group = UGroup::at(group.get_location());
        u_group.put(UGroupType::Class, "cluster");
        u_group.put(UGroupType::Id, &format!("cluster_{full_name}"));
        u_group.put(UGroupType::DataEntity, full_name);
        u_group.put(UGroupType::DataUid, group.get_uid());
        u_group.put(
            UGroupType::DataQualifiedName,
            diagram.quark(group.get_quark()).get_qualified_name(),
        );
        ug.start_group(&u_group);
        if let Some(url) = &group.url {
            ug.start_url(url);
        }
        let has_entry_exit_points = self
            .nodes
            .iter()
            .any(|node| diagram.entity(*node).get_entity_position() != EntityPosition::Normal);
        if has_entry_exit_points {
            self.manage_entry_exit_point(ug.string_bounder(), diagram, bibliotekon);
        }
        if diagram.get_style_name() == SName::StateDiagram && group.get_usymbol().is_none() {
            self.draw_u_state(ug, diagram, rounded);
        } else {
            let package_style = group
                .get_package_style()
                .unwrap_or_else(|| diagram.skin().package_style());
            let stroke = get_stroke_internal(&style);
            let back_color = get_back_color(
                Self::get_own_back_color(group, &style),
                diagram.get_style_name(),
                group.get_usymbol(),
                group.get_group_type(),
                diagram,
            );
            let header = self.header();
            let decoration = ClusterDecoration::new(
                package_style,
                group.get_usymbol(),
                header.get_title().clone(),
                header.get_stereo().clone(),
                self.get_rectangle_area(),
                stroke,
            );
            decoration.draw_u(
                ug,
                back_color,
                border_color,
                rounded,
                header.get_title_horizontal_alignment(),
                diagram.skin().stereotype_alignment(),
                diagonal_corner,
            );
        }
        if group.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }

    fn get_style(&self, diagram: &CucaDiagram) -> Style {
        let group = diagram.entity(self.group);
        let u_symbol = group.get_usymbol().unwrap_or(USymbols::PACKAGE);
        get_default_style_definition(
            diagram.get_style_name(),
            Some(u_symbol),
            group.get_group_type(),
        )
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            group.stereotype.as_ref(),
        )
    }

    fn get_own_back_color(group: &Entity, style: &Style) -> Option<HColor> {
        if group.is_root() {
            return None;
        }
        Some(
            group
                .colors
                .get(ColorType::Back)
                .cloned()
                .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
        )
    }

    /// How far a link ending at `position`, on the cluster's box, moves to reach its drawn border: below the
    /// tab of a folder, for instance.
    pub(crate) fn get_magnetic_border_force_at(
        &self,
        string_bounder: &dyn StringBounder,
        position: XPoint2D,
        diagram: &CucaDiagram,
    ) -> UTranslate {
        let group = diagram.entity(self.group);
        if group.get_usymbol().is_none() && group.get_group_type() != GroupType::Package {
            return UTranslate::default();
        }
        let package_style = group
            .get_package_style()
            .unwrap_or_else(|| diagram.skin().package_style());
        // PlantUML styles the border as in a class diagram, whatever the diagram.
        let style = get_default_style_definition(
            SName::ClassDiagram,
            group.get_usymbol(),
            group.get_group_type(),
        )
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            group.stereotype.as_ref(),
        );
        let header = self.header();
        let rectangle_area = self.get_rectangle_area();
        let decoration = ClusterDecoration::new(
            package_style,
            group.get_usymbol(),
            header.get_title().clone(),
            header.get_stereo().clone(),
            rectangle_area,
            get_stroke_internal(&style),
        );
        let Some(text_block) = decoration.get_text_block(
            HColor::BLACK,
            HColor::BLACK,
            0.0,
            diagram.skin().package_title_alignment(),
            diagram.skin().stereotype_alignment(),
            0.0,
        ) else {
            return UTranslate::default();
        };
        text_block.magnetic_border_force_at(
            string_bounder,
            position.move_by(-rectangle_area.get_min_x(), -rectangle_area.get_min_y()),
        )
    }
}

/// The style rules a group follows (`Cluster.getDefaultStyleDefinition`).
pub(crate) fn get_default_style_definition(
    diagram_style_name: SName,
    symbol: Option<USymbol>,
    group_type: GroupType,
) -> StyleSignature {
    let of =
        |names: &[SName]| StyleSignature::of(&[&[SName::Root, SName::Element][..], names].concat());
    if diagram_style_name == SName::StateDiagram {
        return of(&[SName::StateDiagram, SName::State, SName::Group]);
    }
    if let Some(symbol) = symbol {
        let mut names = vec![diagram_style_name, SName::Group];
        names.extend(symbol.get_s_names());
        return of(&names);
    }
    if group_type == GroupType::Package {
        return of(&[diagram_style_name, SName::Package, SName::Group]);
    }
    of(&[diagram_style_name, SName::Group])
}

/// The border of a group.
fn get_stroke_internal(style: &Style) -> UStroke {
    style.stroke()
}

/// A group's background: its own, or its symbol's style's; transparent when that has none
/// (`Cluster.getBackColor`).
fn get_back_color(
    back_color: Option<HColor>,
    style_name: SName,
    symbol: Option<USymbol>,
    group_type: GroupType,
    diagram: &CucaDiagram,
) -> HColor {
    back_color.unwrap_or_else(|| {
        get_default_style_definition(style_name, symbol, group_type)
            .get_merged_style(&diagram.skin().current_style_builder())
            .value(PName::BackGroundColor)
            .as_color()
    })
}

/// What only state diagrams draw.
impl Cluster {
    /// `Cluster.drawUState`: a composite state is a rounded box with its name and description on top.
    fn draw_u_state(&self, ug: &UGraphic, diagram: &CucaDiagram, rounded: f64) {
        let group = diagram.entity(self.group);
        let builder = diagram.skin().current_style_builder();
        let stereotype = group.stereotype.as_ref();
        let style_state = get_style_state(None, stereotype, &builder);
        let border_color = group
            .colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| style_state.value(PName::LineColor).as_color());
        let back = |part| {
            get_style_state(Some(part), stereotype, &builder)
                .value(PName::BackGroundColor)
                .as_color()
        };
        let (north_backcolor, center_backcolor, south_backcolor) =
            group.colors.get(ColorType::Back).map_or_else(
                || {
                    (
                        back(SName::Name),
                        back(SName::Description),
                        back(SName::Body),
                    )
                },
                |own| (own.clone(), own.clone(), own.clone()),
            );
        let string_bounder = ug.string_bounder();
        let attribute = get_state_description(group, diagram);
        let attribute_height = attribute.calculate_dimension(string_bounder).height;
        let rectangle_area = self.get_rectangle_area();
        if rectangle_area.get_width() == 0.0 {
            return;
        }
        let stroke = group
            .colors
            .get_specific_line_stroke()
            .unwrap_or_else(|| style_state.stroke());
        let margin = f64::from(MARGIN);
        let description_height =
            attribute_height + if attribute_height > 0.0 { margin } else { 0.0 };
        let header = self.header();
        let name_height = header
            .get_title()
            .calculate_dimension(string_bounder)
            .height
            + margin;
        let position = rectangle_area.get_position();
        RoundedContainer {
            dim: rectangle_area.get_dimension(),
            name_height,
            description_height,
            border_color: border_color.clone(),
            north_backcolor,
            center_backcolor,
            south_backcolor,
            stroke,
            rounded,
        }
        .draw_u(&ug.translated(position.dx, position.dy));
        let title_y = self
            .xy_title
            .get()
            .expect("the layout places the title with the cluster")
            .y;
        header.get_title_horizontal_alignment().draw(
            &ug.translated(rectangle_area.get_min_x(), title_y),
            header.get_title().as_ref(),
            margin,
            0.0,
            rectangle_area.get_width(),
        );
        if attribute_height > 0.0 {
            attribute.draw_u(&ug.translated(
                rectangle_area.get_min_x() + margin,
                rectangle_area.get_min_y() + name_height + margin / 2.0,
            ));
        }
        if stereotype.is_some_and(Stereotype::is_with_oo_symbol) {
            EntityImageState::draw_symbol(
                &ug.with_color(border_color),
                rectangle_area.get_max_x(),
                rectangle_area.get_max_y(),
            );
        }
    }

    /// `Cluster.manageEntryExitPoint`: a group with entry or exit points on its border grows around them,
    /// and its title is centred again.
    fn manage_entry_exit_point(
        &self,
        string_bounder: &dyn StringBounder,
        diagram: &CucaDiagram,
        bibliotekon: &Bibliotekon,
    ) {
        let mut insides = Vec::new();
        let mut points = Vec::new();
        for leaf in &self.nodes {
            let node = bibliotekon
                .get_node(*leaf)
                .expect("the nodes of a cluster are laid out");
            if diagram.entity(*leaf).get_entity_position().is_normal() {
                insides.push(node.get_rectangle_area());
            } else {
                points.push(node.get_rectangle_area().get_point_center());
            }
        }
        // A sub-cluster is only placed once drawn, after its parent.
        insides.extend(
            bibliotekon
                .get_children(self.group)
                .filter_map(|child| child.rectangle_area.get()),
        );
        let mut frontier_calculator = FrontierCalculator::new(
            self.get_rectangle_area(),
            &insides,
            &points,
            diagram.skin().get_rankdir(),
        );
        let width = self.get_title_and_attribute_width(diagram);
        if width > 0 && self.get_title_and_attribute_height() > 0 {
            frontier_calculator.ensure_min_width(f64::from(width + 10));
        }
        let rectangle_area = frontier_calculator.get_suggested_position();
        self.rectangle_area.set(Some(rectangle_area));
        let width_title = self.get_title_dimension(string_bounder).width;
        self.xy_title.set(Some(XPoint2D::new(
            rectangle_area.get_min_x() + (rectangle_area.get_width() - width_title) / 2.0,
            rectangle_area.get_min_y() + f64::from(MARGIN),
        )));
    }
}
