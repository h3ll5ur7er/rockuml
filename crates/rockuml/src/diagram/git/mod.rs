//! Commit graphs drawn from `git log --graph` output, `@startgit` (PlantUML's `gitlog` package and
//! `GitDiagramFactory`). Every line between the start and the end is part of the drawing: lines without a
//! `*` hold no commit, so `title`, `skinparam` and `<style>` lines are ignored.

mod git_text_area;
mod gnode;
mod smetana_for_git;
#[cfg(test)]
mod tests;

use std::rc::Rc;

use git_text_area::GitTextArea;
use gnode::GNodeBuilder;
use smetana_for_git::SmetanaForGit;

use super::titled::Titled;
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::style::SName;
use crate::text::StringLocated;

/// A `@startgit` diagram (PlantUML's `GitDiagramFactory` and `GitDiagram`).
pub(super) fn create(source: UmlSource) -> Result<Box<dyn Diagram>, NotYetPorted> {
    let lines = source.lines();
    let drawing = lines
        .get(1..lines.len().saturating_sub(1))
        .unwrap_or_default();
    let text_area = GitTextArea::new(drawing.iter().map(StringLocated::text));
    let gnodes = text_area
        .get_all_commits()
        .and_then(|commits| GNodeBuilder::new(&commits))
        .map_err(|_| NotYetPorted("the crash report of a git graph PlantUML cannot follow"))?;
    let source = Rc::new(source);
    Ok(Box::new(GitDiagram {
        titled: Titled::new(SName::GitDiagram, "GIT", &source),
        source,
        gnodes,
    }))
}

struct GitDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    gnodes: GNodeBuilder,
}

impl Diagram for GitDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let drawing = SmetanaForGit::new(&self.titled.skin, &self.gnodes, string_bounder.as_ref());
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}
