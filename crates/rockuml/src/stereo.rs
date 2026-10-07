//! Stereotypes like `<< Generated >>` or `<< (C,#ADD1B2) Testable >>`: labels shown in guillemets, and
//! an optional spot, a letter in a coloured circle (PlantUML's `Stereotype` and `StereotypeDecoration`).

use std::borrow::Cow;
use std::fmt::Write;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use crate::abel::LeafType;
use crate::color::{ColorType, Colors, HColor, NoSuchColor};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::sprite::{Sprite, SpriteContainer};
use crate::klimt::ugraphic::UGraphic;
use crate::pattern::{RegexTree, java_regex};

/// The pattern stereotypes are written with, as an optional part of a command.
pub(crate) fn optional_pattern(name: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::named(1, name, r"(\<\<.+?\>\>)")),
        RegexTree::spaces_zero_or_more(),
    ])
}

/// Tags like `$tag1 $tag2`, captured under `name` (`Stereotag.pattern`).
pub(crate) fn tags_pattern(name: &'static str) -> RegexTree {
    RegexTree::named(4, name, r"((\$[^%s{}%g<>$]+)([%s]+(\$[^%s{}%g<>$]+))*)?")
}

/// The style names written `<<<name>>>` in a stereotype, without repeats (`Stereostyles.build`).
pub(crate) fn stereostyles(label: &str) -> Vec<String> {
    static STYLE: LazyLock<Regex> = LazyLock::new(|| java_regex(r"\<{3}(.*?)\>{3}", false));
    let mut names: Vec<String> = Vec::new();
    for captures in STYLE.captures_iter(label) {
        let name = captures[1].to_owned();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// A letter in a coloured circle drawn before the labels.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spot {
    pub character: char,
    pub color: Option<HColor>,
}

/// A sprite drawn in place of the labels, like `<<$archimate/business-actor>>`.
#[derive(Clone, Debug, PartialEq)]
struct StereotypeSprite {
    name: String,
    scale: f64,
    color: HColor,
}

/// A stereotype's sprite, drawn in the stereotype's colour at its scale.
struct SpriteBlock {
    sprite: Rc<dyn Sprite>,
    color: HColor,
    scale: f64,
}

impl TextBlock for SpriteBlock {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.sprite
            .as_text_block(&self.color, None, self.scale, None)
            .calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.sprite
            .as_text_block(&self.color, None, self.scale, None)
            .draw_u(ug);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stereotype {
    /// The labels, each in `<<` and `>>`, without the spot.
    label: String,
    spot: Option<Spot>,
    sprite: Option<StereotypeSprite>,
}

impl Stereotype {
    /// A stereotype as written, labels only (`Stereotype.build(label)`).
    pub(crate) fn new(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            spot: None,
            sprite: None,
        }
    }

    /// A stereotype whose `(C,color)` parts become a spot and whose `$name` parts a sprite (`Stereotype.build`
    /// with a circled font).
    pub(crate) fn with_spot(full: &str) -> Result<Self, NoSuchColor> {
        static CIRCLE_CHAR: LazyLock<Regex> = LazyLock::new(|| {
            java_regex(
                r"^\<\<[ \t]*\((\S)(?:[ \t]*,[ \t]*(#[0-9a-fA-F]{6}|\w+)[ \t]*)?\)(?:[,]?(.*?))?\>\>$",
                false,
            )
        });
        static CIRCLE_SPRITE: LazyLock<Regex> = LazyLock::new(|| {
            java_regex(
                r"^\<\<[ \t]*\(?\$([-\p{L}0-9_/]+)((?:\{scale=|\*)([0-9.]+)\}?)?[ \t]*(?:,[ \t]*(#[0-9a-fA-F]{6}|\w+))?[ \t]*(?:[),](.*?))?\>\>$",
                false,
            )
        });
        let mut label = String::new();
        let mut spot = None;
        let mut sprite = None;
        for name in cut_labels(full) {
            if let Some(captures) = CIRCLE_SPRITE.captures(&name) {
                let color = captures
                    .get(4)
                    .map(|color| {
                        HColor::parse(color.as_str())
                            .ok()
                            .flatten()
                            .ok_or_else(|| NoSuchColor(color.as_str().to_owned()))
                    })
                    .transpose()?;
                sprite = Some(StereotypeSprite {
                    name: captures[1].to_owned(),
                    scale: captures
                        .get(3)
                        .and_then(|scale| scale.as_str().parse().ok())
                        .unwrap_or(1.0),
                    color: color.unwrap_or(HColor::BLACK),
                });
                spot = None;
                if let Some(rest) = captures.get(5).filter(|rest| !is_blank(rest.as_str())) {
                    let _ = write!(label, "<<{}>>", rest.as_str());
                }
                continue;
            }
            match CIRCLE_CHAR.captures(&name) {
                Some(captures) => {
                    let color = captures
                        .get(2)
                        .map(|color| {
                            HColor::parse(color.as_str())
                                .ok()
                                .flatten()
                                .ok_or_else(|| NoSuchColor(color.as_str().to_owned()))
                        })
                        .transpose()?;
                    let character = captures[1].chars().next().expect("one character");
                    spot = Some(Spot { character, color });
                    if let Some(rest) = captures.get(3).filter(|rest| !is_blank(rest.as_str())) {
                        let _ = write!(label, "<<{}>>", rest.as_str());
                    }
                }
                None => label.push_str(&name),
            }
        }
        Ok(Self {
            label,
            spot,
            sprite,
        })
    }

    pub(crate) fn spot(&self) -> Option<&Spot> {
        self.spot.as_ref()
    }

    /// The sprite the stereotype names, drawn in its colour at its scale, if `container` has it.
    pub(crate) fn get_sprite(&self, container: &dyn SpriteContainer) -> Option<Box<dyn TextBlock>> {
        let sprite = self.sprite.as_ref()?;
        Some(Box::new(SpriteBlock {
            sprite: container.get_sprite(&sprite.name)?,
            color: sprite.color.clone(),
            scale: sprite.scale,
        }))
    }

    /// `<<O-O>>`, which marks a state drawn with two linked circles in its corner.
    pub(crate) fn is_with_oo_symbol(&self) -> bool {
        self.label.eq_ignore_ascii_case("<<O-O>>")
    }

    /// The labels as shown, in guillemets (`getLabels(Guillemet.GUILLEMET)`).
    pub(crate) fn labels(&self) -> Vec<String> {
        cut_labels(&self.label_double_comparator())
            .iter()
            .map(|label| guillemets(label))
            .collect()
    }

    /// The labels as written, like `<<a>><<b>>` (`getLabel(Guillemet.DOUBLE_COMPARATOR)`); an archimate
    /// sprite is labelled by its name.
    pub(crate) fn label_double_comparator(&self) -> Cow<'_, str> {
        match self
            .sprite
            .as_ref()
            .and_then(|sprite| sprite.name.strip_prefix("archimate/"))
        {
            Some(archimate) => Cow::Owned(format!("<<{archimate}>>")),
            None => Cow::Borrowed(&self.label),
        }
    }

    /// Each label as written, like `<<a>>` (`getLabels(Guillemet.DOUBLE_COMPARATOR)`).
    pub(crate) fn labels_double_comparator(&self) -> Vec<String> {
        cut_labels(&self.label_double_comparator())
    }

    /// Each label's text, without brackets or the space next to them.
    pub(crate) fn multiple_labels(&self) -> Vec<String> {
        static LABEL: LazyLock<Regex> =
            LazyLock::new(|| java_regex(r"\<\<\s?((?:\<&\w+\>|[^<>])+?)\s?\>\>", false));
        LABEL
            .captures_iter(&self.label)
            .map(|captures| captures[1].to_owned())
            .collect()
    }

    /// The names style rules can select the stereotype by.
    pub(crate) fn style_names(&self) -> Vec<String> {
        let mut names: Vec<String> = cut_labels(&self.label)
            .iter()
            .map(|label| without_brackets(label).to_owned())
            .collect();
        if let Some((_, last)) = self
            .sprite
            .as_ref()
            .and_then(|sprite| sprite.name.rsplit_once('/'))
        {
            names.push(last.to_owned());
        }
        names
    }
}

