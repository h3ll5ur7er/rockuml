//! Swimlanes (PlantUML's `activitydiagram3.ftile.Swimlane`).
//!
//! PlantUML compares swimlanes by identity and orders them by declaration. Here a swimlane is named by its
//! [`SwimlaneId`], its index in declaration order, which tiles, connections and layers pass around; the
//! [`Swimlane`] it names, with its title and colours, belongs to the diagram's list of swimlanes. What
//! PlantUML sets on a swimlane while laying the lanes out (translation, width, extent) belongs to whoever
//! lays them out (`Swimlanes`), keyed by the same ids.

use std::collections::BTreeSet;

use crate::color::{ColorType, Colors, HColor};
use crate::creole::Display;

/// A swimlane, by its index in declaration order (PlantUML's `order`). The lane PlantUML adds after the
/// last one while drawing, ordered after all, is `SwimlaneId(usize::MAX)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SwimlaneId(pub(crate) usize);

impl SwimlaneId {
    /// Whether this lane comes before all of `others`, which must not be this lane alone
    /// (`isSmallerThanAllOthers`).
    pub(crate) fn is_smaller_than_all_others(self, others: &BTreeSet<SwimlaneId>) -> bool {
        if others.len() == 1 && others.contains(&self) {
            return false;
        }
        others.iter().all(|other| *other >= self)
    }
}

/// A swimlane's name, title and colours.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Swimlane {
    name: String,
    display: Display,
    colors: Colors,
}

impl Swimlane {
    /// Titled by its name, which `\n` breaks into lines.
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            display: Display::with_newlines(name),
            colors: Colors::default(),
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        &self.name
    }

    pub(crate) fn get_display(&self) -> &Display {
        &self.display
    }

    pub(crate) fn set_display(&mut self, label: Display) {
        self.display = label;
    }

    pub(crate) fn get_colors(&self) -> &Colors {
        &self.colors
    }

    pub(crate) fn set_colors(&mut self, colors: Colors) {
        self.colors = colors;
    }

    /// `setSpecificColorTOBEREMOVED`
    pub(crate) fn set_specific_color(&mut self, kind: ColorType, color: HColor) {
        self.colors = self.colors.with(kind, Some(color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lane_is_smaller_than_later_lanes_but_not_than_itself_alone() {
        let lanes = |ids: &[usize]| ids.iter().copied().map(SwimlaneId).collect();
        assert!(SwimlaneId(1).is_smaller_than_all_others(&lanes(&[1, 2])));
        assert!(SwimlaneId(1).is_smaller_than_all_others(&lanes(&[2, 3])));
        assert!(!SwimlaneId(1).is_smaller_than_all_others(&lanes(&[1])));
        assert!(!SwimlaneId(2).is_smaller_than_all_others(&lanes(&[1, 2])));
        assert!(SwimlaneId(2).is_smaller_than_all_others(&BTreeSet::new()));
    }
}
