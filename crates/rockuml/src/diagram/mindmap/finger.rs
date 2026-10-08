//! An idea drawn with the ideas that grow from it: its own box (the phalanx) and, past a gap, its children
//! packed against each other (the nail) (PlantUML's `FingerImpl`).

use std::cell::OnceCell;

use super::idea::{Branch, IdeaId, IdeaShape};
use super::tetris::{SymetricalTee, Tetris};
use crate::color::{ColorType, Colors};
use crate::creole::CreoleMode;
use crate::ftile::vertical::FtileBoxOld;
use crate::klimt::TextBlock;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XPoint2D};
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::component::TextBlockEmpty;
use crate::skin::{Rankdir, SkinParam};
use crate::style::{PName, Style, ValueReading};

pub(super) struct FingerImpl<'a> {
    branch: &'a Branch,
    idea: IdeaId,
    skin_param: &'a SkinParam,
    /// 1 to the right (or down), -1 to the left (or up).
    direction: f64,
    draw_phalanx: bool,
    nail: Vec<FingerImpl<'a>>,
    phalanx: OnceCell<Box<dyn TextBlock + 'a>>,
    tetris: OnceCell<Tetris>,
}

impl<'a> FingerImpl<'a> {
    pub(super) fn build(
        branch: &'a Branch,
        idea: IdeaId,
        skin_param: &'a SkinParam,
        direction: bool,
    ) -> Self {
        Self {
            branch,
            idea,
            skin_param,
            direction: if direction { 1.0 } else { -1.0 },
            draw_phalanx: true,
            nail: branch
                .idea(idea)
                .children
                .iter()
                .map(|&child| Self::build(branch, child, skin_param, direction))
                .collect(),
            phalanx: OnceCell::new(),
            tetris: OnceCell::new(),
        }
    }

    /// The root drawn by the other side already.
    pub(super) fn do_not_draw_first_phalanx(&mut self) {
        self.draw_phalanx = false;
    }

    fn is_top_to_bottom(&self) -> bool {
        self.skin_param.get_rankdir() == Rankdir::TopToBottom
    }

    fn get_style(&self) -> Style {
        self.branch.style(self.idea)
    }

    fn get_margin(&self) -> ClockwiseTopRightBottomLeft {
        self.get_style().margin()
    }

