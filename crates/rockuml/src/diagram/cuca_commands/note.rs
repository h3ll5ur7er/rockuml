//! Notes on their own, on entities and on links (PlantUML's `command.note` package). None is ported yet:
//! they only recognise their lines.

use crate::command::unported::{self, NotPortedCommands};
use crate::command::{Command, ParserPass};
use crate::klimt::url::Url;
use crate::pattern::{RegexTree, plantuml_regex};
use crate::{color, stereo};

/// `note "text" as N1` (`CommandFactoryNote.createSingleLine`).
pub(in crate::diagram) fn note<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandFactoryNote",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("[%g]"),
            RegexTree::named(1, "DISPLAY", "([^%g]+)"),
            RegexTree::leaf("[%g]"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", "([%pLN_.]+)"),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS"),
            stereo::optional_pattern("STEREO"),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}

/// `note as N1` ... `end note` (`CommandFactoryNote.createMultiLine`).
pub(in crate::diagram) fn note_multi_line<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    Box::new(unported::multi_line(
        "CommandFactoryNote",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("note"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", "([%pLN_.]+)"),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS"),
            stereo::optional_pattern("STEREO"),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
        &plantuml_regex("^[%s]*end[%s]?note$"),
    ))
}

/// What notes on entities start with: the position and the entity named by `code`, or the last entity.
fn note_on_entity_head(code: RegexTree) -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::leaf("note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", "(right|left|top|bottom)"),
        RegexTree::or(vec![
            RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf("of"),
                RegexTree::spaces_one_or_more(),
                code,
            ]),
            RegexTree::leaf(""),
        ]),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS1"),
        stereo::optional_pattern("STEREO"),
        stereo::tags_pattern("TAGS2"),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::spaces_zero_or_more(),
        Url::optional_pattern(),
    ]
}

/// `note left of A : text`, run in `pass` (`CommandFactoryNoteOnEntity.createSingleLine`).
pub(in crate::diagram) fn note_on_entity<D: NotPortedCommands + 'static>(
    code: RegexTree,
    pass: ParserPass,
) -> Box<dyn Command<D>> {
    let mut pattern = note_on_entity_head(code);
    pattern.extend([
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(":"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "NOTE", "(.*)"),
        RegexTree::end(),
    ]);
    unported::single_line("CommandFactoryNoteOnEntity", RegexTree::concat(pattern))
        .in_passes(pass.alone())
        .boxed()
}

/// `note left of A` ... `end note`, or `note left of A {` ... `}` `with_bracket`
/// (`CommandFactoryNoteOnEntity.createMultiLine`).
pub(in crate::diagram) fn note_on_entity_multi_line<D: NotPortedCommands + 'static>(
    code: RegexTree,
    pass: ParserPass,
    with_bracket: bool,
) -> Box<dyn Command<D>> {
    let mut pattern = note_on_entity_head(code);
    let end = if with_bracket {
        pattern.extend([RegexTree::spaces_zero_or_more(), RegexTree::leaf(r"\{")]);
        r"^(\})$"
    } else {
        "^[%s]*(end[%s]?note)$"
    };
    pattern.push(RegexTree::end());
    Box::new(
        unported::multi_line(
            "CommandFactoryNoteOnEntity",
            RegexTree::concat(pattern),
            &plantuml_regex(end),
        )
        .in_passes(pass.alone()),
    )
}

/// What notes on links start with.
fn note_on_link_head() -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::leaf("note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", "(right|left|top|bottom)?"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::counted(1, "(on|of)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("link"),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
    ]
}

/// `note on link : text`, run in `pass` (`CommandFactoryNoteOnLink.createSingleLine`).
pub(in crate::diagram) fn note_on_link<D: NotPortedCommands + 'static>(
    pass: ParserPass,
) -> Box<dyn Command<D>> {
    let mut pattern = note_on_link_head();
    pattern.extend([
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(":"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "NOTE", "(.*)"),
        RegexTree::end(),
    ]);
    unported::single_line("CommandFactoryNoteOnLink", RegexTree::concat(pattern))
        .in_passes(pass.alone())
        .boxed()
}

/// `note on link` ... `end note` (`CommandFactoryNoteOnLink.createMultiLine`).
pub(in crate::diagram) fn note_on_link_multi_line<D: NotPortedCommands + 'static>(
    pass: ParserPass,
) -> Box<dyn Command<D>> {
    let mut pattern = note_on_link_head();
    pattern.push(RegexTree::end());
    Box::new(
        unported::multi_line(
            "CommandFactoryNoteOnLink",
            RegexTree::concat(pattern),
            &plantuml_regex("^end[%s]?note$"),
        )
        .in_passes(pass.alone()),
    )
}
