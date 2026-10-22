//! Style sheets on one line, from files, and skins (PlantUML's `CommandStyleSingleLineCSS`,
//! `CommandStyleImport` and `CommandSkin`).

use std::sync::LazyLock;

use super::style_error;
use crate::command::{Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::diagram::titled::TitledDiagram;
use crate::pattern::{RegexResult, RegexTree};
use crate::style::{PName, StyleBuilder};
use crate::text::LineLocation;

/// PlantUML's `CommandStyleSingleLineCSS`: `<style>class { BackGroundColor pink }</style>`.
pub(super) fn single_line_css<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    command(single_line_css_pattern(), |diagram, arg| {
        let line = arg.get("STYLE", 0).unwrap_or_default();
        diagram
            .titled()
            .skin
            .apply_style_sheet(&[line])
            .map_err(style_error)
    })
}

/// PlantUML's `CommandStyleImport`: `<style file=dark.css>`, a style sheet read from a file or a URL, or
/// else one of the skins PlantUML embeds.
pub(super) fn import<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    command(import_pattern(), |diagram, arg| {
        let path = arg.get("PATH", 0).unwrap_or_default();
        let is_url = path.starts_with("http://") || path.starts_with("https://");
        let skin = &diagram.titled().skin;
        let text = skin
            .file(path)
            .or_else(|| (!is_url).then(|| embedded_skin(path)).flatten())
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            .ok_or_else(|| cannot_read(path))?;
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return Err(cannot_read(path));
        }
        diagram
            .titled()
            .skin
            .apply_style_sheet(&lines)
            .map_err(style_error)
    })
}

/// PlantUML's `CommandSkin`: `skin rose`, the style sheet `rose.skin` to start from, a file or one PlantUML
/// embeds (`TitledDiagram.loadSkin`).
pub(super) fn skin<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    command(skin_pattern(), |diagram, arg| {
        let name = arg.get("SKIN", 0).unwrap_or_default();
        let file_name = skin_file_name(name);
        let skin = &diagram.titled().skin;
        let text = skin
            .file(&file_name)
            .or_else(|| embedded_skin(&file_name))
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            .ok_or_else(|| CommandError::new(format!("Cannot find style {name}")))?;
        let lines: Vec<&str> = text.lines().collect();
        let cannot_parse =
            |message: &str| CommandError::new(format!("Cannot parse style {name}: {message}"));
        let builder = StyleBuilder::from_style_sheet(&lines)
            .map_err(|error| cannot_parse(&style_error(error).message))?;
        if builder.is_empty() {
            return Err(cannot_parse(&format!("No style found in {file_name}")));
        }
        // The skin replaces the whole style sheet, which a fragment cannot do.
        let missing = builder.missing_root_properties();
        if !missing.is_empty() {
            let names: Vec<&str> = missing.into_iter().map(PName::java_name).collect();
            return Err(CommandError::new(format!(
                "Incomplete style {name}: root does not define [{}]",
                names.join(", ")
            )));
        }
        diagram.titled().skin.set_default_skin(builder);
        Ok(())
    })
}

/// The style sheet or skin file a line of the source loads, so that it can be read before the diagram is
/// built.
pub(crate) fn style_file_named(line: &str) -> Option<String> {
    static IMPORT: LazyLock<RegexTree> = LazyLock::new(import_pattern);
    static SKIN: LazyLock<RegexTree> = LazyLock::new(skin_pattern);
    let line = line.trim();
    if let Some(arg) = IMPORT.matcher(line) {
        return arg.get("PATH", 0).map(str::to_owned);
    }
    SKIN.matcher(line)
        .and_then(|arg| arg.get("SKIN", 0).map(skin_file_name))
}

fn command<D: TitledDiagram + 'static>(
    pattern: RegexTree,
    apply: fn(&mut D, &RegexResult) -> CommandResult,
) -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        pattern,
        move |diagram: &mut D, _: &LineLocation, arg: &RegexResult| apply(diagram, arg),
    )))
}

fn embedded_skin(file_name: &str) -> Option<&'static [u8]> {
    crate::assets::get(&format!("skin/{file_name}"))
}

fn skin_file_name(name: &str) -> String {
    format!("{name}.skin")
}

fn cannot_read(path: &str) -> CommandError {
    CommandError::new(format!("Cannot read: {path}"))
}

fn single_line_css_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(r"\<style\>"),
        RegexTree::named(1, "STYLE", r"([-#.\w%s;:{}]+)"),
        RegexTree::leaf(r"\</style\>"),
        RegexTree::end(),
    ])
}

fn import_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(r"\<style"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\w+"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"="),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"[%q%g]?"),
        RegexTree::named(1, "PATH", r"([^%q%g]*)"),
        RegexTree::leaf(r"[%q%g]?"),
        RegexTree::leaf(r"\>"),
        RegexTree::end(),
    ])
}

fn skin_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(r"skin"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "SKIN", r"([\w.]+)"),
        RegexTree::end(),
    ])
}

#[cfg(test)]
mod tests {
    use crate::diagram::DiagramError;
    use crate::diagram::test_support::error_of;

    fn message_of(line: &str) -> Option<String> {
        error_of(&format!(
            "@startuml
{line}
A -> B
@enduml"
        ))
        .map(|DiagramError { message, .. }| message)
    }

    fn sequence_error(message: &str) -> String {
        format!("{message} (Assumed diagram type: sequence)")
    }

    #[test]
    fn skins_are_embedded_or_read() {
        assert_eq!(message_of("skin rose"), None);
        assert_eq!(
            message_of("skin nosuch"),
            Some(sequence_error("Cannot find style nosuch"))
        );
    }

    #[test]
    fn a_skin_must_style_the_root() {
        let missing = "FontName, FontSize, FontStyle, FontColor, LineColor, LineThickness, \
                       BackGroundColor, HorizontalAlignment";
        assert_eq!(
            message_of("skin strictuml"),
            Some(sequence_error(&format!(
                "Incomplete style strictuml: root does not define [{missing}]"
            )))
        );
    }

    #[test]
    fn style_files_that_cannot_be_read_are_errors() {
        assert_eq!(
            message_of("<style file=missing.css>"),
            Some(sequence_error("Cannot read: missing.css"))
        );
    }
}
