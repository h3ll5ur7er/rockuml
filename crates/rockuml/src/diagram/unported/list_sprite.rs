//! The command of PlantUML's sprite lists (`ListSpriteDiagramFactory`).

use crate::command::Command;
use crate::command::unported::{self, NotPortedCommands};
use crate::pattern::RegexTree;

/// PlantUML's `CommandListSprite`.
pub(super) fn list_sprite<D: NotPortedCommands + 'static>() -> Box<dyn Command<D>> {
    unported::single_line(
        "CommandListSprite",
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"listsprites?"),
            RegexTree::end(),
        ]),
    )
    .boxed()
}
