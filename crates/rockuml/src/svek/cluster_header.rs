//! The title and stereotype at the top of a cluster, and the room they take (PlantUML's `ClusterHeader`).

use std::rc::Rc;

use crate::abel::{Entity, GroupType};
use crate::creole::{CreoleMode, Display};
use crate::decoration::symbol::{Block, USymbol};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;
use crate::style::{SName, Style, StyleSignature};

use super::cluster::get_default_style_definition;

pub(crate) struct ClusterHeader {
    title_and_attribute_width: i32,
    title_and_attribute_height: i32,
    title: Block,
    stereo: Block,
    title_horizontal_alignment: HorizontalAlignment,
}

impl ClusterHeader {
    pub(crate) fn new(
        g: &Entity,
        diagram: &CucaDiagram,
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let style = get_style(g, diagram);
        let title_horizontal_alignment = style.horizontal_alignment().unwrap_or_default();
        let title = get_title_block(g, diagram, &style);
        let stereo = get_stereo_block(g, diagram, title_horizontal_alignment);
        let dim_title = title.calculate_dimension(string_bounder);
        let dim_stereo = stereo.calculate_dimension(string_bounder);
        let dim_label = dim_stereo.merge_top_bottom(dim_title);
        let (mut title_and_attribute_width, mut title_and_attribute_height) = (0, 0);
        if dim_label.width > 0.0 {
            let dim_attribute = get_state_description().calculate_dimension(string_bounder);
            let attribute_height = dim_attribute.height;
            let attribute_width = dim_attribute.width;
            let margin_for_fields = if attribute_height > 0.0 {
                f64::from(super::MARGIN)
            } else {
                0.0
            };
            let u_symbol = g.get_usymbol();
            let supp_height = u_symbol.map_or(0, USymbol::supp_height_because_of_shape);
            let supp_width = u_symbol.map_or(0, USymbol::supp_width_because_of_shape);
            title_and_attribute_width = dim_label.width.max(attribute_width) as i32 + supp_width;
            title_and_attribute_height =
                (dim_label.height + attribute_height + margin_for_fields + f64::from(supp_height))
                    as i32;
        }
        Self {
            title_and_attribute_width,
            title_and_attribute_height,
            title,
            stereo,
            title_horizontal_alignment,
        }
    }

    pub(crate) fn get_title_and_attribute_width(&self) -> i32 {
        self.title_and_attribute_width
    }

    pub(crate) fn get_title_and_attribute_height(&self) -> i32 {
        self.title_and_attribute_height
    }

    pub(crate) fn get_title(&self) -> &Block {
        &self.title
    }

    pub(crate) fn get_stereo(&self) -> &Block {
        &self.stereo
    }

    pub(crate) fn get_title_horizontal_alignment(&self) -> HorizontalAlignment {
        self.title_horizontal_alignment
    }
}

/// `Entity.getStateDescription`: the body lines of a composite state. Groups carry none until bodies are
/// ported with the state diagrams.
fn get_state_description() -> TextBlockEmpty {
    TextBlockEmpty::default()
}

fn get_style(g: &Entity, diagram: &CucaDiagram) -> Style {
    get_signature(g, diagram)
        .with_stereostyles(&g.stereostyles)
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            g.stereotype.as_ref(),
        )
}

fn get_signature(g: &Entity, diagram: &CucaDiagram) -> StyleSignature {
    let sname = diagram.get_style_name();
    let base =
        |names: &[SName]| StyleSignature::of(&[&[SName::Root, SName::Element][..], names].concat());
    if g.get_group_type() == GroupType::State {
        return base(&[SName::StateDiagram, SName::State, SName::Name]);
    }
    if let Some(u_symbol) = g.get_usymbol() {
        let mut names = vec![sname];
        names.extend(u_symbol.get_s_names());
        names.extend([SName::Composite, SName::Title]);
        return base(&names);
    }
    if g.get_group_type() == GroupType::Package {
        return base(&[sname, SName::Package, SName::Title]);
    }
    base(&[sname, SName::Composite, SName::Title])
}

fn get_title_block(g: &Entity, diagram: &CucaDiagram, style: &Style) -> Block {
    let font = style.font_configuration_with(&g.colors);
    let alignment = style.horizontal_alignment().unwrap_or_default();
    Rc::new(
        g.display
            .create0(&font, alignment, diagram.skin(), 0.0, CreoleMode::Full),
    )
}

fn get_stereo_block(
    g: &Entity,
    diagram: &CucaDiagram,
    title_horizontal_alignment: HorizontalAlignment,
) -> Block {
    let empty = || -> Block { Rc::new(TextBlockEmpty::default()) };
    let Some(stereotype) = &g.stereotype else {
        return empty();
    };
    // A composite state is drawn by `Cluster::draw_u_state`, which never shows the stereotype.
    if diagram.get_style_name() == SName::StateDiagram && g.get_usymbol().is_none() {
        return empty();
    }
    let visible_stereotypes = diagram
        .get_visible_stereotype_labels(g.id())
        .unwrap_or_default();
    if stereotype.labels().is_empty() || visible_stereotypes.is_empty() {
        return empty();
    }
    let style = get_default_style_definition(
        diagram.get_style_name(),
        g.get_usymbol(),
        g.get_group_type(),
    )
    .get_merged_style_for_stereotype_itself(
        &diagram.skin().current_style_builder(),
        Some(stereotype),
    );
    Rc::new(Display::create(visible_stereotypes).create0(
        &style.font_configuration(),
        title_horizontal_alignment,
        diagram.skin(),
        0.0,
        CreoleMode::Full,
    ))
}
