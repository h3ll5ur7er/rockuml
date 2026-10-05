//! A diagram's `skinparam` settings and the styles they feed (PlantUML's `SkinParam`).

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::java;
use crate::pattern::{java_regex, plantuml_regex};
use crate::style::{Style, StyleBuilder, StyleParser, StyleSignature, skinparam_styles};

const DEFAULT_SKIN: &str = "plantuml.skin";

pub struct SkinParam {
    skin: String,
    /// Loaded from the skin when first needed, so that `skin` commands before that still apply.
    style_builder: OnceCell<StyleBuilder>,
    params: HashMap<String, String>,
    /// PlantUML remembers every value it looked up, even when a later `skinparam` changes it.
    looked_up: RefCell<HashMap<String, Option<String>>>,
}

impl Default for SkinParam {
    fn default() -> Self {
        Self {
            skin: DEFAULT_SKIN.to_owned(),
            style_builder: OnceCell::new(),
            params: HashMap::new(),
            looked_up: RefCell::new(HashMap::new()),
        }
    }
}

impl SkinParam {
    pub fn set_default_skin(&mut self, skin: &str) {
        skin.clone_into(&mut self.skin);
    }

    pub fn style_builder(&self) -> &StyleBuilder {
        self.style_builder.get_or_init(|| {
            StyleBuilder::load_skin(&self.skin)
                .or_else(|| StyleBuilder::load_skin(DEFAULT_SKIN))
                .expect("the default skin is embedded")
        })
    }

    fn style_builder_mut(&mut self) -> &mut StyleBuilder {
        self.style_builder();
        self.style_builder.get_mut().expect("just initialised")
    }

    pub fn merged_style(&self, element: &StyleSignature) -> Option<Style> {
        self.style_builder().merged_style(element)
    }

    pub fn mute_style(&mut self, styles: Vec<Style>) {
        self.style_builder_mut().mute(styles);
    }

    /// `skinparam key value`: remembered under the normalised key, and turned into style rules.
    pub fn set_param(&mut self, key: &str, value: &str) {
        for normalised in clean_for_key(key) {
            self.params
                .insert(normalised.clone(), java::trim(value).to_owned());
            let builder = self.style_builder_mut();
            let styles = skinparam_styles(&normalised, value, builder.counter());
            builder.mute(styles);
        }
        if key.eq_ignore_ascii_case("style") && value.eq_ignore_ascii_case("strictuml") {
            let strict =
                crate::assets::get("skin/strictuml.skin").expect("strictuml.skin is embedded");
            let text = String::from_utf8_lossy(strict);
            let lines: Vec<&str> = text.lines().collect();
            let builder = self.style_builder_mut();
            if let Ok(styles) = StyleParser::new(builder.counter()).parse(&lines) {
                builder.mute(styles);
            }
        }
    }

    pub fn value(&self, key: &str) -> Option<String> {
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
    static STEREOTYPE: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"\<\<(.*?)\>\>"));
    static STEREOTYPE_EXACT_CASE: LazyLock<Regex> =
        LazyLock::new(|| java_regex(r"\<\<(.*?)\>\>", false));

    let key = java::trim(&key.to_lowercase()).replace(['_', '.'], "");
    let key = SEQUENCE.replace_all(&key, "$1");
    let key = ARROW.replace_all(&key, "arrow");
    let key = ALIGN.replace_all(&key, "alignment");
    let without_stereotypes = STEREOTYPE_EXACT_CASE.replace_all(&key, "");
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
    use crate::style::{PName, SName, ValueReading};

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
    fn looked_up_values_are_remembered_like_plantuml() {
        let mut skin = SkinParam::default();
        assert_eq!(skin.value("handwritten"), None);
        skin.set_param("handwritten", "true");
        assert_eq!(skin.value("handwritten"), None);
    }
}
