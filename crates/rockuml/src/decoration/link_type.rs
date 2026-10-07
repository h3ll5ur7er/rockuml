//! What a link looks like: the decorations at both ends, one in the middle, and the line between them
//! (PlantUML's `LinkType` and `LinkMiddleDecor`, without their drawing).

use super::{LinkDecor, LinkStyle};
use crate::klimt::ugraphic::UStroke;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkMiddleDecor {
    None,
    Circle,
    CircleCircled,
    CircleCircled1,
    CircleCircled2,
    Subset,
    Superset,
}

impl LinkMiddleDecor {
    /// Seen from the other end.
    #[must_use]
    pub(crate) fn get_inversed(self) -> Self {
        match self {
            Self::CircleCircled1 => Self::CircleCircled2,
            Self::CircleCircled2 => Self::CircleCircled1,
            other => other,
        }
    }
}

/// `decor1` is drawn at the start of the link, at its first entity; `decor2` at its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LinkType {
    decor1: LinkDecor,
    link_style: LinkStyle,
    decor2: LinkDecor,
    middle_decor: LinkMiddleDecor,
}

impl LinkType {
    pub(crate) fn new(decor1: LinkDecor, decor2: LinkDecor) -> Self {
        Self {
            decor1,
            decor2,
            middle_decor: LinkMiddleDecor::None,
            link_style: LinkStyle::NORMAL,
        }
    }

    pub(crate) fn is_double_decorated(self) -> bool {
        self.decor1 != LinkDecor::None && self.decor2 != LinkDecor::None
    }

    pub(crate) fn looks_like_reverted_for_svg(self) -> bool {
        self.decor1 == LinkDecor::None && self.decor2 != LinkDecor::None
    }

    pub(crate) fn looks_like_no_decor_at_all_svg(self) -> bool {
        (self.decor1 == LinkDecor::None) == (self.decor2 == LinkDecor::None)
    }