/// As PlantUML writes it out, in tooltips for instance: the spot's letter, then the labels as written.
impl std::fmt::Display for Stereotype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.spot {
            Some(spot) => write!(f, "{} {}", spot.character, self.label),
            None => f.write_str(&self.label),
        }
    }
}

/// A `$tag` on an entity, named without its `$`, which `hide $tag` selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stereotag {
    pub name: String,
}

/// The stereotypes written after a state or an activity, like `<<choice>>` or `<<#pink>>`, some of which
/// change what the element is or how it is coloured (PlantUML's `Stereogroup`).
pub(crate) struct Stereogroup {
    definition: Option<String>,
}

impl Stereogroup {
    /// `<<a>> <<b>>...`, captured under `STEREOGROUP`.
    pub(crate) fn optional_pattern() -> RegexTree {
        RegexTree::optional(RegexTree::named(
            1,
            "STEREOGROUP",
            r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)",
        ))
    }

    pub(crate) fn build(definition: Option<&str>) -> Self {
        Self {
            definition: definition.map(str::to_owned),
        }
    }

    pub(crate) fn build_stereotype(&self) -> Option<Stereotype> {
        self.definition.as_deref().map(Stereotype::new)
    }

    /// The text of each `<<label>>`, trimmed.
    pub(crate) fn get_labels(&self) -> Vec<String> {
        static LABEL: LazyLock<Regex> = LazyLock::new(|| java_regex("<<([^<>]+)>>", false));
        let Some(definition) = &self.definition else {
            return Vec::new();
        };
        LABEL
            .captures_iter(definition)
            .map(|captures| captures[1].trim().to_owned())
            .collect()
    }

    /// The pseudo-state the first label makes of a state, like `<<choice>>` or `<<history*>>`.
    pub(crate) fn get_leaf_type(&self) -> Option<LeafType> {
        let labels = self.get_labels();
        match labels.first()?.to_lowercase().as_str() {
            "choice" => Some(LeafType::StateChoice),
            "fork" | "join" => Some(LeafType::StateForkJoin),
            "start" => Some(LeafType::CircleStart),
            "end" => Some(LeafType::CircleEnd),
            "history" => Some(LeafType::PseudoState),
            "history*" => Some(LeafType::DeepHistory),
            _ => None,
        }
    }

    /// The colours labels like `<<#pink>>`, `<<##[dashed]blue>>` (line) or `<<###red>>` (text) set.
    pub(crate) fn get_inner_colors(&self) -> Result<Colors, NoSuchColor> {
        let mut colors = Colors::default();
        for label in self.get_labels() {
            if let Some(text) = label.strip_prefix("###") {
                colors = colors.with(ColorType::Text, HColor::parse(text).ok().flatten());
            } else if let Some(line) = label.strip_prefix("##") {
                let mut line = line;
                if let Some(styled) = line.strip_prefix('[') {
                    line = match styled.split_once(']') {
                        Some((style, rest)) => {
                            colors = colors.add_legacy_stroke(style);
                            rest
                        }
                        None => "",
                    };
                }
                if !line.is_empty() {
                    colors = colors.with(ColorType::Line, HColor::parse(line).ok().flatten());
                }
            } else if label.starts_with('#') {
                colors = colors.merge_with(&Colors::parse(&label, ColorType::Back)?);
            }
        }
        Ok(colors)
    }
}

