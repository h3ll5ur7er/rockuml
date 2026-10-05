//! `skinparam` settings rewritten as style rules (PlantUML's `FromSkinparamToStyle`).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use super::names::{PName, SName};
use super::signature::StyleSignature;
use super::value::Value;
use super::{STEREOTYPE_PRIORITY, Style};

/// One style property a skinparam sets.
struct Target {
    property: PName,
    selector: Vec<SName>,
}

/// Skinparam names in lowercase, each with its targets in PlantUML's registration order.
static KNOWLEDGE: LazyLock<Vec<(String, Target)>> = LazyLock::new(|| {
    use PName::{
        BackGroundColor, FontColor, FontName, FontSize, FontStyle, HeadColor, HorizontalAlignment,
        HyperLinkColor, HyperlinkUnderlineThickness, LineColor, LineStyle, LineThickness,
        MaximumWidth, MinimumWidth, RoundCorner,
    };
    use SName::{
        ActivationBox, Activity, ActivityBar, Actor, Agent, Archimate, Arrow, Artifact, Boundary,
        Box as BoxName, Caption, Card, Circle, Class, Clickable, Cloud, Collections, Component,
        Composite, Control, Database, Delay, Diamond, Document, Element, End, Entity, File, Folder,
        Footer, Frame, Group, GroupHeader, Header, Hexagon, Hnote, Interface, Legend, LifeLine,
        Map, Node, Note, Object, Package, Participant, Person, Private, Protected, Public, Queue,
        Rectangle, Reference, ReferenceHeader, Rnote, Root, Separator, SpotAbstractClass,
        SpotAnnotation, SpotClass, SpotDataClass, SpotEnum, SpotInterface, SpotRecord, Stack,
        Start, State, Stereotype, Stop, Storage, Swimlane, Title, Usecase, VisibilityIcon,
    };
    let mut knowledge = Knowledge::default();
    knowledge.convert(
        "participantClickableBackgroundColor",
        BackGroundColor,
        &[Participant, Clickable],
    );
    knowledge.convert(
        "participantClickableBorderColor",
        LineColor,
        &[Participant, Clickable],
    );
    for kind in [
        Participant,
        Boundary,
        Control,
        Collections,
        Actor,
        Database,
        Entity,
    ] {
        knowledge.magic(kind);
    }
    knowledge.font("header", &[Document, Header]);
    knowledge.font("footer", &[Document, Footer]);
    knowledge.font("caption", &[Document, Caption]);
    knowledge.convert("defaultFontSize", FontSize, &[Element]);
    knowledge.convert("sequenceStereotypeFontSize", FontSize, &[Stereotype]);
    knowledge.convert("sequenceStereotypeFontStyle", FontStyle, &[Stereotype]);
    knowledge.convert("sequenceStereotypeFontColor", FontColor, &[Stereotype]);
    knowledge.convert("sequenceStereotypeFontName", FontName, &[Stereotype]);
    knowledge.convert("SequenceReferenceBorderColor", LineColor, &[Reference]);
    knowledge.convert(
        "SequenceReferenceBorderColor",
        LineColor,
        &[ReferenceHeader],
    );
    knowledge.convert(
        "SequenceReferenceBackgroundColor",
        BackGroundColor,
        &[Reference],
    );
    knowledge.convert(
        "sequenceReferenceHeaderBackgroundColor",
        BackGroundColor,
        &[ReferenceHeader],
    );
    knowledge.font("sequenceReference", &[Reference]);
    knowledge.font("sequenceReference", &[ReferenceHeader]);
    knowledge.convert("sequenceGroupBorderThickness", LineThickness, &[Group]);
    knowledge.convert("SequenceGroupBorderColor", LineColor, &[Group]);
    knowledge.convert("SequenceGroupBorderColor", LineColor, &[GroupHeader]);
    knowledge.convert(
        "SequenceGroupBackgroundColor",
        BackGroundColor,
        &[GroupHeader],
    );
    knowledge.font("SequenceGroup", &[Group]);
    knowledge.font("SequenceGroupHeader", &[GroupHeader]);
    knowledge.convert("SequenceBoxBorderColor", LineColor, &[BoxName]);
    knowledge.convert("SequenceBoxBackgroundColor", BackGroundColor, &[BoxName]);
    knowledge.convert("SequenceBoxFontColor", FontColor, &[BoxName]);
    knowledge.convert("SequenceLifeLineBorderColor", LineColor, &[LifeLine]);
    knowledge.convert(
        "SequenceLifeLineBackgroundColor",
        BackGroundColor,
        &[ActivationBox],
    );
    knowledge.font("sequenceDelay", &[Delay]);
    knowledge.convert("sequenceDelayBorderColor", LineColor, &[Delay]);
    knowledge.convert(
        "sequenceDividerBackgroundColor",
        BackGroundColor,
        &[Separator],
    );
    knowledge.convert("sequenceDividerBorderColor", LineColor, &[Separator]);
    knowledge.font("sequenceDivider", &[Separator]);
    knowledge.convert(
        "sequenceDividerBorderThickness",
        LineThickness,
        &[Separator],
    );
    knowledge.convert("SequenceMessageAlignment", HorizontalAlignment, &[Arrow]);
    knowledge.font("note", &[Note]);
    knowledge.convert("noteBorderThickness", LineThickness, &[Note]);
    knowledge.convert("noteBorderColor", LineColor, &[Note]);
    knowledge.convert("noteBackgroundColor", BackGroundColor, &[Note]);
    knowledge.convert("packageBackgroundColor", BackGroundColor, &[Group]);
    knowledge.convert("packageBorderColor", LineColor, &[Group]);
    knowledge.magic(Package);
    knowledge.convert("PartitionBorderColor", LineColor, &[Composite]);
    knowledge.convert("PartitionBackgroundColor", BackGroundColor, &[Composite]);
    knowledge.font("Partition", &[Composite]);
    knowledge.convert("hyperlinkColor", HyperLinkColor, &[Root]);
    knowledge.convert("activityStartColor", BackGroundColor, &[Circle, Start]);
    knowledge.convert("activityEndColor", LineColor, &[Circle, End]);
    knowledge.convert("activityStopColor", LineColor, &[Circle, Stop]);
    knowledge.convert("activityBarColor", BackGroundColor, &[ActivityBar]);
    knowledge.convert("activityBorderColor", LineColor, &[Activity]);
    knowledge.convert("activityBorderThickness", LineThickness, &[Activity]);
    knowledge.convert("activityBackgroundColor", BackGroundColor, &[Activity]);
    knowledge.font("activity", &[Activity]);
    knowledge.convert(
        "activityDiamondBackgroundColor",
        BackGroundColor,
        &[Diamond],
    );
    knowledge.convert("activityDiamondBorderColor", LineColor, &[Diamond]);
    knowledge.font("activityDiamond", &[Diamond]);
    knowledge.font("arrow", &[Arrow]);
    knowledge.convert("arrowThickness", LineThickness, &[Arrow]);
    knowledge.convert("arrowColor", LineColor, &[Arrow]);
    knowledge.convert("arrowStyle", LineStyle, &[Arrow]);
    knowledge.convert("arrowHeadColor", HeadColor, &[Arrow]);
    knowledge.convert("defaulttextalignment", HorizontalAlignment, &[Root]);
    knowledge.convert("defaultFontName", FontName, &[Root]);
    knowledge.convert("defaultFontColor", FontColor, &[Root]);
    knowledge.font("SwimlaneTitle", &[Swimlane]);
    knowledge.convert("SwimlaneTitleBackgroundColor", BackGroundColor, &[Swimlane]);
    knowledge.convert("SwimlaneBorderColor", LineColor, &[Swimlane]);
    knowledge.convert("SwimlaneBorderThickness", LineThickness, &[Swimlane]);
    knowledge.convert("roundCorner", RoundCorner, &[Root]);
    knowledge.convert("titleBorderThickness", LineThickness, &[Title]);
    knowledge.convert("titleBorderColor", LineColor, &[Title]);
    knowledge.convert("titleBackgroundColor", BackGroundColor, &[Title]);
    knowledge.convert("titleBorderRoundCorner", RoundCorner, &[Title]);
    knowledge.font("title", &[Document, Title]);
    knowledge.convert("legendBorderThickness", LineThickness, &[Legend]);
    knowledge.convert("legendBorderColor", LineColor, &[Legend]);
    knowledge.convert("legendBackgroundColor", BackGroundColor, &[Legend]);
    knowledge.convert("legendBorderRoundCorner", RoundCorner, &[Legend]);
    knowledge.font("legend", &[Legend]);
    knowledge.convert("noteTextAlignment", HorizontalAlignment, &[Note]);
    knowledge.convert("BackgroundColor", BackGroundColor, &[Document]);
    knowledge.convert("classBackgroundColor", BackGroundColor, &[Element, Class]);
    knowledge.convert("classBorderColor", LineColor, &[Element, Class]);
    knowledge.convert("classFontSize", FontSize, &[Element, Class, Header]);
    knowledge.convert("classFontStyle", FontStyle, &[Element, Class, Header]);
    knowledge.convert("classFontColor", FontColor, &[Element, Class, Header]);
    knowledge.convert("classFontName", FontName, &[Element, Class, Header]);
    knowledge.convert("classAttributeFontSize", FontSize, &[Element, Class]);
    knowledge.convert("classAttributeFontStyle", FontStyle, &[Element, Class]);
    knowledge.convert("classAttributeFontColor", FontColor, &[Element, Class]);
    knowledge.convert("classAttributeFontName", FontName, &[Element, Class]);
    knowledge.convert("classBorderThickness", LineThickness, &[Element, Class]);
    knowledge.convert(
        "classHeaderBackgroundColor",
        BackGroundColor,
        &[Element, Class, Header],
    );
    knowledge.convert("objectBackgroundColor", BackGroundColor, &[Object]);
    knowledge.convert("objectBorderColor", LineColor, &[Object]);
    knowledge.font("object", &[Object]);
    knowledge.font("objectAttribute", &[Object]);
    knowledge.convert("objectBorderThickness", LineThickness, &[Object]);
    knowledge.convert("stateBackgroundColor", BackGroundColor, &[State]);
    knowledge.convert("stateBorderColor", LineColor, &[State]);
    knowledge.font("state", &[State]);
    knowledge.font("stateAttribute", &[State]);
    knowledge.convert("stateBorderThickness", LineThickness, &[State]);
    for kind in [
        Agent, Artifact, Card, Interface, Cloud, Component, File, Folder, Frame, Hexagon, Node,
        Person, Queue, Rectangle, Stack, Storage, Usecase, Map, Archimate, Hnote, Rnote,
    ] {
        knowledge.magic(kind);
    }
    knowledge.convert("IconPrivateColor", LineColor, &[VisibilityIcon, Private]);
    knowledge.convert(
        "IconPrivateBackgroundColor",
        BackGroundColor,
        &[VisibilityIcon, Private],
    );
    knowledge.convert("IconPackageColor", LineColor, &[VisibilityIcon, Package]);
    knowledge.convert(
        "IconPackageBackgroundColor",
        BackGroundColor,
        &[VisibilityIcon, Package],
    );
    knowledge.convert(
        "IconProtectedColor",
        LineColor,
        &[VisibilityIcon, Protected],
    );
    knowledge.convert(
        "IconProtectedBackgroundColor",
        BackGroundColor,
        &[VisibilityIcon, Protected],
    );
    knowledge.convert("IconPublicColor", LineColor, &[VisibilityIcon, Public]);
    knowledge.convert(
        "IconPublicBackgroundColor",
        BackGroundColor,
        &[VisibilityIcon, Public],
    );
    knowledge.convert("MinClassWidth", MinimumWidth, &[]);
    knowledge.convert("wrapWidth", MaximumWidth, &[Element]);
    knowledge.convert(
        "HyperlinkUnderline",
        HyperlinkUnderlineThickness,
        &[Element],
    );
    knowledge.convert("StereotypeAlignment", HorizontalAlignment, &[Stereotype]);
    knowledge.convert(
        "stereotypeABackgroundColor",
        BackGroundColor,
        &[SpotAbstractClass],
    );
    knowledge.convert("stereotypeABorderColor", LineColor, &[SpotAbstractClass]);
    knowledge.convert("stereotypeCBackgroundColor", BackGroundColor, &[SpotClass]);
    knowledge.convert("stereotypeCBorderColor", LineColor, &[SpotClass]);
    knowledge.convert("stereotypeEBackgroundColor", BackGroundColor, &[SpotEnum]);
    knowledge.convert("stereotypeEBorderColor", LineColor, &[SpotEnum]);
    knowledge.convert(
        "stereotypeIBackgroundColor",
        BackGroundColor,
        &[SpotInterface],
    );
    knowledge.convert("stereotypeIBorderColor", LineColor, &[SpotInterface]);
    knowledge.convert(
        "stereotypeNBackgroundColor",
        BackGroundColor,
        &[SpotAnnotation],
    );
    knowledge.convert("stereotypeNBorderColor", LineColor, &[SpotAnnotation]);
    knowledge.convert("stereotypeRBackgroundColor", BackGroundColor, &[SpotRecord]);
    knowledge.convert("stereotypeRBorderColor", LineColor, &[SpotRecord]);
    knowledge.convert(
        "stereotypeDBackgroundColor",
        BackGroundColor,
        &[SpotDataClass],
    );
    knowledge.convert("stereotypeDBorderColor", LineColor, &[SpotDataClass]);
    knowledge.0
});

