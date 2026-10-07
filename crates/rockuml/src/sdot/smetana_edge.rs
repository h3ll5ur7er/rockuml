//! A link drawn along the route Smetana gave its edge, with its decorations and labels (PlantUML's
//! `SmetanaEdge`).

use std::f64::consts::PI;

use smetana::{EdgeLayout, Label};

use super::{BoxInfo, YMirror};
use crate::abel::LinkId;
use crate::color::{ColorType, HColor};
use crate::decoration::{LinkDecor, Rainbow};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::dot_path::DotPath;
use crate::klimt::geom::{UTranslate, XCubicCurve2D, XPoint2D};
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::ugraphic::UGraphic;
use crate::style::{SName, Style, StyleSignature};
use crate::svek::{Bibliotekon, Cluster};

pub(crate) struct SmetanaEdge {
    link: LinkId,
    edge: EdgeLayout,
    ymirror: YMirror,
    label: Box<dyn TextBlock>,
    tail_label: Option<Box<dyn TextBlock>>,
    head_label: Option<Box<dyn TextBlock>>,
    tail_role: Option<Box<dyn TextBlock>>,
    head_role: Option<Box<dyn TextBlock>>,
}

/// The texts around a link: its label, the texts at its ends, and the roles drawn next to those.
pub(crate) struct EdgeTexts {
    pub label: Box<dyn TextBlock>,
    pub tail_label: Option<Box<dyn TextBlock>>,
    pub head_label: Option<Box<dyn TextBlock>>,
    pub tail_role: Option<Box<dyn TextBlock>>,
    pub head_role: Option<Box<dyn TextBlock>>,
}

impl SmetanaEdge {
    pub(crate) fn new(link: LinkId, edge: EdgeLayout, ymirror: YMirror, texts: EdgeTexts) -> Self {
        Self {
            link,
            edge,
            ymirror,
            label: texts.label,
            tail_label: texts.tail_label,
            head_label: texts.head_label,
            tail_role: texts.tail_role,
            head_role: texts.head_role,
        }
    }

    pub(crate) fn draw_u(&self, ug: &UGraphic, diagram: &CucaDiagram, bibliotekon: &Bibliotekon) {
        let link = diagram.link(self.link);
        if link.is_hidden(diagram) {
            return;
        }
        let entity1 = diagram.entity(link.get_entity1());
        let entity2 = diagram.entity(link.get_entity2());
        let (name1, name2) = (entity1.get_name(diagram), entity2.get_name(diagram));
        let mut group = UGroup::default();
        group.put(UGroupType::Class, "link");
        group.put(UGroupType::Id, &format!("link_{name1}_{name2}"));
        group.put(UGroupType::DataUid, link.get_uid());
        group.put(UGroupType::DataEntity1, name1);
        group.put(UGroupType::DataEntity2, name2);
        group.put(UGroupType::DataEntity1Uid, entity1.get_uid());
        group.put(UGroupType::DataEntity2Uid, entity2.get_uid());
        if let Some(link_type_name) = link.get_type().get_link_type_name() {
            group.put(UGroupType::DataLinkType, link_type_name);
        }
        ug.start_group(&group);

        // Labels are drawn in the line's colours once there is a line.
        let mut ug = ug.clone();
        let dot_path = self.get_dot_path_internal().map(|dot_path| {
            let dot_path = self.ymirror.get_mirrored_path(&dot_path);
            let cluster_area = |entity: &crate::abel::Entity| {
                entity.is_group().then(|| {
                    bibliotekon
                        .get_cluster(entity.id())
                        .expect("a group linked to has a cluster")
                })
            };
            let (cluster1, cluster2) = (cluster_area(entity1), cluster_area(entity2));
            let mut dot_path = dot_path.simulate_compound(
                cluster2.map(Cluster::get_rectangle_area),
                cluster1.map(Cluster::get_rectangle_area),
            );
            let string_bounder = ug.string_bounder();
            if let Some(cluster1) = cluster1 {
                let force = cluster1.get_magnetic_border_force_at(
                    string_bounder,
                    dot_path.get_start_point(),
                    diagram,
                );
                dot_path.move_start_point(force);
            }
            if let Some(cluster2) = cluster2 {
                let force = cluster2.get_magnetic_border_force_at(
                    string_bounder,
                    dot_path.get_end_point(),
                    diagram,
                );
                dot_path.move_end_point(force);
            }
            ug = self.draw_line(&ug, &mut dot_path, diagram);
            dot_path
        });

        for (label, text) in [
            (self.edge.label.as_ref(), Some(&self.label)),
            (self.edge.head_label.as_ref(), self.head_label.as_ref()),
            (self.edge.tail_label.as_ref(), self.tail_label.as_ref()),
        ] {
            if let (Some(translate), Some(text)) = (self.get_label_rectangle_translate(label), text)
            {
                text.draw_u(&ug.translated(translate.dx, translate.dy));
            }
        }

        if let Some(dot_path) = dot_path {
            // PlantUML mirrors the already mirrored path once more to place the roles.
            let path_for_roles = self.ymirror.get_mirrored_path(&dot_path);
            if let (Some(tail_role), Some(tail_label), Some(tail_tr)) = (
                &self.tail_role,
                &self.tail_label,
                self.get_label_rectangle_translate(self.edge.tail_label.as_ref()),
            ) {
                draw_role_label(
                    &ug,
                    tail_role.as_ref(),
                    tail_label.as_ref(),
                    XPoint2D::new(tail_tr.dx, tail_tr.dy),
                    path_for_roles.get_start_point(),
                    path_for_roles.get_end_point(),
                );
            }
            if let (Some(head_role), Some(head_label), Some(head_tr)) = (
                &self.head_role,
                &self.head_label,
                self.get_label_rectangle_translate(self.edge.head_label.as_ref()),
            ) {
                draw_role_label(
                    &ug,
                    head_role.as_ref(),
                    head_label.as_ref(),
                    XPoint2D::new(head_tr.dx, head_tr.dy),
                    path_for_roles.get_end_point(),
                    path_for_roles.get_start_point(),
                );
            }
        }
        ug.close_group();
    }

