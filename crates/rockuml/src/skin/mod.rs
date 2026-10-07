//! A diagram's `skinparam` settings and the styles they feed (PlantUML's `SkinParam`).

pub(crate) mod actor;
pub(crate) mod arrow;
pub(crate) mod body;
pub(crate) mod component;
pub(crate) mod component_style;
pub(crate) mod rose;
pub(crate) mod symbol;

use actor::ActorStyle;
use component_style::ComponentStyle;

use crate::decoration::symbol::PackageStyle;

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use crate::color::HColor;
use crate::diagram::UmlSource;
use crate::java;
use crate::klimt::HorizontalAlignment;
use crate::klimt::sprite::{Sprite, SpriteContainer, SpriteImage};
use crate::pattern::java_regex;
use crate::style::{
    PName, SName, Style, StyleBuilder, StyleParsingError, StyleSignature, ValueReading,
};

const DEFAULT_SKIN: &str = "plantuml.skin";

#[derive(Default)]
pub(crate) struct SkinParam {
    /// Loaded from the default skin when first needed. Diagram elements keep the builder in force when they
    /// were declared, so a change makes a new builder rather than changing the shared one.
    style_builder: OnceCell<Rc<StyleBuilder>>,
    params: HashMap<String, String>,
    /// PlantUML remembers every value it looked up, even when a later `skinparam` changes it.
    looked_up: RefCell<HashMap<String, Option<String>>>,
    sprites: HashMap<String, Rc<dyn Sprite>>,
    /// The base64 data of the PNGs the source refers to by MD5.
    md5_map: HashMap<String, String>,
    /// The files and URLs the source's `<img>`s name, by name; `None` for those that could not be read.
    image_files: HashMap<String, Option<Vec<u8>>>,
    rankdir: Rankdir,
}

/// Which way entity diagrams flow (PlantUML's `Rankdir`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Rankdir {
    #[default]
    TopToBottom,
    LeftToRight,
}

impl SpriteContainer for SkinParam {
    fn get_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>> {
        self.sprites
            .get(name)
            .cloned()
            .or_else(|| SpriteImage::from_internal(name))
    }

    fn get_from_md5(&self, md5: &str) -> Option<&str> {
        self.md5_map.get(md5).map(String::as_str)
    }

    fn image_file(&self, src: &str) -> Option<&[u8]> {
        self.image_files.get(src)?.as_deref()
    }
}

impl SkinParam {
    /// With the images of `source`.
    pub(crate) fn new(source: &UmlSource) -> Self {
        Self {
            md5_map: source.md5_map().clone(),
            image_files: source.image_files().clone(),
            ..Self::default()
        }
    }

    pub(crate) fn get_rankdir(&self) -> Rankdir {
        self.rankdir
    }

    pub(crate) fn set_rankdir(&mut self, rankdir: Rankdir) {
        self.rankdir = rankdir;
    }

    pub(crate) fn add_sprite(&mut self, name: String, sprite: Rc<dyn Sprite>) {
        self.sprites.insert(name, sprite);
    }

    fn style_builder(&self) -> &StyleBuilder {
        self.style_builder
            .get_or_init(|| Rc::new(StyleBuilder::load_skin(DEFAULT_SKIN)))
    }

    /// The rules in force now, which later changes leave untouched.
    pub(crate) fn current_style_builder(&self) -> Rc<StyleBuilder> {
        self.style_builder();
        self.style_builder.get().expect("just initialised").clone()
    }

    fn style_builder_mut(&mut self) -> &mut StyleBuilder {
        self.style_builder();
        Rc::make_mut(self.style_builder.get_mut().expect("just initialised"))
    }

    pub(crate) fn merged_style(&self, element: &StyleSignature) -> Option<Style> {
        self.style_builder().merged_style(element)
    }

    /// The rules of a `<style>` block, merged over the current ones.
    pub(crate) fn apply_style_sheet(&mut self, lines: &[&str]) -> Result<(), StyleParsingError> {
        self.style_builder_mut().apply_style_sheet(lines)
    }

    /// `skinparam key value`: remembered under the normalised key, and turned into style rules.
    pub(crate) fn set_param(&mut self, key: &str, value: &str) {
        for normalised in clean_for_key(key) {
            self.params
                .insert(normalised.clone(), java::trim(value).to_owned());
            self.style_builder_mut().apply_skinparam(&normalised, value);
        }
        if key.eq_ignore_ascii_case("style") && value.eq_ignore_ascii_case("strictuml") {
            self.style_builder_mut().apply_skin("strictuml.skin");
        }
    }

