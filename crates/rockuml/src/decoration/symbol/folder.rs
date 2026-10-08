//! A folder with a tab, which shows the name of packages (PlantUML's `USymbolFolder`). Links ending on the tab's
//! row are pulled down onto the folder.

use std::rc::Rc;

use super::{BigContent, BigShape, Block, Margin, SmallContent};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};

pub(super) struct USymbolFolder;

const MARGIN_TITLE_X1: f64 = 3.0;
const MARGIN_TITLE_X2: f64 = 3.0;
const MARGIN_TITLE_X3: f64 = 7.0;
const MARGIN_TITLE_Y1: f64 = 3.0;
const MARGIN_TITLE_Y2: f64 = 3.0;

const MARGIN: Margin = Margin::new(10.0, 10.0 + 10.0, 10.0 + 3.0, 10.0);

/// The size taken by the tab of a folder without a title.
const UNTITLED: XDimension2D = XDimension2D::new(40.0, 15.0);

fn draw_folder(ug: &UGraphic, width: f64, height: f64, dim_title: XDimension2D, round_corner: f64) {
    let wtitle = get_w_title(width, dim_title);
    let htitle = get_h_title(dim_title);
    if round_corner == 0.0 {
        ug.draw(&UShape::polygon(vec![
            (0.0, 0.0),
            (wtitle, 0.0),
            (wtitle + MARGIN_TITLE_X3, htitle),
            (width, htitle),
            (width, height),
            (0.0, height),
            (0.0, 0.0),
        ]));
    } else {
        let r = round_corner / 2.0;
        ug.draw(&UShape::path(vec![
            USegment::MoveTo(r, 0.0),
            USegment::LineTo(wtitle - r, 0.0),
            USegment::arc_to((wtitle, r), r * 1.5, true),
            USegment::LineTo(wtitle + MARGIN_TITLE_X3, htitle),
            USegment::LineTo(width - r, htitle),
            USegment::arc_to((width, htitle + r), r, true),
            USegment::LineTo(width, height - r),
            USegment::arc_to((width - r, height), r, true),
            USegment::LineTo(r, height),
            USegment::arc_to((0.0, height - r), r, true),
            USegment::LineTo(0.0, r),
            USegment::arc_to((r, 0.0), r, true),
        ]));
    }
    ug.translated(0.0, htitle).draw(&UShape::Line {
        dx: wtitle + MARGIN_TITLE_X3,
        dy: 0.0,
    });
}

/// The tab's width: a quarter of the folder's, at least 30, without a title.
fn get_w_title(width: f64, dim_title: XDimension2D) -> f64 {
    if dim_title.width == 0.0 {
        30.0_f64.max(width / 4.0)
    } else {
        dim_title.width + MARGIN_TITLE_X1 + MARGIN_TITLE_X2
    }
}

fn get_h_title(dim_title: XDimension2D) -> f64 {
    if dim_title.width == 0.0 {
        10.0
    } else {
        dim_title.height + MARGIN_TITLE_Y1 + MARGIN_TITLE_Y2
    }
}

/// How far a link ending at `position` on the top edge moves down onto the folder: past the tab, by the tab's
/// height; along its sloping side, by part of it.
fn get_force_at(width: f64, dim_title: XDimension2D, position: XPoint2D) -> UTranslate {
    let wtitle = get_w_title(width, dim_title);
    let htitle = get_h_title(dim_title);
    if position.x >= wtitle && position.y >= 0.0 && position.y <= htitle {
        return UTranslate::new(0.0, htitle);
    }
    if position.y <= 0.0 && position.x >= wtitle + MARGIN_TITLE_X3 {
        return UTranslate::new(0.0, htitle);
    }
    if position.y <= 0.0 && position.x >= wtitle - MARGIN_TITLE_X3 {
        let delta = position.x - (wtitle - MARGIN_TITLE_X3);
        let how = delta / (2.0 * MARGIN_TITLE_X3);
        return UTranslate::new(0.0, htitle * how);
    }
    UTranslate::default()
}

pub(super) fn as_small(
    title: Block,
    show_title: bool,
    content: SmallContent,
) -> Box<dyn TextBlock> {
    Box::new(SmallFolder {
        title,
        show_title,
        content,
    })
}

struct SmallFolder {
    title: Block,
    show_title: bool,
    content: SmallContent,
}

impl SmallFolder {
    fn get_dim_title(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        if self.show_title {
            self.title.calculate_dimension(string_bounder)
        } else {
            UNTITLED
        }
    }
}

impl TextBlock for SmallFolder {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_name = self.get_dim_title(string_bounder);
        let dim_label = self.content.label.calculate_dimension(string_bounder);
        let dim_stereo = self.content.stereotype.calculate_dimension(string_bounder);
        MARGIN.add_dimension(XDimension2D::new(
            dim_name.width.max(dim_stereo.width).max(dim_label.width),
            dim_name.height + dim_stereo.height + dim_label.height,
        ))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.calculate_dimension(ug.string_bounder());
        let ug = self
            .content
            .fashion
            .apply(ug)
            .with_stencil(Rc::new(RectangleStencil {
                width: dimension.width,
            }));
        let dim_title = self.get_dim_title(ug.string_bounder());
        draw_folder(
            &ug,
            dimension.width,
            dimension.height,
            dim_title,
            self.content.fashion.round_corner,
        );
        if self.show_title {
            self.title.draw_u(&ug.translated(4.0, 3.0));
        }
        self.content
            .text(HorizontalAlignment::Center)
            .draw_u(&ug.translated(MARGIN.x1, MARGIN.y1 + dim_title.height));
    }

    fn magnetic_border_force_at(
        &self,
        string_bounder: &dyn StringBounder,
        position: XPoint2D,
    ) -> UTranslate {
        let dimension = self.calculate_dimension(string_bounder);
        get_force_at(
            dimension.width,
            self.get_dim_title(string_bounder),
            position,
        )
    }
}

impl BigShape for USymbolFolder {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        let string_bounder = ug.string_bounder();
        let dim_title = content.title.calculate_dimension(string_bounder);
        draw_folder(
            ug,
            content.width,
            content.height,
            dim_title,
            content.fashion.round_corner,
        );
        content.title.draw_u(&ug.translated(4.0, 2.0));
        let dim_stereo = content.stereotype.calculate_dimension(string_bounder);
        content.stereotype.draw_u(&ug.translated(
            4.0 + (content.width - dim_stereo.width) / 2.0,
            2.0 + get_h_title(dim_title),
        ));
    }

    fn magnetic_border_force_at(
        &self,
        content: &BigContent,
        string_bounder: &dyn StringBounder,
        position: XPoint2D,
    ) -> UTranslate {
        get_force_at(
            content.width,
            content.title.calculate_dimension(string_bounder),
            position,
        )
    }
}
