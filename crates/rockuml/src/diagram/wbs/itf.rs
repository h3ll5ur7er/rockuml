//! The layout of a work breakdown: the root's children side by side below it (`Fork`), each with its own
//! children stacked below it to the left and right (`ITFComposed`) down to the leaves (`ITFLeaf`).

use std::cell::Cell;

use super::element::{ElementId, Elements};
use crate::color::{ColorType, Colors};
use crate::creole::CreoleMode;
use crate::diagram::mindmap::IdeaShape;
use crate::ftile::vertical::FtileBoxOld;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, UTranslate, XDimension2D, XPoint2D};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::style::{PName, SName, StyleSignature, ValueReading};

/// What the layout reads the elements from.
#[derive(Clone, Copy)]
pub(super) struct Context<'a> {
    pub(super) elements: &'a Elements,
    pub(super) skin_param: &'a SkinParam,
}

/// A subtree below its parent's line (PlantUML's `ITF`): `t1` and `t2` are the middle of its top and bottom,
/// `f1` and `f2` the middle of its left and right side.
pub(super) trait Itf {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D;

    fn get_t1(&self, string_bounder: &dyn StringBounder) -> XPoint2D;

    fn get_f1(&self, string_bounder: &dyn StringBounder) -> XPoint2D;

    fn get_f2(&self, string_bounder: &dyn StringBounder) -> XPoint2D;

    fn get_main_box_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.calculate_dimension(string_bounder).width
    }

    fn set_forced_min_width(&self, _width: f64) {}

    /// Draws the subtree on `ug`, which is `origin` away from the diagram's origin.
    fn draw_at(&self, ug: &UGraphic, origin: UTranslate);
}

/// The children of an element, laid out (`ITFComposed.build2`).
fn build2<'a>(context: Context<'a>, id: ElementId) -> Box<dyn Itf + 'a> {
    let element = context.elements.get(id);
    if element.is_leaf() {
        return Box::new(ItfLeaf::new(context, id));
    }
    let build_all = |children: &[ElementId]| {
        children
            .iter()
            .map(|&child| build2(context, child))
            .collect()
    };
    Box::new(ItfComposed::new(
        context,
        id,
        build_all(&element.children_left),
        build_all(&element.children_right),
    ))
}

/// What every block of the layout shares (PlantUML's `WBSTextBlock`).
#[derive(Clone, Copy)]
struct WbsTextBlock<'a> {
    context: Context<'a>,
    id: ElementId,
}

impl<'a> WbsTextBlock<'a> {
    /// Whether the children's boxes take the width of the widest (`Width auto`).
    fn is_auto_width(self) -> bool {
        self.context
            .elements
            .style(self.id)
            .value(PName::Width)
            .as_string()
            .eq_ignore_ascii_case("auto")
    }

    /// Draws a line in the style of the element's links, from the leftmost to the rightmost x.
    fn draw_line(self, ug: &UGraphic, x1: f64, y1: f64, x2: f64, y2: f64) {
        let element = self.context.elements.get(self.id);
        let style =
            StyleSignature::of(&[SName::Root, SName::Element, SName::WbsDiagram, SName::Arrow])
                .with_level(super::level_of(element.level))
                .get_merged_style_with(&element.style_builder, element.stereotype.as_ref());
        let ug = ug
            .apply(style.value(PName::LineColor).as_color())
            .apply(style.stroke());
        let (p1x, p2x) = (x1.min(x2), x1.max(x2));
        ug.translated(p1x, y1).draw(&UShape::Line {
            dx: p2x - p1x,
            dy: y2 - y1,
        });
    }

    /// The element's own box, or text, or nothing (`buildMain`).
    fn build_main(self) -> Main<'a> {
        let element = self.context.elements.get(self.id);
        let style = self.context.elements.style(self.id);
        match element.shape {
            IdeaShape::Pseudo => Main::Empty(TextBlockEmpty::default()),
            IdeaShape::Box => {
                let colors = Colors::default().with(ColorType::Back, element.back_color.clone());
                Main::Box(FtileBoxOld::create_wbs(
                    &style,
                    self.context.skin_param,
                    &colors,
                    &element.label,
                ))
            }
            IdeaShape::None => {
                let text = element.label.create0(
                    &style.font_configuration(),
                    style
                        .horizontal_alignment()
                        .unwrap_or(HorizontalAlignment::Left),
                    self.context.skin_param,
                    style.wrap_width(),
                    CreoleMode::Full,
                );
                Main::Text(Box::new(TextBlockMarged::new(
                    text,
                    ClockwiseTopRightBottomLeft::top_right_bottom_left(1.0, 3.0, 1.0, 0.0),
                )))
            }
        }
    }
}