/// `StringUtils.isEmpty`: only spaces and tabs.
fn is_blank(text: &str) -> bool {
    text.chars().all(|c| matches!(c, ' ' | '\t'))
}

/// Every `<<label>>` in the text, skipping those written with three brackets.
fn cut_labels(label: &str) -> Vec<String> {
    static LABEL: LazyLock<Regex> = LazyLock::new(|| java_regex(r"\<{2,3}.*?\>{2,3}", false));
    LABEL
        .find_iter(label)
        .map(|found| found.as_str())
        .filter(|found| !found.starts_with("<<<"))
        .map(str::to_owned)
        .collect()
}

/// `Guillemet.manageGuillemetStrict` with `«` and `»`.
fn guillemets(label: &str) -> String {
    format!("\u{AB}{}\u{BB}", without_brackets(label))
}

fn without_brackets(label: &str) -> &str {
    let label = label
        .strip_prefix("<< ")
        .or_else(|| label.strip_prefix("<<"))
        .unwrap_or(label);
    label
        .strip_suffix(" >>")
        .or_else(|| label.strip_suffix(">>"))
        .unwrap_or(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_shown_in_guillemets() {
        let stereotype = Stereotype::new("<< Generated >><<Big>>");
        assert_eq!(
            stereotype.labels(),
            ["\u{AB}Generated\u{BB}", "\u{AB}Big\u{BB}"]
        );
        assert_eq!(stereotype.style_names(), ["Generated", "Big"]);
    }

    #[test]
    fn labels_are_compared_as_written_or_bare() {
        let stereotype = Stereotype::new("<< Generated >><<Big>>");
        assert_eq!(
            stereotype.label_double_comparator(),
            "<< Generated >><<Big>>"
        );
        assert_eq!(
            stereotype.labels_double_comparator(),
            ["<< Generated >>", "<<Big>>"]
        );
        assert_eq!(stereotype.multiple_labels(), ["Generated", "Big"]);
    }

    #[test]
    fn spots_are_split_from_the_labels() {
        let stereotype = Stereotype::with_spot("<< (C,#ADD1B2) Testable >>").unwrap();
        assert_eq!(
            stereotype.spot(),
            Some(&Spot {
                character: 'C',
                color: HColor::parse("#ADD1B2").unwrap()
            })
        );
        assert_eq!(stereotype.labels(), ["\u{AB}Testable\u{BB}"]);
        let only_spot = Stereotype::with_spot("<< (D,orchid) >>").unwrap();
        assert_eq!(only_spot.labels(), Vec::<String>::new());
        assert_eq!(only_spot.to_string(), "D ");
    }

    #[test]
    fn a_stereogroup_names_pseudo_states_and_colours() {
        let group =
            Stereogroup::build(Some("<< History* >> <<#pink>><<##[dashed]blue>><<###red>>"));
        assert_eq!(group.get_leaf_type(), Some(LeafType::DeepHistory));
        let colors = group.get_inner_colors().unwrap();
        let color = |name| HColor::parse(name).unwrap();
        assert_eq!(colors.get(ColorType::Back), color("pink").as_ref());
        assert_eq!(colors.get(ColorType::Line), color("blue").as_ref());
        assert_eq!(colors.get(ColorType::Text), color("red").as_ref());
        assert!(colors.get_specific_line_stroke().is_some());
        assert_eq!(Stereogroup::build(Some("<<custom>>")).get_leaf_type(), None);
        assert!(Stereogroup::build(None).build_stereotype().is_none());
    }

    #[test]
    fn written_out_with_the_spot_letter_first() {
        let stereotype = Stereotype::with_spot("<< (C,#ADD1B2) Testable >>").unwrap();
        assert_eq!(stereotype.to_string(), "C << Testable >>");
        let plain = Stereotype::new("<< Generated >>");
        assert_eq!(plain.to_string(), "<< Generated >>");
    }
}
