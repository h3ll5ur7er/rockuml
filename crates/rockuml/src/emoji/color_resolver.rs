use crate::color::HColor;

/// The colours an SVG picture is drawn in: its own, or shades of a forced colour.
pub(crate) struct ColorResolver {
    font_color: Option<HColor>,
    forced_color: Option<HColor>,
    /// The darkest gray among the picture's colours, which a forced colour replaces unchanged.
    min_gray_level: i32,
}

impl ColorResolver {
    pub(crate) fn new(
        font_color: Option<HColor>,
        forced_color: Option<HColor>,
        min_gray_level: i32,
    ) -> Self {
        Self {
            font_color,
            forced_color,
            min_gray_level,
        }
    }

    /// The colour of shapes that name none.
    pub(crate) fn default_color(&self) -> HColor {
        self.forced_color
            .as_ref()
            .or(self.font_color.as_ref())
            .cloned()
            .unwrap_or(HColor::BLACK)
    }

    /// The colour `code` names in the picture, as drawn.
    pub(crate) fn true_color(&self, code: &str) -> HColor {
        if code.eq_ignore_ascii_case("none") {
            return HColor::NONE;
        }
        let result = HColor::parse_or_white(code);
        match &self.forced_color {
            None => result,
            Some(forced) if forced.is_gray() => result.as_monochrome(),
            Some(forced) => result.as_monochrome_shade(forced, self.min_gray_level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(text: &str) -> HColor {
        HColor::parse_or_white(text)
    }

    #[test]
    fn without_a_forced_colour_the_picture_keeps_its_colours() {
        let resolver = ColorResolver::new(None, None, 999);
        assert_eq!(resolver.default_color(), HColor::BLACK);
        assert_eq!(resolver.true_color("#FFCC4D"), color("#FFCC4D"));
        assert_eq!(resolver.true_color("None"), HColor::NONE);
    }

    #[test]
    fn a_forced_gray_turns_the_picture_gray_and_a_colour_shades_it() {
        let gray = ColorResolver::new(None, Some(color("gray")), 71);
        assert_eq!(gray.true_color("#FFCC4D"), color("#CCCCCC"));
        assert_eq!(gray.default_color(), color("gray"));
        let orange = ColorResolver::new(Some(color("red")), Some(color("orange")), 71);
        assert_eq!(orange.true_color("#664500"), color("#FEA400"));
    }
}