#[derive(Default)]
struct Knowledge(Vec<(String, Target)>);

impl Knowledge {
    fn convert(&mut self, skinparam: &str, property: PName, selector: &[SName]) {
        let target = Target {
            property,
            selector: selector.to_vec(),
        };
        self.0.push((skinparam.to_lowercase(), target));
    }

    fn font(&mut self, skinparam: &str, selector: &[SName]) {
        for (suffix, property) in [
            ("FontSize", PName::FontSize),
            ("FontStyle", PName::FontStyle),
            ("FontColor", PName::FontColor),
            ("FontName", PName::FontName),
        ] {
            self.convert(&format!("{skinparam}{suffix}"), property, selector);
        }
    }

    /// The usual parameters of an element kind, named after it: `nodeBackgroundColor`, `nodeFontSize`...
    fn magic(&mut self, kind: SName) {
        let selector = &[kind];
        let name = kind.java_name().replace('_', "");
        for (suffix, property) in [
            ("BackgroundColor", PName::BackGroundColor),
            ("BorderColor", PName::LineColor),
            ("BorderThickness", PName::LineThickness),
            ("RoundCorner", PName::RoundCorner),
            ("DiagonalCorner", PName::DiagonalCorner),
            ("BorderStyle", PName::LineStyle),
        ] {
            self.convert(&format!("{name}{suffix}"), property, selector);
        }
        self.font(&name, selector);
        self.convert(&format!("{name}Shadowing"), PName::Shadowing, selector);
        for (suffix, property) in [
            ("StereotypeFontSize", PName::FontSize),
            ("StereotypeFontStyle", PName::FontStyle),
            ("StereotypeFontColor", PName::FontColor),
            ("StereotypeFontName", PName::FontName),
        ] {
            self.convert(
                &format!("{name}{suffix}"),
                property,
                &[SName::Stereotype, kind],
            );
        }
    }
}