    fn value_is(&self, key: &str, expected: &str) -> bool {
        self.value(key)
            .is_some_and(|value| value.eq_ignore_ascii_case(expected))
    }

    /// `skinparam backgroundColor`, or the document style's background.
    pub(crate) fn get_background_color(&self) -> HColor {
        match self.value("backgroundcolor") {
            Some(value)
                if value.eq_ignore_ascii_case("transparent")
                    || value.eq_ignore_ascii_case("none") =>
            {
                HColor::NONE
            }
            Some(value) => HColor::parse_or_white(&value),
            None => self
                .merged_style(&StyleSignature::of(&[SName::Root, SName::Document]))
                .expect("the skin styles the document")
                .value(PName::BackGroundColor)
                .as_color(),
        }
    }

    pub(crate) fn strict_uml_style(&self) -> bool {
        self.value_is("style", "strictuml")
    }

    pub(crate) fn response_message_below_arrow(&self) -> bool {
        self.value_is("responsemessagebelowarrow", "true")
    }

    pub(crate) fn actor_style(&self) -> ActorStyle {
        ActorStyle::named(&self.value("actorstyle").unwrap_or_default())
    }

    /// Strict UML draws UML 2 components whatever `componentStyle` says.
    pub(crate) fn component_style(&self) -> ComponentStyle {
        if self.strict_uml_style() {
            return ComponentStyle::Uml2;
        }
        let value = self.value("componentstyle").unwrap_or_default();
        if value.eq_ignore_ascii_case("uml1") {
            ComponentStyle::Uml1
        } else if value.eq_ignore_ascii_case("rectangle") {
            ComponentStyle::Rectangle
        } else {
            ComponentStyle::Uml2
        }
    }

    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "read by the package commands, which are ported next"
        )
    )]
    pub(crate) fn package_style(&self) -> PackageStyle {
        self.value("packageStyle")
            .and_then(|value| PackageStyle::from_string(&value))
            .unwrap_or(PackageStyle::Folder)
    }

    /// `stereotypeAlignment`, centred by default.
    pub(crate) fn stereotype_alignment(&self) -> HorizontalAlignment {
        self.value("stereotypealignment")
            .and_then(|value| HorizontalAlignment::from_name(&value))
            .unwrap_or(HorizontalAlignment::Center)
    }

    /// `defaultTextAlignment`, or else `default`.
    pub(crate) fn default_text_alignment(
        &self,
        default: HorizontalAlignment,
    ) -> HorizontalAlignment {
        self.value("defaulttextalignment")
            .and_then(|value| HorizontalAlignment::from_name(&value))
            .unwrap_or(default)
    }

    /// `noteTextAlignment`, then `defaultTextAlignment`, then `default`.
    pub(crate) fn note_text_alignment(&self, default: HorizontalAlignment) -> HorizontalAlignment {
        ["noteTextAlignment", "defaulttextalignment"]
            .iter()
            .find_map(|key| {
                self.value(key)
                    .and_then(|value| HorizontalAlignment::from_name(&value))
            })
            .unwrap_or(default)
    }

    pub(crate) fn force_sequence_participant_underlined(&self) -> bool {
        self.value_is("sequenceParticipant", "underline")
    }

    /// `maxMessageSize`: how wide messages may grow before they wrap, or 0.
    pub(crate) fn max_message_size(&self) -> f64 {
        let value = self
            .value("wrapmessagewidth")
            .or_else(|| self.value("maxmessagesize"));
        crate::style::max_width(&value.unwrap_or_default())
    }

    /// `BoxPadding`: the room each side of a box around participants, or 0 unless a plain decimal.
    pub(crate) fn box_padding(&self) -> f64 {
        self.value("boxPadding")
            .filter(|value| is_int_or_decimal(value))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.0)
    }

    pub(crate) fn value(&self, key: &str) -> Option<String> {
        if let Some(known) = self.looked_up.borrow().get(key) {
            return known.clone();
        }
        let found = clean_for_key(key)
            .iter()
            .find_map(|normalised| self.params.get(normalised))
            .cloned();
        self.looked_up
            .borrow_mut()
            .insert(key.to_owned(), found.clone());
        found
    }
}

/// `\d+(\.\d+)?`.
fn is_int_or_decimal(value: &str) -> bool {
    let is_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    match value.split_once('.') {
        Some((int, decimals)) => is_digits(int) && is_digits(decimals),
        None => is_digits(value),
    }
}