/// An element's own block; only boxes widen to their siblings'.
enum Main<'a> {
    Empty(TextBlockEmpty),
    Box(FtileBoxOld),
    Text(Box<dyn TextBlock + 'a>),
}

impl Main<'_> {
    fn block(&self) -> &dyn TextBlock {
        match self {
            Main::Empty(empty) => empty,
            Main::Box(fbox) => fbox,
            Main::Text(text) => text.as_ref(),
        }
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block().calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.block().draw_u(ug);
    }
}

/// Makes the boxes of `children` as wide as the widest, once, if the element asks for it (`applyAutoWidth`).
fn apply_auto_width(
    applied: &Cell<bool>,
    text_block: WbsTextBlock<'_>,
    children: &[&dyn Itf],
    string_bounder: &dyn StringBounder,
) {
    if applied.replace(true) || !text_block.is_auto_width() {
        return;
    }
    let max_width = children
        .iter()
        .map(|child| child.get_main_box_width(string_bounder))
        .fold(0.0, f64::max);
    for child in children {
        child.set_forced_min_width(max_width);
    }
}

/// A childless element (PlantUML's `ITFLeaf`).
struct ItfLeaf<'a> {
    text_block: WbsTextBlock<'a>,
    main: Main<'a>,
    forced_min_width: Cell<f64>,
}

impl<'a> ItfLeaf<'a> {
    fn new(context: Context<'a>, id: ElementId) -> Self {
        let text_block = WbsTextBlock { context, id };
        Self {
            text_block,
            main: text_block.build_main(),
            forced_min_width: Cell::new(0.0),
        }
    }

    fn is_box(&self) -> bool {
        matches!(self.main, Main::Box(_))
    }
}

impl Itf for ItfLeaf<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dimension = self.main.calculate_dimension(string_bounder);
        let forced_min_width = self.forced_min_width.get();
        if !self.is_box() && forced_min_width > dimension.width {
            return XDimension2D::new(forced_min_width, dimension.height);
        }
        dimension
    }

    fn get_t1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let dimension = self.calculate_dimension(string_bounder);
        XPoint2D::new(dimension.width / 2.0, 0.0)
    }

    fn get_f1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let dimension = self.calculate_dimension(string_bounder);
        XPoint2D::new(0.0, dimension.height / 2.0)
    }

    fn get_f2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let dimension = self.calculate_dimension(string_bounder);
        XPoint2D::new(dimension.width, dimension.height / 2.0)
    }

    fn set_forced_min_width(&self, width: f64) {
        self.forced_min_width.set(width);
        if let Main::Box(fbox) = &self.main {
            fbox.set_minimum_width(width);
        }
    }

    fn draw_at(&self, ug: &UGraphic, origin: UTranslate) {
        let string_bounder = ug.string_bounder();
        self.text_block
            .context
            .elements
            .get(self.text_block.id)
            .set_geometry(origin, self.calculate_dimension(string_bounder));
        let forced_min_width = self.forced_min_width.get();
        if !self.is_box() && forced_min_width > 0.0 {
            let box_dimension = self.main.calculate_dimension(string_bounder);
            if forced_min_width > box_dimension.width {
                let dx = (forced_min_width - box_dimension.width) / 2.0;
                self.main.draw_u(&ug.translated(dx, 0.0));
                return;
            }
        }
        self.main.draw_u(ug);
    }
}

/// An element with children, hanging below it to the left and right (PlantUML's `ITFComposed`).
struct ItfComposed<'a> {
    text_block: WbsTextBlock<'a>,
    left: Vec<Box<dyn Itf + 'a>>,
    right: Vec<Box<dyn Itf + 'a>>,
    main: Main<'a>,
    margin_bottom: f64,
    auto_width_applied: Cell<bool>,
}

const DELTA1X: f64 = 10.0;

