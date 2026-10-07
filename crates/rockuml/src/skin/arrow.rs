//! How a sequence message's arrow looks: body, heads and decorations at each end (PlantUML's
//! `ArrowConfiguration` and its parts).

use crate::color::HColor;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrowBody {
    Normal,
    Dotted,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrowHead {
    Normal,
    CrossX,
    Async,
    None,
}

/// Half-headed arrows draw only the top or bottom half of their head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrowPart {
    Full,
    TopPart,
    BottomPart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrowDecoration {
    None,
    Circle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrowDirection {
    LeftToRightNormal,
    RightToLeftReverse,
    SelfArrow,
    BothDirection,
}

/// One end of an arrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ArrowDressing {
    pub head: ArrowHead,
    pub part: ArrowPart,
}

impl ArrowDressing {
    const NONE: Self = Self {
        head: ArrowHead::None,
        part: ArrowPart::Full,
    };

    fn with_head(self, head: ArrowHead) -> Self {
        Self { head, ..self }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArrowConfiguration {
    body: ArrowBody,
    dressing1: ArrowDressing,
    dressing2: ArrowDressing,
    decoration1: ArrowDecoration,
    decoration2: ArrowDecoration,
    color: Option<HColor>,
    is_self: bool,
    thickness: f64,
    reverse_define: bool,
    inclination: i32,
}

impl ArrowConfiguration {
    fn with_dressings(dressing1: ArrowDressing, dressing2: ArrowDressing) -> Self {
        Self {
            body: ArrowBody::Normal,
            dressing1,
            dressing2,
            decoration1: ArrowDecoration::None,
            decoration2: ArrowDecoration::None,
            color: None,
            is_self: false,
            thickness: 1.0,
            reverse_define: false,
            inclination: 0,
        }
    }

    pub(crate) fn with_direction_normal() -> Self {
        Self::with_dressings(
            ArrowDressing::NONE,
            ArrowDressing::NONE.with_head(ArrowHead::Normal),
        )
    }

    pub(crate) fn with_direction_both() -> Self {
        Self::with_dressings(
            ArrowDressing::NONE.with_head(ArrowHead::Normal),
            ArrowDressing::NONE.with_head(ArrowHead::Normal),
        )
    }

    /// The arrow seen from its other end.
    #[must_use]
    pub(crate) fn reverse(&self) -> Self {
        Self {
            dressing1: self.dressing2,
            dressing2: self.dressing1,
            decoration1: self.decoration2,
            decoration2: self.decoration1,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn self_arrow(&self) -> Self {
        Self {
            is_self: true,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_body(&self, body: ArrowBody) -> Self {
        Self {
            body,
            ..self.clone()
        }
    }

    /// Both heads that exist become `head`.
    #[must_use]
    pub(crate) fn with_head(&self, head: ArrowHead) -> Self {
        let add = |dressing: ArrowDressing| {
            if dressing.head == ArrowHead::None {
                dressing
            } else {
                dressing.with_head(head)
            }
        };
        Self {
            dressing1: add(self.dressing1),
            dressing2: add(self.dressing2),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_head1(&self, head: ArrowHead) -> Self {
        Self {
            dressing1: self.dressing1.with_head(head),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_head2(&self, head: ArrowHead) -> Self {
        Self {
            dressing2: self.dressing2.with_head(head),
            ..self.clone()
        }
    }

    /// The part applies to the end with a head, the second end first.
    #[must_use]
    pub(crate) fn with_part(&self, part: ArrowPart) -> Self {
        let mut result = self.clone();
        if self.dressing2.head != ArrowHead::None {
            result.dressing2.part = part;
        } else {
            result.dressing1.part = part;
        }
        result
    }

    #[must_use]
    pub(crate) fn with_decoration1(&self, decoration1: ArrowDecoration) -> Self {
        Self {
            decoration1,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_decoration2(&self, decoration2: ArrowDecoration) -> Self {
        Self {
            decoration2,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_color(&self, color: HColor) -> Self {
        Self {
            color: Some(color),
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_thickness(&self, thickness: f64) -> Self {
        Self {
            thickness,
            ..self.clone()
        }
    }

    /// Written right to left, like `Bob <- Alice`.
    #[must_use]
    pub(crate) fn reverse_define(&self) -> Self {
        Self {
            reverse_define: !self.reverse_define,
            ..self.clone()
        }
    }

    #[must_use]
    pub(crate) fn with_inclination(&self, inclination: i32) -> Self {
        Self {
            inclination,
            ..self.clone()
        }
    }

    pub(crate) fn decoration1(&self) -> ArrowDecoration {
        self.decoration1
    }

    pub(crate) fn decoration2(&self) -> ArrowDecoration {
        self.decoration2
    }

    pub(crate) fn dressing1(&self) -> ArrowDressing {
        self.dressing1
    }

    pub(crate) fn dressing2(&self) -> ArrowDressing {
        self.dressing2
    }

    pub(crate) fn color(&self) -> Option<&HColor> {
        self.color.as_ref()
    }

    pub(crate) fn arrow_direction(&self) -> ArrowDirection {
        match (self.dressing1.head, self.dressing2.head) {
            _ if self.is_self => ArrowDirection::SelfArrow,
            (ArrowHead::None, head2) if head2 != ArrowHead::None => {
                ArrowDirection::LeftToRightNormal
            }
            (head1, ArrowHead::None) if head1 != ArrowHead::None => {
                ArrowDirection::RightToLeftReverse
            }
            _ => ArrowDirection::BothDirection,
        }
    }

    pub(crate) fn is_dotted(&self) -> bool {
        self.body == ArrowBody::Dotted
    }

    pub(crate) fn is_hidden(&self) -> bool {
        self.body == ArrowBody::Hidden
    }

    /// The head of the end that has one, the second end first.
    pub(crate) fn head(&self) -> ArrowHead {
        if self.dressing2.head != ArrowHead::None {
            self.dressing2.head
        } else {
            self.dressing1.head
        }
    }

    pub(crate) fn is_async1(&self) -> bool {
        self.dressing1.head == ArrowHead::Async
    }

    pub(crate) fn is_async2(&self) -> bool {
        self.dressing2.head == ArrowHead::Async
    }

    pub(crate) fn part(&self) -> ArrowPart {
        if self.dressing2.head != ArrowHead::None {
            self.dressing2.part
        } else {
            self.dressing1.part
        }
    }

    pub(crate) fn is_reverse_define(&self) -> bool {
        self.reverse_define
    }

    /// The slope of slanted arrows starts at the end without a head.
    pub(crate) fn inclination1(&self) -> i32 {
        if matches!(self.dressing2.head, ArrowHead::None | ArrowHead::CrossX) {
            self.inclination
        } else {
            0
        }
    }

    pub(crate) fn inclination2(&self) -> i32 {
        if matches!(
            self.dressing1.head,
            ArrowHead::None | ArrowHead::CrossX | ArrowHead::Normal
        ) {
            self.inclination
        } else {
            0
        }
    }

    /// The style's stroke; dotted arrows without a dash pattern of their own dash 2-2.
    pub(crate) fn apply_stroke(&self, ug: &UGraphic, style: &Style) -> UGraphic {
        let stroke = style.stroke();
        if self.is_dotted() && stroke.dash_visible == 0.0 && stroke.dash_space == 0.0 {
            return ug.with_stroke(UStroke {
                dash_visible: 2.0,
                dash_space: 2.0,
                thickness: stroke.thickness,
            });
        }
        ug.with_stroke(stroke)
    }

    /// The configuration's own thickness, dashed 2-2 when dotted (PlantUML's deprecated `applyStroke`).
    pub(crate) fn apply_own_stroke(&self, ug: &UGraphic) -> UGraphic {
        if self.is_dotted() {
            return ug.with_stroke(UStroke {
                dash_visible: 2.0,
                dash_space: 2.0,
                thickness: self.thickness,
            });
        }
        ug.with_stroke(UStroke::with_thickness(self.thickness))
    }

    pub(crate) fn apply_thickness_only(&self, ug: &UGraphic) -> UGraphic {
        ug.with_stroke(UStroke::with_thickness(self.thickness))
    }
}
