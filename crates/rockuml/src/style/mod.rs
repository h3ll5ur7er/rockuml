//! PlantUML's style sheets: the `.skin` defaults and `<style>` blocks, CSS-like rules whose properties
//! merge by specificity and declaration order.

mod from_skinparam;
mod names;
mod parser;
mod signature;
mod value;

use std::collections::BTreeMap;

pub use from_skinparam::skinparam_styles;
pub use names::{PName, SName};
pub use parser::{StyleParser, StyleParsingError};
pub use signature::StyleSignature;
pub use value::{Value, ValueReading};

/// Raised for properties of stereotype rules, so that they beat plain rules.
const STEREOTYPE_PRIORITY: i32 = 1000;

/// The properties of one rule, or of an element once every matching rule is merged.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    signature: StyleSignature,
    properties: BTreeMap<PName, Value>,
}

impl Style {
    pub fn new(signature: StyleSignature, properties: BTreeMap<PName, Value>) -> Self {
        Self {
            signature,
            properties,
        }
    }

    pub fn signature(&self) -> &StyleSignature {
        &self.signature
    }

    pub fn value(&self, name: PName) -> Option<&Value> {
        self.properties.get(&name)
    }

    pub fn has_value(&self, name: PName) -> bool {
        self.properties.contains_key(&name)
    }

    /// `other` declared over this style: its properties win unless declared with a lower priority.
    #[must_use]
    pub fn merge_with(&self, other: &Style) -> Style {
        let mut properties = self.properties.clone();
        for (&name, value) in &other.properties {
            properties.insert(name, value.merge_with(self.properties.get(&name)));
        }
        Style::new(self.signature.merge_with(&other.signature), properties)
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
pub struct StyleBuilder {
    storage: StyleStorage,
    /// Numbers declarations so that later ones get higher priorities.
    counter: i32,
}

impl StyleBuilder {
    /// The builder with the rules of one of PlantUML's embedded `.skin` files, like `plantuml.skin`;
    /// `None` if there is no such skin.
    pub fn load_skin(name: &str) -> Option<Self> {
        let text = crate::assets::get(&format!("skin/{name}"))?;
        let text = String::from_utf8_lossy(text);
        let lines: Vec<&str> = text.lines().collect();
        let mut builder = Self::default();
        let styles = StyleParser::new(&mut builder.counter).parse(&lines).ok()?;
        builder.mute(styles);
        Some(builder)
    }

    /// Adds rules, each merged over an existing rule of the same signature.
    pub fn mute(&mut self, styles: impl IntoIterator<Item = Style>) {
        for style in styles {
            let merged = match self.storage.get(style.signature()) {
                Some(existing) => existing.merge_with(&style),
                None => style,
            };
            self.storage.put(merged);
        }
    }

    /// Numbers declarations parsed or converted for this builder.
    pub fn counter(&mut self) -> &mut i32 {
        &mut self.counter
    }

    /// The style of an element: every rule that applies to it, merged in storage order.
    pub fn merged_style(&self, element: &StyleSignature) -> Option<Style> {
        self.storage
            .styles()
            .filter(|style| style.signature().matches(element))
            .fold(None, |merged: Option<Style>, style| match merged {
                None => Some(style.clone()),
                Some(merged) => Some(merged.merge_with(style)),
            })
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
        let builder = StyleBuilder::load_skin("plantuml.skin").unwrap();
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
