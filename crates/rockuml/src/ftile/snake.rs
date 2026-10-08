//! An arrow of an activity diagram (PlantUML's `Snake`): a worm in one or more colours, arrowheads at its
//! ends, and labels.
//!
//! Tiles draw arrows with `ug.draw(&snake)`; `UGraphicForSnake` collects them, merges arrows that continue
//! each other, and draws them all at the end with [`Snake::draw_internal`].

use std::cell::RefCell;
use std::rc::Rc;

use super::{MergeStrategy, Worm, WormMutation};
use crate::decoration::{HtmlColorAndStyle, Rainbow};
use crate::direction::Direction;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::shape::UPolygon;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::skin::SkinParam;
use crate::style::{SName, StyleSignature};

#[derive(Clone)]
struct Text {
    block: Rc<dyn TextBlock>,
    vertical_alignment: Option<VerticalAlignment>,
    horizontal_alignment: Option<HorizontalAlignment>,
}

impl Text {
    fn has_text(&self, string_bounder: &dyn StringBounder) -> bool {
        let dim = self.block.calculate_dimension(string_bounder);
        !(dim.height == 0.0 && dim.width == 0.0)
    }
}

#[derive(Clone)]
pub(crate) struct Snake {
    worm: Worm,
    start_decoration: Option<UPolygon>,
    end_decoration: Option<UPolygon>,
    color: Rainbow,
    /// Shared with the copies `move_by` and the `with_` methods make, as in PlantUML, where adding a label
    /// to one adds it to all.
    texts: Rc<RefCell<Vec<Text>>>,
    mergeable: MergeStrategy,
    emphasize_direction: Option<Direction>,
}

impl Snake {
    /// An arrow without arrowheads in `color`, which must have colours.
    pub(crate) fn create(skin_param: &SkinParam, color: Rainbow) -> Self {
        Self::create_with_decorations(skin_param, None, color, None)
    }

    /// An arrow in `color` ending in `end_decoration`.
    pub(crate) fn create_with_end(
        skin_param: &SkinParam,
        color: Rainbow,
        end_decoration: UPolygon,
    ) -> Self {
        Self::create_with_decorations(skin_param, None, color, Some(end_decoration))
    }

    pub(crate) fn create_with_decorations(
        skin_param: &SkinParam,
        start_decoration: Option<UPolygon>,
        color: Rainbow,
        end_decoration: Option<UPolygon>,
    ) -> Self {
        debug_assert!(
            color.size() > 0,
            "PlantUML refuses an arrow without colours"
        );
        let style = skin_param
            .merged_style(&StyleSignature::of(&[
                SName::Root,
                SName::Element,
                SName::ActivityDiagram,
                SName::Activity,
                SName::Arrow,
            ]))
            .expect("the skin styles activity arrows");
        Self {
            worm: Worm::new(style.stroke(), skin_param.arrows()),
            start_decoration,
            end_decoration,
            color,
            texts: Rc::new(RefCell::new(Vec::new())),
            mergeable: MergeStrategy::Full,
            emphasize_direction: None,
        }
    }

