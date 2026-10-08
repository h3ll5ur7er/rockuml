//! Mind maps, `@startmindmap` (PlantUML's `mindmap` package).

mod commands;
mod finger;
mod idea;
mod tetris;

use std::rc::Rc;

use finger::FingerImpl;
pub(super) use idea::IdeaShape;
use idea::{Branch, IdeaSpec};

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, ParserPass};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::{Rankdir, SkinParam};
use crate::stereo::Stereotype;
use crate::style::SName;

pub(super) struct MindMapDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    /// One per root, drawn one below the other.
    mindmaps: Vec<MindMap>,
    /// Whether ideas grow to the right (or down) unless their line says otherwise.
    default_direction: bool,
    /// The marker of the first org-mode line, which the indentation of the others is measured against.
    first: Option<String>,
}

/// Reads mind maps (PlantUML's `MindMapDiagramFactory`).
pub(super) struct MindMapDiagramFactory;

impl CommandFactory for MindMapDiagramFactory {
    type Diagram = MindMapDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::MindMap;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> MindMapDiagram {
        let mut titled = Titled::new(SName::MindmapDiagram, "MINDMAP", source);
        titled.skin.set_rankdir(Rankdir::LeftToRight);
        MindMapDiagram {
            source: source.clone(),
            titled,
            mindmaps: vec![MindMap::default()],
            default_direction: true,
            first: None,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<MindMapDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend([
            super::cuca_commands::rank_dir(),
            commands::orgmode_multiline(),
            commands::orgmode(),
            commands::root(),
            commands::plus(),
            commands::direction(),
        ]);
        commands
    }
}

/// What an idea's line says about it.
struct NewIdea {
    stereotype: Option<Stereotype>,
    back_color: Option<HColor>,
    level: i32,
    label: Display,
    shape: IdeaShape,
    direction: bool,
}

impl MindMapDiagram {
    fn set_default_direction(&mut self, right_or_down: bool) {
        self.default_direction = right_or_down;
    }

    /// An idea in the default direction, its label's ending stereotype taken off as its own.
    fn add_idea(
        &mut self,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
    ) -> CommandResult {
        let direction = self.default_direction;
        self.add_idea_in(back_color, level, label, shape, direction)
    }

    fn add_idea_in(
        &mut self,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
        direction: bool,
    ) -> CommandResult {
        let stereotype = label.get_ending_stereotype();
        let label = if stereotype.is_some() {
            label.remove_ending_stereotype()
        } else {
            label
        };
        self.add(&NewIdea {
            stereotype,
            back_color,
            level,
            label,
            shape,
            direction,
        })
    }

    fn add_idea_stereotyped(
        &mut self,
        stereotype: Stereotype,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
    ) -> CommandResult {
        let direction = self.default_direction;
        self.add(&NewIdea {
            stereotype: Some(stereotype),
            back_color,
            level,
            label,
            shape,
            direction,
        })
    }

    fn add(&mut self, idea: &NewIdea) -> CommandResult {
        if self.last().is_full(idea.level) {
            self.mindmaps.push(MindMap::default());
        }
        let style_builder = self.titled.skin.current_style_builder();
        self.mindmaps
            .last_mut()
            .expect("there is a mind map")
            .add_idea_internal(&style_builder, idea)
    }

    fn last(&self) -> &MindMap {
        self.mindmaps.last().expect("there is a mind map")
    }

    /// The depth of an org-mode line, whose markers may be indented with spaces or tabs (`getSmartLevel`);
    /// `None` for an indentation PlantUML cannot make sense of.
    fn get_smart_level(&mut self, marker: &str) -> Option<i32> {
        let first = self.first.get_or_insert_with(|| marker.to_owned()).clone();
        let mut marker = marker.to_owned();
        if marker.ends_with("**") {
            crate::java::trim(&marker.replace('\t', " ")).clone_into(&mut marker);
        }
        let marker = marker.replace('\t', " ");
        let length = |text: &str| i32::try_from(text.encode_utf16().count()).ok();
        if !marker.contains(' ') {
            return Some(length(&marker)? - 1);
        }
        if marker.ends_with(&first) {
            return Some(length(&marker)? - length(&first)?);
        }
        if crate::java::trim(&marker).encode_utf16().count() == 1 {
            return Some(length(&marker)? - 1);
        }
        if marker.starts_with(&first) {
            return Some(length(&marker)? - length(&first)?);
        }
        None
    }
}

/// One root and the ideas growing from it to either side (PlantUML's `MindMap`).
#[derive(Default)]
struct MindMap {
    regular: Branch,
    reverse: Branch,
    /// The depth step of the first idea below the root, which every other depth must be a multiple of.
    multiplier: i32,
}

impl MindMap {
    fn add_idea_internal(
        &mut self,
        style_builder: &Rc<crate::style::StyleBuilder>,
        idea: &NewIdea,
    ) -> CommandResult {
        let mut level = idea.level;
        if !self.reverse.has_root() && !self.regular.has_root() {
            level = 0;
        }
        let spec = || IdeaSpec {
            style_builder: style_builder.clone(),
            back_color: idea.back_color.clone(),
            label: idea.label.clone(),
            shape: idea.shape,
            stereotype: idea.stereotype.clone(),
        };
        if level == 0 {
            self.regular.init_root(spec());
            self.reverse.init_root(spec());
            return Ok(());
        }
        if self.multiplier == 0 {
            self.multiplier = level;
        }
        if level % self.multiplier != 0 {
            return Err(CommandError::new("Bad indentation"));
        }
        let level = usize::try_from(level / self.multiplier)
            .map_err(|_| CommandError::new("Bad indentation"))?;
        if idea.direction {
            self.regular.add(level, spec())
        } else {
            self.reverse.add(level, spec())
        }
    }