    #[must_use]
    pub(crate) fn without_decors1(self) -> Self {
        Self {
            decor1: LinkDecor::None,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn without_decors2(self) -> Self {
        Self {
            decor2: LinkDecor::None,
            ..self
        }
    }

    pub(crate) fn is_invisible(self) -> bool {
        self.link_style.is_invisible()
    }

    fn with_style(self, link_style: LinkStyle) -> Self {
        Self { link_style, ..self }
    }

    fn with_middle(self, middle_decor: LinkMiddleDecor) -> Self {
        Self {
            middle_decor,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn go_dashed(self) -> Self {
        self.with_style(LinkStyle::DASHED)
    }

    #[must_use]
    pub(crate) fn go_dotted(self) -> Self {
        self.with_style(LinkStyle::DOTTED)
    }

    #[must_use]
    pub(crate) fn go_thickness(self, thickness: f64) -> Self {
        self.with_style(self.link_style.go_thickness(thickness))
    }

    #[must_use]
    pub(crate) fn go_bold(self) -> Self {
        self.with_style(LinkStyle::BOLD)
    }

    /// The same link drawn from its other end.
    #[must_use]
    pub(crate) fn get_inversed(self) -> Self {
        Self {
            decor1: self.decor2,
            decor2: self.decor1,
            middle_decor: self.middle_decor.get_inversed(),
            link_style: self.link_style,
        }
    }

    #[must_use]
    pub(crate) fn with_middle_circle(self) -> Self {
        self.with_middle(LinkMiddleDecor::Circle)
    }

    #[must_use]
    pub(crate) fn with_middle_circle_circled(self) -> Self {
        self.with_middle(LinkMiddleDecor::CircleCircled)
    }

    #[must_use]
    pub(crate) fn with_middle_circle_circled1(self) -> Self {
        self.with_middle(LinkMiddleDecor::CircleCircled1)
    }

    #[must_use]
    pub(crate) fn with_middle_circle_circled2(self) -> Self {
        self.with_middle(LinkMiddleDecor::CircleCircled2)
    }

    #[must_use]
    pub(crate) fn with_middle_subset(self) -> Self {
        self.with_middle(LinkMiddleDecor::Subset)
    }

    #[must_use]
    pub(crate) fn with_middle_superset(self) -> Self {
        self.with_middle(LinkMiddleDecor::Superset)
    }

    #[must_use]
    pub(crate) fn get_invisible(self) -> Self {
        self.with_style(LinkStyle::INVISIBLE)
    }

    pub(crate) fn get_decor1(self) -> LinkDecor {
        self.decor1
    }

    pub(crate) fn get_style(self) -> LinkStyle {
        self.link_style
    }

    pub(crate) fn get_decor2(self) -> LinkDecor {
        self.decor2
    }

    pub(crate) fn get_middle_decor(self) -> LinkMiddleDecor {
        self.middle_decor
    }

    pub(crate) fn is_extends(self) -> bool {
        self.decor1 == LinkDecor::Extends || self.decor2 == LinkDecor::Extends
    }

    /// The start half, for the first part of a link cut in two.
    #[must_use]
    pub(crate) fn get_part1(self) -> Self {
        self.without_decors2()
    }

    /// The end half, for the second part of a link cut in two.
    #[must_use]
    pub(crate) fn get_part2(self) -> Self {
        self.without_decors1()
    }

    /// The stroke of the line: the style's own thickness wins, then a plain `default_thickness` lends its
    /// thickness to the style, and a dashed one replaces it.
    /// What kind of relation the link draws, as SVG tells it in `data-link-type`.
    pub(crate) fn get_link_type_name(self) -> Option<&'static str> {
        let has = |decor: LinkDecor| self.decor1 == decor || self.decor2 == decor;
        let has_any = |decors: &[LinkDecor]| decors.iter().any(|decor| has(*decor));
        if has(LinkDecor::Composition) {
            Some("composition")
        } else if has(LinkDecor::Aggregation) {
            Some("aggregation")
        } else if has(LinkDecor::Extends) {
            Some("extension")
        } else if has(LinkDecor::Redefines) {
            Some("redefines")
        } else if has(LinkDecor::DefinedBy) {
            Some("definedby")
        } else if has_any(&[LinkDecor::Arrow, LinkDecor::ArrowTriangle]) {
            Some("dependency")
        } else if has(LinkDecor::NotNavigable) {
            Some("not_navigable")
        } else if has_any(&[
            LinkDecor::Crowfoot,
            LinkDecor::CircleCrowfoot,
            LinkDecor::LineCrowfoot,
        ]) {
            Some("crowfoot")
        } else if has_any(&[LinkDecor::CircleLine, LinkDecor::DoubleLine])
            || (self.decor1 == LinkDecor::None && self.decor2 == LinkDecor::None)
        {
            Some("association")
        } else if has(LinkDecor::Plus) {
            Some("nested")
        } else {
            None
        }
    }

    pub(crate) fn get_stroke3(self, default_thickness: Option<UStroke>) -> UStroke {
        if self.link_style.is_thickness_overrided() {
            return self.link_style.get_stroke3();
        }
        match default_thickness {
            None => self.link_style.get_stroke3(),
            Some(stroke) if stroke.dash_visible == 0.0 && stroke.dash_space == 0.0 => {
                self.link_style.go_thickness(stroke.thickness).get_stroke3()
            }
            Some(stroke) => stroke,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inversion_swaps_the_ends_and_the_middle_circles() {
        let link_type = LinkType::new(LinkDecor::Composition, LinkDecor::Arrow)
            .with_middle_circle_circled1()
            .go_dashed();
        let inversed = link_type.get_inversed();
        assert_eq!(inversed.get_decor1(), LinkDecor::Arrow);
        assert_eq!(inversed.get_decor2(), LinkDecor::Composition);
        assert_eq!(inversed.get_middle_decor(), LinkMiddleDecor::CircleCircled2);
        assert_eq!(inversed.get_style(), LinkStyle::DASHED);
        assert_eq!(inversed.get_inversed(), link_type);
    }

    #[test]
    fn halves_keep_one_decoration_each() {
        let link_type = LinkType::new(LinkDecor::Extends, LinkDecor::Arrow);
        assert_eq!(link_type.get_part1().get_decor2(), LinkDecor::None);
        assert_eq!(link_type.get_part1().get_decor1(), LinkDecor::Extends);
        assert_eq!(link_type.get_part2().get_decor1(), LinkDecor::None);
        assert!(link_type.is_extends());
        assert!(link_type.is_double_decorated());
    }

    #[test]
    fn svg_comments_follow_the_decorations() {
        let none = LinkDecor::None;
        assert!(LinkType::new(none, LinkDecor::Arrow).looks_like_reverted_for_svg());
        assert!(LinkType::new(none, none).looks_like_no_decor_at_all_svg());
        assert!(LinkType::new(LinkDecor::Arrow, LinkDecor::Arrow).looks_like_no_decor_at_all_svg());
        assert!(!LinkType::new(LinkDecor::Arrow, none).looks_like_no_decor_at_all_svg());
    }

    #[test]
    fn hidden_links_are_invisible() {
        let link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
        assert!(!link_type.is_invisible());
        assert!(link_type.get_invisible().is_invisible());
    }

    #[test]
    fn the_stroke_comes_from_the_style_then_the_default() {
        let plain = LinkType::new(LinkDecor::None, LinkDecor::Arrow);
        assert_eq!(plain.get_stroke3(None), UStroke::SIMPLE);
        assert_eq!(
            plain.get_stroke3(Some(UStroke::with_thickness(2.0))),
            UStroke::with_thickness(2.0)
        );
        assert_eq!(
            plain
                .go_dashed()
                .get_stroke3(Some(UStroke::with_thickness(2.0)))
                .to_string(),
            "7.0-7.0-2.0"
        );
        let dotted_default = UStroke {
            dash_visible: 1.0,
            dash_space: 3.0,
            thickness: 1.0,
        };
        assert_eq!(plain.get_stroke3(Some(dotted_default)), dotted_default);
        assert_eq!(
            plain.go_thickness(4.0).get_stroke3(Some(dotted_default)),
            UStroke::with_thickness(4.0)
        );
    }
}
