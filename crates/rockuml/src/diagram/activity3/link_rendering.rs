//! How an arrow between two instructions is drawn: its colours and its label (PlantUML's
//! `activitydiagram3.LinkRendering`).

use crate::creole::Display;
use crate::decoration::Rainbow;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LinkRendering {
    rainbow: Rainbow,
    /// `None` is PlantUML's `Display.NULL`: no label at all, which is not the same as an empty one.
    display: Option<Display>,
}

impl LinkRendering {
    pub(crate) fn create(rainbow: Rainbow) -> Self {
        Self {
            rainbow,
            display: None,
        }
    }

    /// No colour and no label.
    pub(crate) fn none() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn with_rainbow(&self, rainbow: Rainbow) -> Self {
        Self {
            rainbow,
            display: self.display.clone(),
        }
    }

    #[must_use]
    pub(crate) fn with_display(&self, display: Option<Display>) -> Self {
        Self {
            rainbow: self.rainbow.clone(),
            display,
        }
    }

    pub(crate) fn get_display(&self) -> Option<&Display> {
        self.display.as_ref()
    }

    pub(crate) fn get_rainbow(&self) -> &Rainbow {
        &self.rainbow
    }

    /// The colours, or `default_color` when there are none (`getRainbow(Rainbow)`).
    pub(crate) fn get_rainbow_or(&self, default_color: &Rainbow) -> Rainbow {
        self.rainbow.with_default(default_color)
    }

    pub(crate) fn is_none(&self) -> bool {
        self.display.is_none() && self.rainbow.size() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::HColor;

    #[test]
    fn a_rendering_without_colour_or_label_is_none_and_an_empty_label_counts() {
        assert!(LinkRendering::none().is_none());
        let labelled = LinkRendering::none().with_display(Some(Display::create([""])));
        assert!(!labelled.is_none());
        let red = Rainbow::from_color(Some(HColor::RED), None);
        let coloured = LinkRendering::create(red.clone());
        assert!(!coloured.is_none());
        assert_eq!(LinkRendering::none().get_rainbow_or(&red), red);
        assert_eq!(
            labelled.with_rainbow(red.clone()).get_display(),
            labelled.get_display()
        );
    }
}