    /// The line and the decorations at its ends, which move its ends back by their length; the surface in the
    /// line's colours.
    fn draw_line(&self, ug: &UGraphic, dot_path: &mut DotPath, diagram: &CucaDiagram) -> UGraphic {
        let link = diagram.link(self.link);
        let style_line = get_style(diagram, link.stereotype.as_ref());
        let rainbow = Rainbow::build_from_style(&style_line);
        let colors = link.get_colors();
        let color = colors
            .get(ColorType::Arrow)
            .or_else(|| colors.get(ColorType::Line))
            .unwrap_or_else(|| rainbow.get_color())
            .clone();
        let ug = ug.with_backcolor(HColor::NONE).with_color(color.clone());
        let link_type = link.get_type();
        let stroke = colors.get_specific_line_stroke().unwrap_or_else(|| {
            if link_type.get_style().is_normal() {
                link_type.get_stroke3(Some(style_line.stroke()))
            } else {
                link_type.get_stroke3(diagram.skin().get_thickness("arrow"))
            }
        });
        if let Some(url) = &link.url {
            ug.start_url(url);
        }
        let with_fill = |decor: LinkDecor| {
            let ug = ug.with_color(color.clone());
            if decor.is_fill() {
                ug.with_backcolor(color.clone())
            } else {
                ug
            }
        };
        let background = diagram.skin().get_background_color();
        print_extremity_at_start(
            dot_path,
            &with_fill(link_type.get_decor2()),
            link_type.get_decor2(),
            &background,
        );
        print_extremity_at_end(
            dot_path,
            &with_fill(link_type.get_decor1()),
            link_type.get_decor1(),
            &background,
        );
        ug.with_stroke(stroke)
            .with_color(color)
            .draw(&dot_path.to_u_path());
        if link.url.is_some() {
            ug.close_url();
        }
        ug
    }

    /// Where the route starts, in drawing coordinates.
    pub(crate) fn get_start_point(&self) -> Option<XPoint2D> {
        let dot_path = self.get_dot_path_internal()?;
        Some(self.ymirror.get_mirrored(dot_path.get_start_point()))
    }