    fn is_full(&self, level: i32) -> bool {
        level == 0 && self.regular.has_root()
    }

    /// The fingers of both sides; the root's box is drawn once, by the regular side (`computeFinger`).
    fn fingers<'a>(&'a self, skin_param: &'a SkinParam) -> Fingers<'a> {
        if !self.regular.has_root() {
            return Fingers::default();
        }
        let mut reverse = self
            .reverse
            .has_children()
            .then(|| FingerImpl::build(&self.reverse, Branch::ROOT, skin_param, false));
        let regular = (reverse.is_none() || self.regular.has_children())
            .then(|| FingerImpl::build(&self.regular, Branch::ROOT, skin_param, true));
        if let (Some(reverse), Some(_)) = (&mut reverse, &regular) {
            reverse.do_not_draw_first_phalanx();
        }
        Fingers {
            regular,
            reverse,
            top_to_bottom: skin_param.get_rankdir() == Rankdir::TopToBottom,
        }
    }
}

#[derive(Default)]
struct Fingers<'a> {
    regular: Option<FingerImpl<'a>>,
    reverse: Option<FingerImpl<'a>>,
    top_to_bottom: bool,
}

impl Fingers<'_> {
    fn get_half_thickness(
        finger: Option<&FingerImpl<'_>>,
        string_bounder: &dyn StringBounder,
    ) -> f64 {
        finger.map_or(0.0, |finger| {
            finger.get_full_thickness(string_bounder) / 2.0
        })
    }

    fn get_x12(finger: Option<&FingerImpl<'_>>, string_bounder: &dyn StringBounder) -> f64 {
        finger.map_or(0.0, |finger| {
            finger.get_full_elongation(string_bounder) + finger.get_x12()
        })
    }

    fn half_thickness(&self, string_bounder: &dyn StringBounder) -> f64 {
        Self::get_half_thickness(self.regular.as_ref(), string_bounder).max(
            Self::get_half_thickness(self.reverse.as_ref(), string_bounder),
        )
    }
}

impl TextBlock for Fingers<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let y = self.half_thickness(string_bounder);
        let width = Self::get_x12(self.reverse.as_ref(), string_bounder)
            + Self::get_x12(self.regular.as_ref(), string_bounder);
        let height = y + self.half_thickness(string_bounder);
        if self.top_to_bottom {
            XDimension2D::new(height, width)
        } else {
            XDimension2D::new(width, height)
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        if self.regular.is_none() && self.reverse.is_none() {
            return;
        }
        let string_bounder = ug.string_bounder();
        let y = self.half_thickness(string_bounder);
        let x = Self::get_x12(self.reverse.as_ref(), string_bounder);
        let ug = if self.top_to_bottom {
            ug.translated(y, x)
        } else {
            ug.translated(x, y)
        };
        for finger in [&self.regular, &self.reverse].into_iter().flatten() {
            finger.draw_u(&ug);
        }
    }
}

/// Every mind map of the diagram, one below the other.
struct MindMaps<'a>(Vec<Fingers<'a>>);

impl TextBlock for MindMaps<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let (width, height) = self.0.iter().fold((0.0, 0.0), |(width, height), mindmap| {
            let dimension = mindmap.calculate_dimension(string_bounder);
            (f64::max(width, dimension.width), height + dimension.height)
        });
        XDimension2D::new(width + 10.0, height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let mut ug = ug.clone();
        for mindmap in &self.0 {
            mindmap.draw_u(&ug);
            let dimension = mindmap.calculate_dimension(ug.string_bounder());
            ug = ug.translated(0.0, dimension.height);
        }
    }
}

impl AbstractDiagram for MindMapDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for MindMapDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for MindMapDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let skin = &self.titled.skin;
        let mindmaps = MindMaps(
            self.mindmaps
                .iter()
                .map(|mindmap| mindmap.fingers(skin))
                .collect(),
        );
        Ok(self.titled.add_chrome(Box::new(mindmaps), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}