/// The style rules for `skinparam key value`, numbered with `counter`. `key` is already normalised
/// (lowercase, `participant` for `sequenceParticipant`...), possibly with a `<<stereotype>>` suffix.
pub fn skinparam_styles(key: &str, value: &str, counter: &mut i32) -> Vec<Style> {
    let (key, stereotype) = split_stereotype(key);
    let mut converter = Converter {
        stereotype,
        counter,
        styles: Vec::new(),
    };
    converter.convert(key, value);
    converter.styles
}

/// `key<<stereo>>` names the key and the stereotype; Java tokenizes on angle brackets.
fn split_stereotype(key: &str) -> (&str, Option<&str>) {
    if !key.contains("<<") {
        return (key, None);
    }
    let mut tokens = key.split(['<', '>']).filter(|token| !token.is_empty());
    let key = tokens.next().unwrap_or_default();
    (key, tokens.next().map(crate::java::trim))
}

/// The skinparam value in the vocabulary of styles: switches become widths, line styles dash patterns.
fn style_value<'a>(key: &str, value: &'a str) -> &'a str {
    let is = |word: &str| value.eq_ignore_ascii_case(word);
    if key.ends_with("shadowing") && is("false") {
        "0"
    } else if key.ends_with("shadowing") && is("true") {
        "3"
    } else if key == "hyperlinkunderline" && is("false") {
        "0"
    } else if key == "hyperlinkunderline" && is("true") {
        "1"
    } else if is("right:right") {
        "right"
    } else if is("dotted") {
        "1;3"
    } else if is("dashed") {
        "7;7"
    } else {
        value
    }
}

