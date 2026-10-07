//! The lines written into an entity's body, like a state's `State1 : text` (PlantUML's `BodierSimple`).

use crate::creole::Display;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Bodier {
    raw_body: Vec<String>,
}

impl Bodier {
    /// `\n` in the text starts another line.
    pub(crate) fn add_field_or_method(&mut self, s: &str) {
        self.raw_body
            .extend(Display::with_newlines(s).lines().iter().cloned());
    }

    pub(crate) fn get_raw_body(&self) -> &[String] {
        &self.raw_body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_of_a_field_is_a_line_of_the_body() {
        let mut bodier = Bodier::default();
        bodier.add_field_or_method("first");
        bodier.add_field_or_method(r"second\nthird");
        assert_eq!(bodier.get_raw_body(), ["first", "second", "third"]);
    }
}
