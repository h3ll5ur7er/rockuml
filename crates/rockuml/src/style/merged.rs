//! The style of an element, merged from the rules of a builder (PlantUML's `getMergedStyle`).

use super::{SName, Style, StyleBuilder, StyleSignature};
use crate::stereo::Stereotype;

impl StyleSignature {
    /// # Panics
    ///
    /// If no rule applies, which cannot happen with the default skin for the elements PlantUML styles.
    pub(crate) fn get_merged_style(&self, builder: &StyleBuilder) -> Style {
        builder
            .merged_style(self)
            .expect("the default skin styles every element")
    }

    /// The style of an element with a stereotype: the stereotype's rules for each of its labels, merged
    /// (`withTOBECHANGED(stereotype).getMergedStyle(builder)`).
    pub(crate) fn get_merged_style_with(
        &self,
        builder: &StyleBuilder,
        stereotype: Option<&Stereotype>,
    ) -> Style {
        self.merged_for_labels(builder, stereotype, None)
    }

    /// The style of the stereotype's own text (`forStereotypeItself(stereotype).getMergedStyle(builder)`).
    pub(crate) fn get_merged_style_for_stereotype_itself(
        &self,
        builder: &StyleBuilder,
        stereotype: Option<&Stereotype>,
    ) -> Style {
        self.merged_for_labels(builder, stereotype, Some(SName::Stereotype))
    }

    fn merged_for_labels(
        &self,
        builder: &StyleBuilder,
        stereotype: Option<&Stereotype>,
        extra: Option<SName>,
    ) -> Style {
        let labels = stereotype.map(Stereotype::style_names).unwrap_or_default();
        if labels.is_empty() {
            return self.get_merged_style(builder);
        }
        labels
            .iter()
            .map(|label| {
                let mut labelled = self.with_stereotype(label);
                if let Some(extra) = extra {
                    labelled = labelled.with_name(extra);
                }
                labelled.get_merged_style(builder)
            })
            .reduce(|result, style| result.merge_keeping_stereotype_values(&style))
            .expect("at least one label")
    }
}