struct Converter<'a> {
    stereotype: Option<&'a str>,
    counter: &'a mut i32,
    styles: Vec<Style>,
}

impl Converter<'_> {
    fn convert(&mut self, key: &str, value: &str) {
        let mut value = style_value(key, value).to_owned();
        let lowercase_key = key.to_lowercase();
        let targets: Vec<&Target> = KNOWLEDGE
            .iter()
            .filter(|(skinparam, _)| *skinparam == lowercase_key)
            .map(|(_, target)| target)
            .collect();
        if targets.is_empty() {
            if key.eq_ignore_ascii_case("shadowing") {
                let value = self.shadowing_value(&value);
                self.add(PName::Shadowing, value, &[SName::Root]);
            } else if key.eq_ignore_ascii_case("noteshadowing") {
                let value = self.shadowing_value(&value);
                self.add(PName::Shadowing, value, &[SName::Root, SName::Note]);
            }
            return;
        }

        if value.contains(';') || value.starts_with("text:") {
            if !value.contains(';') {
                self.read_decoration(&value, &targets);
                return;
            }
            if value.starts_with(';') {
                value.insert(0, ' ');
            }
            let mut parts = value.split(';').filter(|part| !part.is_empty());
            let first = parts.next().unwrap_or_default().to_owned();
            for decoration in parts {
                self.read_decoration(decoration, &targets);
            }
            value = first;
        }
        if value != " " {
            for target in targets {
                let declared = self.next_value(&value);
                self.add(target.property, declared, &target.selector);
            }
        }
    }

    /// `text:color`, `line.dotted`, `line.dashed` or `bold` after a `;` in a value.
    fn read_decoration(&mut self, decoration: &str, targets: &[&Target]) {
        let (property, text) = if decoration.starts_with("text:") {
            let color = decoration.split(':').nth(1).unwrap_or_default();
            (PName::FontColor, color)
        } else if decoration.starts_with("line.dotted") {
            (PName::LineStyle, "1;3")
        } else if decoration.starts_with("line.dashed") {
            (PName::LineStyle, "7;7")
        } else if decoration.to_lowercase().contains("bold") {
            (PName::LineThickness, "2")
        } else {
            return;
        };
        for target in targets {
            let declared = self.next_value(text);
            self.add(property, declared, &target.selector);
        }
    }

    fn shadowing_value(&mut self, value: &str) -> Value {
        let value = if value.eq_ignore_ascii_case("false") || value.eq_ignore_ascii_case("no") {
            "0"
        } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes") {
            "3"
        } else {
            value
        };
        self.next_value(value)
    }

    fn next_value(&mut self, text: &str) -> Value {
        *self.counter += 1;
        Value::regular(text, *self.counter)
    }

    fn add(&mut self, property: PName, value: Value, selector: &[SName]) {
        let mut signature = StyleSignature::of(selector);
        let value = match self.stereotype {
            Some(stereotypes) => {
                for stereotype in crate::java::split(stereotypes, "&") {
                    signature = signature.with_stereotype(&stereotype);
                }
                value.with_added_priority(STEREOTYPE_PRIORITY)
            }
            None => value,
        };
        self.styles
            .push(Style::new(signature, BTreeMap::from([(property, value)])));
    }
}

