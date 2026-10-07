//! The shape of an activity's box, chosen by a stereotype like `<<input>>` (PlantUML's `BoxStyle`). A
//! stand-in for the rendering track's port, which adds the shapes.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoxStyle {
    Plain,
    SdlInput,
    SdlOutput,
    SdlProcedure,
    SdlLoad,
    SdlSave,
    SdlContinuous,
    SdlTask,
    UmlObject,
    UmlObjectSignal,
    UmlTrigger,
    UmlSendSignal,
    UmlAcceptEvent,
    UmlTimeEvent,
}

impl BoxStyle {
    /// PlantUML's order of the styles.
    const VALUES: [BoxStyle; 14] = [
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

    /// The stereotype label that selects the style, none for the plain box.
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

    /// The style a label names, ignoring case and dashes, or the plain box.
    pub(crate) fn from_string(style: &str) -> Self {
        let style = style.replace('-', "");
        Self::VALUES
            .into_iter()
            .find(|candidate| {
                candidate
                    .name()
                    .is_some_and(|name| style.eq_ignore_ascii_case(name))
            })
            .unwrap_or(Self::Plain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_styles_ignoring_case_and_dashes() {
        assert_eq!(
            BoxStyle::from_string("Send-Signal"),
            BoxStyle::UmlSendSignal
        );
        assert_eq!(BoxStyle::from_string("INPUT"), BoxStyle::SdlInput);
        assert_eq!(BoxStyle::from_string("custom"), BoxStyle::Plain);
    }
}
