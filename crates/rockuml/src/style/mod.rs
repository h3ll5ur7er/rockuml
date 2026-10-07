//! PlantUML's style sheets: the `.skin` defaults and `<style>` blocks, CSS-like rules whose properties
//! merge by specificity and declaration order.

mod from_skinparam;
mod names;
mod parser;
mod signature;
mod style_values;
mod value;

use std::collections::BTreeMap;

use crate::stereo::Stereotype;
use from_skinparam::skinparam_styles;
pub(crate) use names::{PName, SName};
use parser::StyleParser;
pub(crate) use parser::StyleParsingError;
pub(crate) use signature::StyleSignature;
pub(crate) use style_values::max_width;
pub(crate) use value::{Value, ValueReading};

/// Raised for properties of stereotype rules, so that they beat plain rules.
const STEREOTYPE_PRIORITY: i32 = 1000;

/// The properties of one rule, or of an element once every matching rule is merged.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Style {
    signature: StyleSignature,
    properties: BTreeMap<PName, Value>,
}

impl Style {
    pub(crate) fn new(signature: StyleSignature, properties: BTreeMap<PName, Value>) -> Self {
        Self {
            signature,
            properties,
        }
    }

    pub(crate) fn signature(&self) -> &StyleSignature {
        &self.signature
    }

    pub(crate) fn value(&self, name: PName) -> Option<&Value> {
        self.properties.get(&name)
    }

    pub(crate) fn has_value(&self, name: PName) -> bool {
        self.properties.contains_key(&name)
    }

    fn with_value(&self, name: PName, value: Value) -> Style {
        let mut properties = self.properties.clone();
        properties.insert(name, value);
        Style::new(self.signature.clone(), properties)
    }

    /// `other` declared over this style: its properties win unless declared with a lower priority.
    #[must_use]
    pub(crate) fn merge_with(&self, other: &Style) -> Style {
        self.merge(other, false)
    }

    /// Like [`Self::merge_with`], except that properties a stereotype rule set here are kept
    /// (`MergeStrategy.KEEP_EXISTING_VALUE_OF_STEREOTYPE`).
    #[must_use]
    pub(crate) fn merge_keeping_stereotype_values(&self, other: &Style) -> Style {
        self.merge(other, true)
    }

    fn merge(&self, other: &Style, keep_stereotype_values: bool) -> Style {
        let mut properties = self.properties.clone();
        for (&name, value) in &other.properties {
            let previous = self.properties.get(&name);
            if keep_stereotype_values
                && previous.is_some_and(|previous| previous.priority() > STEREOTYPE_PRIORITY)
            {
                continue;
            }
            properties.insert(name, value.merge_with(previous));
        }
        Style::new(self.signature.merge_with(&other.signature), properties)
    }

    /// A nested rule like `group { header {} }` over this flat one like `groupHeader`, keeping only what the
    /// nested rule sets itself rather than inherits from `nested_parent`.
    #[must_use]
    pub(crate) fn merge_nested_child_over(&self, nested: &Style, nested_parent: &Style) -> Style {
        let divergent: BTreeMap<PName, Value> = nested
            .properties
            .iter()
            .filter(|(name, value)| {
                nested_parent
                    .properties
                    .get(name)
                    .is_none_or(|ancestor| Some(ancestor).as_string() != Some(*value).as_string())
            })
            .map(|(name, value)| (*name, value.clone()))
            .collect();
        if divergent.is_empty() {
            return self.clone();
        }
        self.merge_with(&Style::new(nested.signature.clone(), divergent))
    }
}

/// Rules in declaration order, those with stereotypes ahead of the others: the order PlantUML merges them in.
#[derive(Clone, Debug, Default)]
struct StyleStorage {
    with_stereotypes: Vec<Style>,
    plain: Vec<Style>,
}

impl StyleStorage {
    fn list(&self, signature: &StyleSignature) -> &Vec<Style> {
        if signature.has_stereotypes() {
            &self.with_stereotypes
        } else {
            &self.plain
        }
    }

    fn get(&self, signature: &StyleSignature) -> Option<&Style> {
        self.list(signature)
            .iter()
            .find(|style| style.signature() == signature)
    }