/// The keys a skinparam is stored under: lowercase, without `_` or `.`, with long and legacy names
/// shortened, and one key per `<<stereotype>>` in it.
fn clean_for_key(key: &str) -> Vec<String> {
    static SEQUENCE: LazyLock<Regex> =
        LazyLock::new(|| java_regex("sequence(participant|actor)", false));
    static ARROW: LazyLock<Regex> = LazyLock::new(|| {
        java_regex(
            "(activity|class|component|object|sequence|state|usecase)arrow",
            false,
        )
    });
    static ALIGN: LazyLock<Regex> = LazyLock::new(|| java_regex("align$", false));
    // PlantUML finds the stereotypes case-insensitively and removes them case-sensitively, which is the
    // same for a pattern without letters.
    static STEREOTYPE: LazyLock<Regex> = LazyLock::new(|| java_regex(r"\<\<(.*?)\>\>", false));

    let key = java::trim(&key.to_lowercase()).replace(['_', '.'], "");
    let key = SEQUENCE.replace_all(&key, "$1");
    let key = ARROW.replace_all(&key, "arrow");
    let key = ALIGN.replace_all(&key, "alignment");
    let without_stereotypes = STEREOTYPE.replace_all(&key, "");
    let keys: Vec<String> = STEREOTYPE
        .captures_iter(&key)
        .map(|captures| format!("{without_stereotypes}<<{}>>", &captures[1]))
        .collect();
    if keys.is_empty() {
        vec![key.into_owned()]
    } else {
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_normalised_like_plantuml() {
        assert_eq!(
            clean_for_key(" Sequence_Participant.BackgroundColor "),
            ["participantbackgroundcolor"]
        );
        assert_eq!(clean_for_key("ClassArrowColor"), ["arrowcolor"]);
        assert_eq!(clean_for_key("noteTextAlign"), ["notetextalignment"]);
        assert_eq!(
            clean_for_key("nodeBackgroundColor<<DB>><<Big>>"),
            ["nodebackgroundcolor<<db>>", "nodebackgroundcolor<<big>>"]
        );
    }

    #[test]
    fn skinparams_change_the_styles() {
        let mut skin = SkinParam::default();
        skin.set_param("BackgroundColor", "#ABCDEF");
        let document = skin
            .merged_style(&StyleSignature::of(&[SName::Root, SName::Document]))
            .unwrap();
        assert_eq!(
            document.value(PName::BackGroundColor).as_string(),
            "#ABCDEF"
        );
        assert_eq!(skin.value("backgroundcolor").as_deref(), Some("#ABCDEF"));
    }

    #[test]
    fn the_background_comes_from_the_skinparam_or_the_document() {
        let background = |value: Option<&str>| {
            let mut skin = SkinParam::default();
            if let Some(value) = value {
                skin.set_param("backgroundColor", value);
            }
            skin.get_background_color()
        };
        assert_eq!(background(None), HColor::WHITE);
        assert_eq!(
            background(Some("#ABCDEF")),
            HColor::parse("#ABCDEF").unwrap().unwrap()
        );
        assert_eq!(background(Some("Transparent")), HColor::NONE);
        assert_eq!(background(Some("nosuchcolor")), HColor::WHITE);
    }

    #[test]
    fn box_padding_is_a_plain_decimal() {
        let padding = |value: Option<&str>| {
            let mut skin = SkinParam::default();
            if let Some(value) = value {
                skin.set_param("BoxPadding", value);
            }
            skin.box_padding()
        };
        assert_eq!(padding(None), 0.0);
        assert_eq!(padding(Some("12.5")), 12.5);
        assert_eq!(padding(Some("-3")), 0.0);
        assert_eq!(padding(Some("1e3")), 0.0);
    }

    #[test]
    fn the_strictuml_style_loads_its_skin() {
        let mut skin = SkinParam::default();
        skin.set_param("style", "strictuml");
        let element = StyleSignature::of(&[SName::Root, SName::Element]);
        let shadowing = skin.merged_style(&element).unwrap();
        assert_eq!(shadowing.value(PName::Shadowing).as_string(), "0.0");
    }

    #[test]
    fn looked_up_values_are_remembered_like_plantuml() {
        let mut skin = SkinParam::default();
        assert_eq!(skin.value("handwritten"), None);
        skin.set_param("handwritten", "true");
        assert_eq!(skin.value("handwritten"), None);
    }
}
