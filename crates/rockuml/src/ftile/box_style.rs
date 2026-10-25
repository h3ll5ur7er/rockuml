//! The outline of an activity box, chosen by the character ending the activity or by a stereotype
//! (PlantUML's `BoxStyle` and its fourteen subclasses).

use crate::klimt::geom::UTranslate;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::stereo::Stereotype;

const DELTA_INPUT_OUTPUT: f64 = 10.0;
const DELTA_CONTINUOUS: f64 = 5.0;
const PADDING: f64 = 5.0;

/// Declared in PlantUML's order, which [`BoxStyle::from_string`] searches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoxStyle {
    /// `(=)`
    Plain,
    /// `|=<`
    SdlInput,
    /// `|=>`
    SdlOutput,
    /// `[|=|]`
    SdlProcedure,
    /// `\=\`
    SdlLoad,
    /// `/=/`
    SdlSave,
    /// `< >`
    SdlContinuous,
    /// `[=]`
    SdlTask,
    /// `[=]`
    UmlObject,
    /// `>=>`
    UmlObjectSignal,
    /// `|=<`
    UmlTrigger,
    /// `|=>`
    UmlSendSignal,
    /// `>=|`
    UmlAcceptEvent,
    /// `X`
    UmlTimeEvent,
}

impl BoxStyle {
    const VALUES: [Self; 14] = [
        Self::Plain,
        Self::SdlInput,
        Self::SdlOutput,
        Self::SdlProcedure,
        Self::SdlLoad,
        Self::SdlSave,
        Self::SdlContinuous,
        Self::SdlTask,
        Self::UmlObject,
        Self::UmlObjectSignal,
        Self::UmlTrigger,
        Self::UmlSendSignal,
        Self::UmlAcceptEvent,
        Self::UmlTimeEvent,
    ];