    /// Replaces a rule in place, keeping its position, or appends a new one.
    fn put(&mut self, style: Style) {
        let list = if style.signature().has_stereotypes() {
            &mut self.with_stereotypes
        } else {
            &mut self.plain
        };
        match list
            .iter_mut()
            .find(|existing| existing.signature() == style.signature())
        {
            Some(existing) => *existing = style,
            None => list.push(style),
        }
    }

    fn styles(&self) -> impl Iterator<Item = &Style> {
        self.with_stereotypes.iter().chain(&self.plain)
    }
}

/// All rules in force for a diagram: the skin's, then the diagram's own.
#[derive(Clone, Debug, Default)]
pub(crate) struct StyleBuilder {
    storage: StyleStorage,
    /// Numbers declarations so that later ones get higher priorities.
    counter: i32,
}

impl StyleBuilder {
    /// The builder with the rules of one of PlantUML's embedded `.skin` files, like `plantuml.skin`.
    ///
    /// # Panics
    /// See [`Self::apply_skin`].
    pub(crate) fn load_skin(name: &str) -> Self {
        let mut builder = Self::default();
        builder.apply_skin(name);
        builder
    }

    /// The rules of an embedded `.skin` file, merged over the current ones.
    ///
    /// # Panics
    /// If no such skin is embedded, or it does not parse: skins are named by the code, never by users.
    pub(crate) fn apply_skin(&mut self, name: &str) {
        let text = crate::assets::get(&format!("skin/{name}"))
            .unwrap_or_else(|| panic!("{name} is embedded"));
        let text = String::from_utf8_lossy(text);
        let lines: Vec<&str> = text.lines().collect();
        self.apply_style_sheet(&lines)
            .unwrap_or_else(|error| panic!("{name} parses: {error:?}"));
    }

    /// The rules of a style sheet, merged over the current ones.
    pub(crate) fn apply_style_sheet(&mut self, lines: &[&str]) -> Result<(), StyleParsingError> {
        let styles = StyleParser::new(&mut self.counter).parse(lines)?;
        self.mute(styles);
        Ok(())
    }

    /// Adds rules, each merged over an existing rule of the same signature.
    fn mute(&mut self, styles: impl IntoIterator<Item = Style>) {
        for style in styles {
            let merged = match self.storage.get(style.signature()) {
                Some(existing) => existing.merge_with(&style),
                None => style,
            };
            self.storage.put(merged);
        }
    }

    /// The rules `skinparam key value` stands for, merged over the current ones.
    pub(crate) fn apply_skinparam(&mut self, key: &str, value: &str) {
        let styles = skinparam_styles(key, value, &mut self.counter);
        self.mute(styles);
    }

    /// The style of an element: every rule that applies to it, merged in storage order.
    pub(crate) fn merged_style(&self, element: &StyleSignature) -> Option<Style> {
        self.storage
            .styles()
            .filter(|style| style.signature().matches(element))
            .fold(None, |merged: Option<Style>, style| match merged {
                None => Some(style.clone()),
                Some(merged) => Some(merged.merge_with(style)),
            })
    }

    /// The style of an element with a stereotype: the stereotype's rules for each of its labels, merged
    /// (`withTOBECHANGED` and `StyleSignatures.getMergedStyle`).
    pub(crate) fn merged_style_with_stereotype(
        &self,
        signature: &StyleSignature,
        stereotype: Option<&Stereotype>,
    ) -> Option<Style> {
        self.merged_style_for_labels(signature, stereotype, None)
    }

    /// The style of an element's stereotype itself (`forStereotypeItself`).
    pub(crate) fn merged_style_for_stereotype_itself(
        &self,
        signature: &StyleSignature,
        stereotype: Option<&Stereotype>,
    ) -> Option<Style> {
        self.merged_style_for_labels(signature, stereotype, Some(SName::Stereotype))
    }