    pub(super) fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let phalanx = self.get_phalanx();
        let dim_phalanx = phalanx.calculate_dimension(string_bounder);
        if self.draw_phalanx {
            let (pos_x, pos_y) = if self.is_top_to_bottom() {
                (
                    -self.get_phalanx_thickness(string_bounder) / 2.0,
                    if self.direction == 1.0 {
                        0.0
                    } else {
                        -dim_phalanx.height
                    },
                )
            } else {
                (
                    if self.direction == 1.0 {
                        0.0
                    } else {
                        -dim_phalanx.width
                    },
                    -self.get_phalanx_thickness(string_bounder) / 2.0,
                )
            };
            phalanx.draw_u(&ug.translated(pos_x, pos_y));
        }
        let p1 = if self.is_top_to_bottom() {
            XPoint2D::new(0.0, self.direction * dim_phalanx.height)
        } else {
            XPoint2D::new(self.direction * dim_phalanx.width, 0.0)
        };
        let tetris = self.get_tetris(string_bounder);
        for (child, stp) in self.nail.iter().zip(tetris.get_elements()) {
            let p2 = if self.is_top_to_bottom() {
                XPoint2D::new(
                    stp.get_y(),
                    self.direction * (dim_phalanx.height + self.get_x12()),
                )
            } else {
                XPoint2D::new(
                    self.direction * (dim_phalanx.width + self.get_x12()),
                    stp.get_y(),
                )
            };
            child.draw_u(&ug.translated(p2.x, p2.y));
            let style_arrow = self.branch.style_arrow(self.idea);
            let link_color = style_arrow.value(PName::LineColor).as_color();
            if !link_color.is_transparent() {
                self.draw_line(&ug.apply(link_color).apply(style_arrow.stroke()), p1, p2);
            }
        }
    }

    fn draw_line(&self, ug: &UGraphic, p1: XPoint2D, p2: XPoint2D) {
        let mut segments = vec![USegment::MoveTo(p1.x, p1.y)];
        if self.is_top_to_bottom() {
            let delta1 = self.direction * 3.0;
            let delta2 = self.direction * 10.0;
            segments.push(USegment::LineTo(p1.x, p1.y + delta1));
            segments.push(USegment::CubicTo {
                ctrl1: (p1.x, p1.y + delta2),
                ctrl2: (p2.x, p2.y - delta2),
                end: (p2.x, p2.y - delta1),
            });
        } else {
            let delta1 = self.direction * 10.0;
            let delta2 = self.direction * 25.0;
            segments.push(USegment::LineTo(p1.x + delta1, p1.y));
            segments.push(USegment::CubicTo {
                ctrl1: (p1.x + delta2, p1.y),
                ctrl2: (p2.x - delta2, p2.y),
                end: (p2.x - delta1, p2.y),
            });
        }
        segments.push(USegment::LineTo(p2.x, p2.y));
        ug.draw(&UShape::path(segments));
    }

    fn get_tetris(&self, string_bounder: &dyn StringBounder) -> &Tetris {
        self.tetris.get_or_init(|| {
            let mut tetris = Tetris::new();
            for child in &self.nail {
                tetris.add(child.as_symetrical_tee(string_bounder));
            }
            tetris.balance();
            tetris
        })
    }

    fn as_symetrical_tee(&self, string_bounder: &dyn StringBounder) -> SymetricalTee {
        let thickness1 = self.get_phalanx_thickness(string_bounder);
        let elongation1 = self.get_phalanx_elongation(string_bounder);
        if self.nail.is_empty() {
            return SymetricalTee::new(thickness1, elongation1, 0.0, 0.0);
        }
        let thickness2 = self.get_nail_thickness(string_bounder);
        let elongation2 = self.get_nail_elongation(string_bounder);
        SymetricalTee::new(
            thickness1,
            elongation1 + self.get_x1(),
            thickness2,
            self.get_x2() + elongation2,
        )
    }

    fn get_x1(&self) -> f64 {
        if self.is_top_to_bottom() {
            self.get_margin().top
        } else {
            self.get_margin().left
        }
    }

    fn get_x2(&self) -> f64 {
        if self.is_top_to_bottom() {
            self.get_margin().bottom + 5.0
        } else {
            self.get_margin().right + 30.0
        }
    }

    pub(super) fn get_x12(&self) -> f64 {
        self.get_x1() + self.get_x2()
    }

    fn get_phalanx_thickness(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dimension = self.get_phalanx().calculate_dimension(string_bounder);
        if self.is_top_to_bottom() {
            dimension.width
        } else {
            dimension.height
        }
    }

    fn get_phalanx_elongation(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dimension = self.get_phalanx().calculate_dimension(string_bounder);
        if self.is_top_to_bottom() {
            dimension.height
        } else {
            dimension.width
        }
    }

    fn get_phalanx(&self) -> &dyn TextBlock {
        self.phalanx.get_or_init(|| self.create_phalanx()).as_ref()
    }

    fn create_phalanx(&self) -> Box<dyn TextBlock + 'a> {
        if !self.draw_phalanx {
            return Box::new(TextBlockEmpty::default());
        }
        let style = self.get_style();
        let idea = self.branch.idea(self.idea);
        if idea.shape == IdeaShape::Box {
            let colors = Colors::default().with(ColorType::Back, idea.back_color.clone());
            let block = FtileBoxOld::create_mind_map(&style, self.skin_param, &colors, &idea.label);
            let margin = style.margin();
            return Box::new(if self.is_top_to_bottom() {
                TextBlockMarged::new(block, margins(margin.left, margin.right, 0.0, 0.0))
            } else {
                TextBlockMarged::new(block, margins(0.0, 0.0, margin.top, margin.bottom))
            });
        }
        let text = idea.label.create0(
            &style.font_configuration(),
            style
                .horizontal_alignment()
                .unwrap_or(crate::klimt::HorizontalAlignment::Left),
            self.skin_param,
            style.wrap_width(),
            CreoleMode::Full,
        );
        Box::new(if self.direction == 1.0 {
            TextBlockMarged::new(text, margins(3.0, 0.0, 1.0, 1.0))
        } else {
            TextBlockMarged::new(text, margins(0.0, 3.0, 1.0, 1.0))
        })
    }

    fn get_nail_thickness(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_tetris(string_bounder).get_height()
    }

    fn get_nail_elongation(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_tetris(string_bounder).get_width()
    }

    pub(super) fn get_full_thickness(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_phalanx_thickness(string_bounder)
            .max(self.get_nail_thickness(string_bounder))
    }

    pub(super) fn get_full_elongation(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_phalanx_elongation(string_bounder) + self.get_nail_elongation(string_bounder)
    }
}

/// `TextBlockUtils.withMargin(block, x1, x2, y1, y2)`'s margins.
fn margins(x1: f64, x2: f64, y1: f64, y2: f64) -> ClockwiseTopRightBottomLeft {
    ClockwiseTopRightBottomLeft::top_right_bottom_left(y1, x2, y2, x1)
}