    /// Where the route ends, in drawing coordinates.
    pub(crate) fn get_end_point(&self) -> Option<XPoint2D> {
        let dot_path = self.get_dot_path_internal()?;
        Some(self.ymirror.get_mirrored(dot_path.get_end_point()))
    }

    fn get_label_rectangle_translate(&self, label: Option<&Label>) -> Option<UTranslate> {
        let box_info = BoxInfo::from_textlabel(label?);
        Some(
            self.ymirror
                .get_mirrored_translate(UTranslate::point(box_info.get_lower_left())),
        )
    }

    /// The first piece of the route, in Graphviz coordinates.
    fn get_dot_path_internal(&self) -> Option<DotPath> {
        let points = &self.edge.beziers.first()?.points;
        let point = |i: usize| XPoint2D::new(points[i].x, points[i].y);
        let mut dot_path = DotPath::default().add_curve(XCubicCurve2D::new(
            point(0),
            point(1),
            point(2),
            point(3),
        ));
        for i in (4..points.len()).step_by(3) {
            dot_path = dot_path.add_curve_from_end(point(i), point(i + 1), point(i + 2));
        }
        Some(dot_path)
    }
}

/// The style of the diagram's arrows with the link's stereotype.
fn get_style(diagram: &CucaDiagram, stereotype: Option<&crate::stereo::Stereotype>) -> Style {
    StyleSignature::of(&[
        SName::Root,
        SName::Element,
        diagram.get_style_name(),
        SName::Arrow,
    ])
    .get_merged_style_with(&diagram.skin().current_style_builder(), stereotype)
}

fn print_extremity_at_start(
    dot_path: &mut DotPath,
    ug: &UGraphic,
    decor: LinkDecor,
    background: &HColor,
) {
    let Some(factory) = decor.get_extremity_factory_complete(background.clone()) else {
        return;
    };
    let start_angle = dot_path.get_start_angle() + PI;
    let extremity = factory.create_udrawable(dot_path.get_start_point(), start_angle);
    dot_path.move_start_point(
        UTranslate::new(extremity.decoration_length(), 0.0).rotate(start_angle - PI),
    );
    extremity.draw_u(ug);
}

fn print_extremity_at_end(
    dot_path: &mut DotPath,
    ug: &UGraphic,
    decor: LinkDecor,
    background: &HColor,
) {
    let Some(factory) = decor.get_extremity_factory_complete(background.clone()) else {
        return;
    };
    let end_angle = dot_path.get_end_angle();
    let extremity = factory.create_udrawable(dot_path.get_end_point(), end_angle);
    dot_path
        .move_end_point(UTranslate::new(extremity.decoration_length(), 0.0).rotate(end_angle - PI));
    extremity.draw_u(ug);
}

/// A role beside the quantifier at the same end, on the side of the line away from it.
fn draw_role_label(
    ug: &UGraphic,
    role: &dyn TextBlock,
    quantifier: &dyn TextBlock,
    quantifier_pos: XPoint2D,
    this_endpoint: XPoint2D,
    other_endpoint: XPoint2D,
) {
    let string_bounder = ug.string_bounder();
    let q_dim = quantifier.calculate_dimension(string_bounder);
    let r_dim = role.calculate_dimension(string_bounder);
    let dir_x = other_endpoint.x - this_endpoint.x;
    let dir_y = other_endpoint.y - this_endpoint.y;
    if dir_x.abs() + dir_y.abs() < 0.001 {
        role.draw_u(&ug.translated(quantifier_pos.x, quantifier_pos.y + q_dim.height));
        return;
    }
    let gap = 2.0;
    let (role_x, role_y);
    if dir_y.abs() >= dir_x.abs() {
        let q_center_x = quantifier_pos.x + q_dim.width / 2.0;
        let line_x = this_endpoint.x;
        role_x = if q_center_x < line_x {
            line_x + gap
        } else {
            line_x - r_dim.width - gap
        };
        role_y = quantifier_pos.y;
    } else {
        let q_center_y = quantifier_pos.y + q_dim.height / 2.0;
        let line_y = this_endpoint.y;
        role_y = if q_center_y < line_y {
            line_y + gap
        } else {
            line_y - r_dim.height - gap
        };
        role_x = quantifier_pos.x;
    }
    role.draw_u(&ug.translated(role_x, role_y));
}