impl<'a> ItfComposed<'a> {
    fn new(
        context: Context<'a>,
        id: ElementId,
        left: Vec<Box<dyn Itf + 'a>>,
        right: Vec<Box<dyn Itf + 'a>>,
    ) -> Self {
        let text_block = WbsTextBlock { context, id };
        let margin_bottom = if context.elements.get(id).shape == IdeaShape::Pseudo {
            0.0
        } else {
            context.elements.style(id).margin().bottom
        };
        Self {
            text_block,
            left,
            right,
            main: text_block.build_main(),
            margin_bottom,
            auto_width_applied: Cell::new(false),
        }
    }

    fn apply_auto_width(&self, string_bounder: &dyn StringBounder) {
        let children: Vec<&dyn Itf> = self
            .left
            .iter()
            .chain(&self.right)
            .map(AsRef::as_ref)
            .collect();
        apply_auto_width(
            &self.auto_width_applied,
            self.text_block,
            &children,
            string_bounder,
        );
    }

    fn getw1(&self, string_bounder: &dyn StringBounder) -> f64 {
        let main_width = self.main.calculate_dimension(string_bounder).width;
        (main_width / 2.0).max(DELTA1X + coll_width(string_bounder, &self.left))
    }
}

impl Itf for ItfComposed<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.apply_auto_width(string_bounder);
        let main_dim = self.main.calculate_dimension(string_bounder);
        let main_width = main_dim.width;
        let height = main_dim.height
            + coll_height(string_bounder, &self.left, self.margin_bottom).max(coll_height(
                string_bounder,
                &self.right,
                self.margin_bottom,
            ));
        let width = (main_width / 2.0).max(DELTA1X + coll_width(string_bounder, &self.left))
            + (main_width / 2.0).max(DELTA1X + coll_width(string_bounder, &self.right));
        XDimension2D::new(width, height)
    }

    fn get_t1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        XPoint2D::new(self.getw1(string_bounder), 0.0)
    }

    fn get_f1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let main_dim = self.main.calculate_dimension(string_bounder);
        XPoint2D::new(
            self.getw1(string_bounder) - main_dim.width / 2.0,
            main_dim.height / 2.0,
        )
    }

    fn get_f2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let main_dim = self.main.calculate_dimension(string_bounder);
        XPoint2D::new(
            self.getw1(string_bounder) + main_dim.width / 2.0,
            main_dim.height / 2.0,
        )
    }

    fn get_main_box_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.main.calculate_dimension(string_bounder).width
    }

    fn set_forced_min_width(&self, width: f64) {
        if let Main::Box(fbox) = &self.main {
            fbox.set_minimum_width(width);
        }
    }

    // PlantUML also works out a link colour here, which `draw_line` then replaces with its own.
    fn draw_at(&self, ug: &UGraphic, origin: UTranslate) {
        self.apply_auto_width(ug.string_bounder());
        let string_bounder = ug.string_bounder();
        let main_dim = self.main.calculate_dimension(string_bounder);
        self.text_block
            .context
            .elements
            .get(self.text_block.id)
            .set_geometry(origin, main_dim);
        let wx = self.getw1(string_bounder) - main_dim.width / 2.0;
        self.main.draw_u(&ug.translated(wx, 0.0));
        let x = self.getw1(string_bounder);
        let mut y = main_dim.height;
        let mut last_y1 = y;
        for child in &self.left {
            y += self.margin_bottom;
            let child_dim = child.calculate_dimension(string_bounder);
            let f2 = child.get_f2(string_bounder);
            last_y1 = y + f2.y;
            let child_x = x - child_dim.width - DELTA1X;
            self.text_block
                .draw_line(ug, child_x + f2.x, last_y1, x, last_y1);
            child.draw_at(
                &ug.translated(child_x, y),
                origin.compose(UTranslate::new(child_x, y)),
            );
            y += child_dim.height;
        }
        y = main_dim.height;
        let mut last_y2 = y;
        for child in &self.right {
            y += self.margin_bottom;
            let child_dim = child.calculate_dimension(string_bounder);
            let f1 = child.get_f1(string_bounder);
            last_y2 = y + f1.y;
            self.text_block
                .draw_line(ug, x, last_y2, x + DELTA1X + f1.x, last_y2);
            child.draw_at(
                &ug.translated(x + DELTA1X, y),
                origin.compose(UTranslate::new(x + DELTA1X, y)),
            );
            y += child_dim.height;
        }
        self.text_block
            .draw_line(ug, x, main_dim.height, x, last_y1.max(last_y2));
    }
}