    fn merged_style_for_labels(
        &self,
        signature: &StyleSignature,
        stereotype: Option<&Stereotype>,
        extra: Option<SName>,
    ) -> Option<Style> {
        let labels = stereotype.map(Stereotype::style_names).unwrap_or_default();
        if labels.is_empty() {
            return self.merged_style(signature);
        }
        labels
            .iter()
            .map(|label| {
                let mut labelled = signature.with_stereotype(label);
                if let Some(extra) = extra {
                    labelled = labelled.with_name(extra);
                }
                self.merged_style(&labelled)
            })
            .reduce(|result, style| match (result, style) {
                (Some(result), Some(style)) => Some(result.merge_keeping_stereotype_values(&style)),
                (result, style) => result.or(style),
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every property of the merged style as `Name=value(priority)`, in declaration order of the names.
    fn dump(builder: &StyleBuilder, names: &[SName]) -> String {
        let style = builder.merged_style(&StyleSignature::of(names)).unwrap();
        style
            .properties
            .iter()
            .map(|(name, value)| {
                format!("{name:?}={}({})", Some(value).as_string(), value.priority())
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Expectations printed by PlantUML's `StyleLoader.loadSkin("plantuml.skin")` for the same signatures.
    #[test]
    fn the_default_skin_resolves_like_plantuml() {
        use SName::{
            Class, ClassDiagram, Document, Element, Header, MindmapDiagram, Node, Participant,
            Root, SequenceDiagram, Title,
        };
        let builder = StyleBuilder::load_skin("plantuml.skin");
        let cases: [(&[SName], &str); 6] = [
            (
                &[Root],
                "Shadowing=0.0(13) FontName=SansSerif(1) FontColor=black(4) FontSize=14(5) FontStyle=plain(6) BackGroundColor=#f1f1f1(12) RoundCorner=0(8) LineThickness=1.0(10) DiagonalCorner=0(9) HyperLinkColor=blue(2) HyperlinkUnderlineThickness=1(3) LineColor=#181818(11) HorizontalAlignment=left(7)",
            ),
            (
                &[Root, Document],
                "Shadowing=0.0(13) FontName=SansSerif(1) FontColor=black(4) FontSize=14(5) FontStyle=plain(6) BackGroundColor=white(14) RoundCorner=0(8) LineThickness=1.0(10) DiagonalCorner=0(9) HyperLinkColor=blue(2) HyperlinkUnderlineThickness=1(3) LineColor=#181818(11) HorizontalAlignment=left(7)",
            ),
            (
                &[Root, Document, Title],
                "Shadowing=0.0(13) FontName=SansSerif(1) FontColor=black(4) FontSize=14(21) FontStyle=bold(22) BackGroundColor=transparent(26) RoundCorner=0(8) LineThickness=1.0(10) DiagonalCorner=0(9) HyperLinkColor=blue(2) HyperlinkUnderlineThickness=1(3) LineColor=transparent(25) Padding=5(23) Margin=5(24) HorizontalAlignment=center(20)",
            ),
            (
                &[Root, Element, MindmapDiagram, Node],
                "Shadowing=0.0(53) FontName=SansSerif(1) FontColor=black(4) FontSize=14(5) FontStyle=plain(6) BackGroundColor=#f1f1f1(12) RoundCorner=25(155) LineThickness=1.5(156) DiagonalCorner=0(9) HyperLinkColor=blue(2) HyperlinkUnderlineThickness=1(3) LineColor=#181818(11) Padding=10(153) Margin=10(154) HorizontalAlignment=left(7)",
            ),
            (
                &[Root, Element, SequenceDiagram, Participant],
                "Shadowing=0.0(53) FontName=SansSerif(1) FontColor=black(4) FontSize=14(5) FontStyle=plain(6) BackGroundColor=#e2e2f0(104) RoundCorner=5(103) LineThickness=0.5(54) DiagonalCorner=0(9) HyperLinkColor=blue(2) HyperlinkUnderlineThickness=1(3) LineColor=#181818(11) Padding=7(106) HorizontalAlignment=center(105)",
            ),
            (
                &[Root, Element, ClassDiagram, Class, Header],
                "Shadowing=0.0(53) FontName=SansSerif(1) FontColor=black(4) FontSize=14(5) FontStyle=plain(6) BackGroundColor=#f1f1f1(12) RoundCorner=5(108) LineThickness=0.5(54) DiagonalCorner=0(9) HyperLinkColor=blue(2) HyperlinkUnderlineThickness=1(3) LineColor=#181818(11) HorizontalAlignment=left(7)",
            ),
        ];
        for (names, expected) in cases {
            assert_eq!(dump(&builder, names), expected, "{names:?}");
        }
    }
}
