//! How an arrow between two instructions is drawn: its colours and its label (PlantUML's `LinkRendering`).

use crate::creole::Display;
use crate::decoration::Rainbow;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LinkRendering {
    pub(crate) rainbow: Rainbow,
    pub(crate) display: Option<Display>,
}

impl LinkRendering {
    pub(crate) fn create(rainbow: Rainbow) -> Self {
        Self {
            rainbow,
            display: None,
        }
    }

    pub(crate) const fn none() -> Self {
        Self {
            rainbow: Rainbow::none(),
            display: None,
        }
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

    /// Neither a label nor a colour.
    pub(crate) fn is_none(&self) -> bool {
        self.display.is_none() && self.rainbow.size() == 0
    }
}