#[cfg(test)]
mod tests {
    use super::super::ValueReading;
    use super::*;

    fn converted(key: &str, value: &str) -> Vec<String> {
        let mut counter = 0;
        skinparam_styles(key, value, &mut counter)
            .iter()
            .map(|style| {
                let (property, value) = style.properties.iter().next().unwrap();
                format!(
                    "{:?} {property:?}={}({})",
                    style.signature(),
                    Some(value).as_string(),
                    value.priority()
                )
            })
            .collect()
    }

    #[test]
    fn each_target_gets_the_next_declaration_number() {
        let styles = converted("sequencereferencebordercolor", "red");
        assert_eq!(styles.len(), 2);
        assert!(styles[0].ends_with("LineColor=red(1)"));
        assert!(styles[1].ends_with("LineColor=red(2)"));
    }

    #[test]
    fn magic_parameters_exist_for_element_kinds() {
        assert_eq!(converted("nodebackgroundcolor", "red").len(), 1);
        assert_eq!(converted("interfacestereotypefontsize", "9").len(), 1);
        // Registered for groups and again as a package magic parameter.
        assert_eq!(converted("packagebordercolor", "red").len(), 2);
    }

    #[test]
    fn decorations_after_semicolons_set_more_properties() {
        let styles = converted("arrowcolor", "red;line.dashed;text:blue");
        let properties: Vec<&str> = styles
            .iter()
            .map(|style| style.rsplit(' ').next().unwrap())
            .collect();
        assert_eq!(
            properties,
            ["LineStyle=7;7(1)", "FontColor=blue(2)", "LineColor=red(3)"]
        );
    }

    #[test]
    fn stereotyped_keys_make_stereotype_rules() {
        let mut counter = 0;
        let styles = skinparam_styles("nodebackgroundcolor<<db&big>>", "red", &mut counter);
        assert_eq!(
            styles[0].value(PName::BackGroundColor).unwrap().priority(),
            1 + STEREOTYPE_PRIORITY
        );
        assert!(styles[0].signature().has_stereotypes());
    }

    #[test]
    fn a_trailing_ampersand_adds_no_stereotype() {
        let signature = |key| {
            let mut counter = 0;
            skinparam_styles(key, "red", &mut counter)[0]
                .signature()
                .clone()
        };
        assert_eq!(
            signature("nodebackgroundcolor<<db&>>"),
            signature("nodebackgroundcolor<<db>>")
        );
    }

    #[test]
    fn plain_shadowing_switches_map_to_widths() {
        assert_eq!(converted("shadowing", "true").len(), 1);
        assert!(converted("shadowing", "true")[0].ends_with("Shadowing=3(1)"));
        assert_eq!(converted("unknownparam", "x"), Vec::<String>::new());
    }
}