    /// The stereotype naming the style, which the box shows; none for plain boxes (`name`).
    pub(crate) fn name(self) -> Option<&'static str> {
        Some(match self {
            Self::Plain => return None,
            Self::SdlInput => "input",
            Self::SdlOutput => "output",
            Self::SdlProcedure => "procedure",
            Self::SdlLoad => "load",
            Self::SdlSave => "save",
            Self::SdlContinuous => "continuous",
            Self::SdlTask => "task",
            Self::UmlObject => "object",
            Self::UmlObjectSignal => "objectSignal",
            Self::UmlTrigger => "trigger",
            Self::UmlSendSignal => "sendSignal",
            Self::UmlAcceptEvent => "acceptEvent",
            Self::UmlTimeEvent => "timeEvent",
        })
    }

    /// How much of the box's width the outline gives up to its pointed side.
    pub(crate) fn get_shield(self) -> f64 {
        match self {
            Self::SdlInput
            | Self::SdlOutput
            | Self::UmlObjectSignal
            | Self::UmlTrigger
            | Self::UmlSendSignal
            | Self::UmlAcceptEvent
            | Self::UmlTimeEvent => 10.0,
            _ => 0.0,
        }
    }

    /// The style a stereotype names, ignoring case and dashes; plain for any other.
    pub(crate) fn from_string(style: &str) -> Self {
        let style = style.replace('-', "");
        Self::VALUES
            .into_iter()
            .find(|value| {
                value
                    .name()
                    .is_some_and(|name| style.eq_ignore_ascii_case(name))
            })
            .unwrap_or(Self::Plain)
    }

    pub(crate) fn get_stereotype(self) -> Option<Stereotype> {
        self.name()
            .map(|name| Stereotype::new(&format!("<<{name}>>")))
    }

    /// Draws the outline of a box `width` wide, less the shield (`drawMe`).
    pub(crate) fn draw_me(
        self,
        ug: &UGraphic,
        width: f64,
        height: f64,
        shadowing: f64,
        round_corner: f64,
    ) {
        let width = width - self.get_shield();
        let input = || {
            UShape::polygon(vec![
                (0.0, 0.0),
                (width + DELTA_INPUT_OUTPUT, 0.0),
                (width, height / 2.0),
                (width + DELTA_INPUT_OUTPUT, height),
                (0.0, height),
            ])
        };
        let output = || {
            UShape::polygon(vec![
                (0.0, 0.0),
                (width, 0.0),
                (width + DELTA_INPUT_OUTPUT, height / 2.0),
                (width, height),
                (0.0, height),
            ])
        };
        let shape = match self {
            Self::Plain => UShape::Rectangle(URectangle::new(width, height).rounded(round_corner)),
            Self::SdlInput | Self::UmlTrigger => input(),
            Self::SdlOutput | Self::UmlSendSignal => output(),
            Self::SdlProcedure => {
                ug.draw(&UShape::Rectangle(
                    URectangle::new(width, height).with_delta_shadow(shadowing),
                ));
                let vline = UShape::Line {
                    dx: 0.0,
                    dy: height,
                };
                ug.apply(UTranslate::new(PADDING, 0.0)).draw(&vline);
                ug.apply(UTranslate::new(width - PADDING, 0.0)).draw(&vline);
                return;
            }
            Self::SdlLoad => UShape::polygon(vec![
                (0.0, 0.0),
                (width - DELTA_INPUT_OUTPUT, 0.0),
                (width, height),
                (DELTA_INPUT_OUTPUT, height),
            ]),
            Self::SdlSave => UShape::polygon(vec![
                (DELTA_INPUT_OUTPUT, 0.0),
                (width, 0.0),
                (width - DELTA_INPUT_OUTPUT, height),
                (0.0, height),
            ]),
            Self::SdlContinuous => UShape::path(vec![
                USegment::MoveTo(DELTA_CONTINUOUS, 0.0),
                USegment::LineTo(0.0, height / 2.0),
                USegment::LineTo(DELTA_CONTINUOUS, height),
                USegment::MoveTo(width - DELTA_CONTINUOUS, 0.0),
                USegment::LineTo(width, height / 2.0),
                USegment::LineTo(width - DELTA_CONTINUOUS, height),
            ]),
            Self::SdlTask | Self::UmlObject => UShape::Rectangle(URectangle::new(width, height)),
            Self::UmlObjectSignal => UShape::polygon(vec![
                (-DELTA_INPUT_OUTPUT, 0.0),
                (width, 0.0),
                (width + DELTA_INPUT_OUTPUT, height / 2.0),
                (width, height),
                (-DELTA_INPUT_OUTPUT, height),
                (0.0, height / 2.0),
            ]),
            Self::UmlAcceptEvent => UShape::polygon(vec![
                (-DELTA_INPUT_OUTPUT, 0.0),
                (width, 0.0),
                (width, height),
                (-DELTA_INPUT_OUTPUT, height),
                (0.0, height / 2.0),
            ]),
            Self::UmlTimeEvent => {
                let half_width = width / 2.0;
                let third_height = height / 3.0;
                UShape::polygon(vec![
                    (half_width - third_height, third_height),
                    (half_width + third_height, third_height),
                    (half_width - third_height, height),
                    (half_width + third_height, height),
                ])
            }
        };
        ug.draw(&shape.with_delta_shadow(shadowing));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::color::HColor;
    use crate::klimt::debug::StringBounderDebug;
    use crate::klimt::ugraphic::{UGraphicBackend, UParam};

    /// The shadow of each shape drawn, by its PlantUML class.
    #[derive(Default)]
    struct Shadows(Vec<(&'static str, f64)>);

    impl UGraphicBackend for Shadows {
        fn draw(&mut self, shape: &UShape, _at: UTranslate, _param: &UParam) {
            self.0.push((shape.java_class_name(), shape.delta_shadow()));
        }
    }

    fn shadows_drawn(style: BoxStyle, round_corner: f64) -> Vec<(&'static str, f64)> {
        let shadows = Rc::new(RefCell::new(Shadows::default()));
        let ug = UGraphic::new(shadows.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
        style.draw_me(&ug, 100.0, 40.0, 3.0, round_corner);
        shadows.take().0
    }

    #[test]
    fn outlines_cast_the_shadow_even_when_rounded() {
        for style in BoxStyle::VALUES {
            assert_eq!(shadows_drawn(style, 25.0)[0].1, 3.0, "{style:?}");
        }
    }

    #[test]
    fn the_lines_of_a_procedure_cast_none() {
        assert_eq!(
            shadows_drawn(BoxStyle::SdlProcedure, 0.0),
            [("URectangle", 3.0), ("ULine", 0.0), ("ULine", 0.0)]
        );
    }

    #[test]
    fn styles_are_found_by_stereotype_ignoring_case_and_dashes() {
        assert_eq!(
            BoxStyle::from_string("Send-Signal"),
            BoxStyle::UmlSendSignal
        );
        assert_eq!(BoxStyle::from_string("input"), BoxStyle::SdlInput);
        assert_eq!(BoxStyle::from_string("whatever"), BoxStyle::Plain);
        assert_eq!(BoxStyle::Plain.get_stereotype(), None);
        assert_eq!(BoxStyle::SdlTask.get_shield(), 0.0);
        assert_eq!(BoxStyle::UmlTimeEvent.get_shield(), 10.0);
    }
}