    /// The same arrow `dx` and `dy` further (`move`).
    #[must_use]
    pub(crate) fn move_by(&self, dx: f64, dy: f64) -> Self {
        Self {
            worm: self.worm.move_by(dx, dy),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn translate(&self, translate: UTranslate) -> Self {
        self.move_by(translate.dx, translate.dy)
    }

    /// Arrowheads take no room when compressing across; only before points are added.
    #[must_use]
    pub(crate) fn ignore_for_compression(mut self) -> Self {
        self.worm.set_ignore_for_compression();
        self
    }

    /// The first segment going `emphasize_direction` gets an arrowhead in its middle.
    #[must_use]
    pub(crate) fn emphasize_direction(&self, emphasize_direction: Direction) -> Self {
        Self {
            emphasize_direction: Some(emphasize_direction),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn without_end_decoration(&self) -> Self {
        Self {
            end_decoration: None,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_merge(&self, mergeable: MergeStrategy) -> Self {
        Self {
            mergeable,
            ..self.clone()
        }
    }

    /// Adds a label beside the first segment, aligned as arrow labels are (`withLabel(TextBlock,
    /// HorizontalAlignment)`); none for `None`.
    #[must_use]
    pub(crate) fn with_label(
        self,
        block: Option<Rc<dyn TextBlock>>,
        horizontal_alignment: HorizontalAlignment,
    ) -> Self {
        if let Some(block) = block {
            self.texts.borrow_mut().push(Text {
                block,
                vertical_alignment: None,
                horizontal_alignment: Some(horizontal_alignment),
            });
        }
        self
    }

    /// Adds a label below (`Bottom`) or beside the middle (`Center`) of the arrow (`withLabel(TextBlock,
    /// VerticalAlignment)`); none for `None`, which also stands for PlantUML's empty text block.
    #[must_use]
    pub(crate) fn with_label_vertical(
        self,
        block: Option<Rc<dyn TextBlock>>,
        vertical_alignment: VerticalAlignment,
    ) -> Self {
        if let Some(block) = block {
            self.texts.borrow_mut().push(Text {
                block,
                vertical_alignment: Some(vertical_alignment),
                horizontal_alignment: None,
            });
        }
        self
    }

    pub(crate) fn add_point(&mut self, x: f64, y: f64) {
        self.worm.add_point(x, y);
    }

    pub(crate) fn add_point_at(&mut self, point: XPoint2D) {
        self.worm.add_point_at(point);
    }

    /// Draws the arrow now, which only `UGraphicForSnake` and surfaces do: tiles draw arrows with
    /// `ug.draw(&snake)`.
    pub(crate) fn draw_internal(&self, ug: &UGraphic) {
        if self.is_empty() {
            return;
        }
        match self.color.get_colors() {
            [] => {}
            [color] => {
                self.worm.draw_internal_one_color(
                    self.start_decoration.as_ref(),
                    ug,
                    color,
                    1.5,
                    self.emphasize_direction,
                    self.end_decoration.as_ref(),
                );
                self.draw_internal_label(ug);
            }
            _ => self.draw_rainbow(ug),
        }
    }

    /// One line per colour, side by side; without a gap the lines are drawn thick enough to touch.
    fn draw_rainbow(&self, ug: &UGraphic) {
        let color_arrow_separation_space = self.color.get_color_arrow_separation_space();
        let move_ = 2.0 + f64::from(color_arrow_separation_space);
        let mutation = WormMutation::create(&self.worm, move_);
        let mut colors: Vec<&HtmlColorAndStyle> = self.color.get_colors().iter().collect();
        if mutation.is_dx_negative() {
            colors.reverse();
        }
        let global_move = -(colors.len() as f64 - 1.0) / 2.0;
        let mut current = self
            .worm
            .move_first_point(mutation.get_first().multiply_by(global_move));
        if mutation.size() > 2 {
            current = current.move_last_point(mutation.get_last().multiply_by(global_move));
        }
        for (i, color) in colors.iter().enumerate() {
            let stroke = match color_arrow_separation_space {
                0 if i == colors.len() - 1 => 2.0,
                0 => 3.0,
                _ => 1.5,
            };
            current.draw_internal_one_color(
                self.start_decoration.as_ref(),
                ug,
                color,
                stroke,
                self.emphasize_direction,
                self.end_decoration.as_ref(),
            );
            current = mutation.mute(&current);
        }
        let text_translate = mutation.get_text_translate(colors.len());
        self.draw_internal_label(&ug.apply(text_translate));
    }

    fn draw_internal_label(&self, ug: &UGraphic) {
        for text in self.texts.borrow().iter() {
            if text.has_text(ug.string_bounder()) {
                let position = self.get_text_block_position(ug.string_bounder(), text);
                text.block.draw_u(&ug.apply(UTranslate::point(position)));
            }
        }
    }

    /// How far right the arrow and its labels reach.
    pub(crate) fn get_max_x(&self, string_bounder: &dyn StringBounder) -> f64 {
        let mut result = self.worm.get_max_x();
        for text in self.texts.borrow().iter() {
            let position = self.get_text_block_position(string_bounder, text);
            let dim = text.block.calculate_dimension(string_bounder);
            result = result.max(position.x + dim.width);
        }
        result
    }

    fn get_text_block_position(&self, string_bounder: &dyn StringBounder, text: &Text) -> XPoint2D {
        let worm = &self.worm;
        let pt1 = worm.get_point(0);
        let pt2 = worm.get_point(1);
        let dim = text.block.calculate_dimension(string_bounder);
        let code = worm.get_directions_code();
        let zigzag = code.starts_with("DLD") || code.starts_with("DRD");
        let mut x = pt1.x.max(pt2.x) + 4.0;
        let mut y = f64::midpoint(pt1.y, pt2.y) - dim.height / 2.0;
        if text.vertical_alignment == Some(VerticalAlignment::Bottom) {
            x = worm.get_min_x();
            y = worm.get_max_y();
        } else if text.vertical_alignment == Some(VerticalAlignment::Center) {
            x = worm.get_min_x();
            y = (worm.get_first().y + worm.get_last().y - 10.0) / 2.0 - dim.height / 2.0;
        } else if text.horizontal_alignment == Some(HorizontalAlignment::Center) && zigzag {
            let pt3 = worm.get_point(2);
            x = f64::midpoint(pt2.x, pt3.x) - dim.width / 2.0;
        } else if text.horizontal_alignment == Some(HorizontalAlignment::Right) && zigzag {
            x = pt1.x.max(pt2.x) - dim.width - 4.0;
        } else if code == "RD" {
            x = pt1.x.max(pt2.x);
            y = f64::midpoint(pt1.y, worm.get_point(2).y) - dim.height / 2.0;
        } else if code == "LD" {
            x = pt1.x.min(pt2.x);
            y = f64::midpoint(pt1.y, worm.get_point(2).y) - dim.height / 2.0;
        }
        XPoint2D::new(x, y)
    }

    /// An arrow without points, on which PlantUML fails; here it is not drawn.
    pub(crate) fn is_empty(&self) -> bool {
        self.worm.size() == 0
    }

    pub(crate) fn get_first(&self) -> XPoint2D {
        self.worm.get_point(0)
    }

    pub(crate) fn get_last(&self) -> XPoint2D {
        self.worm.get_point(self.worm.size() - 1)
    }

    /// Whether no arrow may lose its arrowhead for ending where this one starts (`cannotBeTouched`).
    pub(crate) fn cannot_be_touched(&self) -> bool {
        self.mergeable != MergeStrategy::Full || self.worm.is_pure_horizontal()
    }

    /// Whether two points are the same, to a thousandth.
    pub(crate) fn same(pt1: XPoint2D, pt2: XPoint2D) -> bool {
        pt1.distance(pt2) < 0.001
    }

    /// This arrow and `other` as one, when one ends where the other starts, both may merge and `other`
    /// shows no label; it keeps the colours and the start of whichever comes first.
    pub(crate) fn merge(&self, other: &Self, string_bounder: &dyn StringBounder) -> Option<Self> {
        let strategy = self.mergeable.max(other.mergeable);
        if strategy == MergeStrategy::None {
            return None;
        }
        if other
            .texts
            .borrow()
            .iter()
            .any(|text| text.has_text(string_bounder))
        {
            return None;
        }
        if Self::same(self.get_last(), other.get_first()) {
            // PlantUML has not coded arrows with a start decoration merging; they never do.
            debug_assert!(self.start_decoration.is_none() && other.start_decoration.is_none());
            let one_of = other
                .end_decoration
                .clone()
                .or_else(|| self.end_decoration.clone());
            let merge_texts: Vec<Text> = self
                .texts
                .borrow()
                .iter()
                .chain(other.texts.borrow().iter())
                .cloned()
                .collect();
            return Some(Self {
                worm: self.worm.merge(&other.worm, strategy),
                start_decoration: None,
                end_decoration: one_of,
                color: self.color.clone(),
                texts: Rc::new(RefCell::new(merge_texts)),
                mergeable: strategy,
                emphasize_direction: self.emphasize_direction.or(other.emphasize_direction),
            });
        }
        if Self::same(self.get_first(), other.get_last()) {
            return other.merge(self, string_bounder);
        }
        None
    }
}

impl std::fmt::Debug for Snake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snake")
            .field("worm", &self.worm)
            .field("mergeable", &self.mergeable)
            .finish_non_exhaustive()
    }
}
