//! `skinparam` blocks and `!pragma` (PlantUML's `CommandSkinParamMultilines`, `SkinLoader` and
//! `CommandPragma`).

use std::sync::LazyLock;

use regex::Regex;

use crate::command::{BlocLines, Command, CommandControl, CommandResult};
use crate::diagram::titled::TitledDiagram;
use crate::pattern::plantuml_regex;

/// `skinparam sequence {` ... `}`: parameters named by their nested blocks, like `sequenceArrowColor`.
pub(super) struct SkinParamBlock;

static START: LazyLock<Regex> = LazyLock::new(|| {
    plantuml_regex(r"^skinparam[%s]*(?:[%s]+([\w.]*(?:\<\<.*\>\>)?[\w.]*))?[%s]*\{$")
});

/// A parameter (`name value`), a nested block (`name {`) or its end.
static PARAMETER: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex(r"^([\w.]*(?:\<\<.*\>\>)?[\w.]*)[%s]+(?:(\{)|(.*))$|^\}?$"));

static COMMENT: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex(r"^[%s]*([%q].*||/[%q].*[%q]/[%s]*)$"));

impl<D: TitledDiagram> Command<D> for SkinParamBlock {
    /// Complete once the brackets balance.
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let Some(first) = lines.first() else {
            return CommandControl::NotOk;
        };
        if !START.is_match(first.trimmed().text()) {
            return CommandControl::NotOk;
        }
        if lines.len() == 1 {
            return CommandControl::OkPartial;
        }
        let mut level = 1;
        for line in lines.iter().skip(1) {
            let text = line.trimmed();
            let text = text.text();
            if !COMMENT.is_match(text) && !PARAMETER.is_match(text) {
                return CommandControl::NotOk;
            }
            if text.ends_with('{') {
                level += 1;
            }
            if text.ends_with('}') {
                level -= 1;
            }
            if level < 0 {
                return CommandControl::NotOk;
            }
        }
        if level == 0 {
            CommandControl::Ok
        } else {
            CommandControl::OkPartial
        }
    }

    fn execute(&self, diagram: &mut D, lines: BlocLines) -> CommandResult {
        let first = lines.first().expect("a block has its first line").trimmed();
        let mut context: Vec<String> = START
            .captures(first.text())
            .and_then(|captures| captures.get(1))
            .map(|group| vec![group.as_str().to_owned()])
            .unwrap_or_default();
        let body = lines.sub_extract(1, 1).trimmed();
        for line in body.iter().filter(|line| !line.text().is_empty()) {
            if line.text() == "}" {
                context.pop();
                continue;
            }
            let captures = PARAMETER
                .captures(line.text())
                .expect("checked when the block was recognised");
            let name = captures.get(1).map_or("", |name| name.as_str());
            if captures.get(2).is_some() {
                context.push(name.to_owned());
            } else if let Some(value) = captures.get(3) {
                let key = format!("{}{name}", context.concat());
                diagram.titled().skin.set_param(&key, value.as_str());
            }
        }
        Ok(())
    }
}