fn coll_width(string_bounder: &dyn StringBounder, all: &[Box<dyn Itf + '_>]) -> f64 {
    all.iter()
        .map(|child| child.calculate_dimension(string_bounder).width)
        .fold(0.0, f64::max)
}

fn coll_height(string_bounder: &dyn StringBounder, all: &[Box<dyn Itf + '_>], deltay: f64) -> f64 {
    all.iter()
        .map(|child| deltay + child.calculate_dimension(string_bounder).height)
        .sum()
}

/// The root with its first-level elements side by side below it (PlantUML's `Fork`).
pub(super) struct Fork<'a> {
    text_block: WbsTextBlock<'a>,
    main: Main<'a>,
    right: Vec<Box<dyn Itf + 'a>>,
    auto_width_applied: Cell<bool>,
}

impl<'a> Fork<'a> {
    const DELTA1X: f64 = 20.0;
    const DELTAY: f64 = 40.0;

    pub(super) fn new(context: Context<'a>) -> Self {
        let text_block = WbsTextBlock {
            context,
            id: Elements::ROOT,
        };
        let root = context.elements.get(Elements::ROOT);
        Self {
            text_block,
            main: text_block.build_main(),
            right: Self::children(context, &root.children_right),
            auto_width_applied: Cell::new(false),
        }
    }

    fn children(context: Context<'a>, children: &[ElementId]) -> Vec<Box<dyn Itf + 'a>> {
        children
            .iter()
            .map(|&child| build2(context, child))
            .collect()
    }

    fn apply_auto_width(&self, string_bounder: &dyn StringBounder) {
        let children: Vec<&dyn Itf> = self.right.iter().map(AsRef::as_ref).collect();
        apply_auto_width(
            &self.auto_width_applied,
            self.text_block,
            &children,
            string_bounder,
        );
    }

    pub(super) fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.apply_auto_width(string_bounder);
        let mut width = 0.0;
        let mut height: f64 = 0.0;
        for child in &self.right {
            let child_dim = child.calculate_dimension(string_bounder);
            height = height.max(child_dim.height);
            width += child_dim.width;
        }
        if self.right.len() > 1 {
            width += (self.right.len() - 1) as f64 * Self::DELTA1X;
        }
        let main_dim = self.main.calculate_dimension(string_bounder);
        height += main_dim.height;
        height += Self::DELTAY;
        width = f64::max(width, main_dim.width);
        XDimension2D::new(width, height)
    }

    /// Draws the root and its subtrees; `ug` is the diagram's origin.
    pub(super) fn draw_u(&self, ug: &UGraphic) {
        self.apply_auto_width(ug.string_bounder());
        let string_bounder = ug.string_bounder();
        let main_dim = self.main.calculate_dimension(string_bounder);
        let y0 = main_dim.height;
        let y1 = y0 + Self::DELTAY / 2.0;
        let y2 = y0 + Self::DELTAY;
        let main_width = main_dim.width;
        let line = |x1, y1, x2, y2| self.text_block.draw_line(ug, x1, y1, x2, y2);
        if self.right.is_empty() {
            self.main.draw_u(ug);
            line(main_width / 2.0, y0, main_width / 2.0, y1);
            return;
        }
        let mut x = 0.0;
        let first_x = self.right[0].get_t1(string_bounder).x;
        let mut last_x = first_x;
        for child in &self.right {
            last_x = x + child.get_t1(string_bounder).x;
            line(last_x, y1, last_x, y2);
            child.draw_at(&ug.translated(x, y2), UTranslate::new(x, y2));
            x += child.calculate_dimension(string_bounder).width + Self::DELTA1X;
        }
        let pos_main = if last_x > first_x {
            line(first_x, y1, last_x, y1);
            first_x + (last_x - first_x - main_width) / 2.0
        } else {
            let full_dim = self.calculate_dimension(string_bounder);
            let pos_main = (full_dim.width - main_width) / 2.0;
            line(first_x, y1, pos_main + main_width / 2.0, y1);
            pos_main
        };
        self.main.draw_u(&ug.translated(pos_main, 0.0));
        line(
            pos_main + main_width / 2.0,
            y0,
            pos_main + main_width / 2.0,
            y1,
        );
    }
}
