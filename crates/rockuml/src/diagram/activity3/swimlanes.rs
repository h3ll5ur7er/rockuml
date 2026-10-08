//! The swimlanes of an activity diagram, and the state its commands build the instructions with: the
//! instruction that takes the next ones, the lane they go to, the arrow leading to the next one (the model
//! half of PlantUML's `Swimlanes`, and `Swimlane`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::instruction::{InstructionId, Instructions, SwimlaneSet};
use super::link_rendering::LinkRendering;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::Display;
use crate::ftile::vcompact::{self, VCompactFactory};
use crate::ftile::{FtileFactory, TextBlockInterceptorUDrawable};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{TextBlock, UDrawable};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::UGraphicForSnake;

/// A swimlane, known by its order of declaration, as tiles, connections and layers name it; PlantUML compares
/// lanes by identity. The lane PlantUML adds after the last while drawing, ordered after all, is
/// `SwimlaneId(usize::MAX)`. What laying the lanes out sets on PlantUML's `Swimlane` (translation, width,
/// extent) is kept by the layout, keyed by these ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SwimlaneId(pub(crate) usize);

impl SwimlaneId {
    /// Whether this lane comes before all of `others`, which must not be this lane alone
    /// (`isSmallerThanAllOthers`). PlantUML fails on a set holding no lane; such a member is passed over.
    pub(crate) fn is_smaller_than_all_others(self, others: &SwimlaneSet) -> bool {
        if others.len() == 1 && others.contains(&Some(self)) {
            return false;
        }
        others.iter().flatten().all(|other| *other >= self)
    }
}

pub(crate) struct Swimlane {
    pub(crate) name: String,
    pub(crate) order: SwimlaneId,
    pub(crate) display: Display,
    pub(crate) colors: Colors,
}

pub(crate) struct Swimlanes {
    lanes: Vec<Swimlane>,
    current_swimlane: Option<SwimlaneId>,
    /// Every instruction, the root list first.
    pub(crate) instructions: Instructions,
    current_instruction: InstructionId,
    next_link_renderer: LinkRendering,
}

impl Swimlanes {
    pub(crate) fn new() -> Self {
        Self {
            lanes: Vec::new(),
            current_swimlane: None,
            instructions: Instructions::new(),
            current_instruction: Instructions::ROOT,
            next_link_renderer: LinkRendering::none(),
        }
    }

    /// Makes the lane `name` current, declaring it first if needed, and colours or labels it.
    pub(crate) fn swimlane(&mut self, name: &str, color: Option<HColor>, label: Option<Display>) {
        let id = self.get_or_create(name);
        self.current_swimlane = Some(id);
        let lane = &mut self.lanes[id.0];
        lane.colors = lane.colors.with(ColorType::Back, color);
        if let Some(label) = label {
            lane.display = label;
        }
    }

    fn get_or_create(&mut self, name: &str) -> SwimlaneId {
        if let Some(lane) = self.lanes.iter().find(|lane| lane.name == name) {
            return lane.order;
        }
        let order = SwimlaneId(self.lanes.len());
        self.lanes.push(Swimlane {
            name: name.to_owned(),
            order,
            display: Display::with_newlines(name),
            colors: Colors::default(),
        });
        order
    }

    /// The lanes in their order of declaration.
    pub(crate) fn swimlanes(&self) -> &[Swimlane] {
        &self.lanes
    }

    pub(crate) fn lane(&self, id: SwimlaneId) -> &Swimlane {
        &self.lanes[id.0]
    }

    pub(crate) fn get_current(&self) -> InstructionId {
        self.current_instruction
    }

    pub(crate) fn set_current(&mut self, current: InstructionId) {
        self.current_instruction = current;
    }

    pub(crate) fn next_link_renderer(&self) -> &LinkRendering {
        &self.next_link_renderer
    }

    pub(crate) fn set_next_link_renderer(&mut self, link: LinkRendering) {
        self.next_link_renderer = link;
    }

    pub(crate) fn get_current_swimlane(&self) -> Option<SwimlaneId> {
        self.current_swimlane
    }
}

/// The drawing half of PlantUML's `Swimlanes`: the tiles built from the instructions, drawn in the lanes.
/// Diagrams with more than one lane are drawn by `drawWhenSwimlanes`, after `computeSizeInternal` laid the
/// lanes out (track E2); the diagram refuses them until then.
pub(super) struct SwimlanesDrawing<'a> {
    swimlanes: &'a Swimlanes,
    skin_param: Rc<SkinParam>,
    /// `!pragma useVerticalIf on`.
    use_vertical_if: bool,
    string_bounder: Rc<dyn StringBounder>,
    cached_min_max: OnceCell<MinMax>,
}

impl<'a> SwimlanesDrawing<'a> {
    pub(super) fn new(
        swimlanes: &'a Swimlanes,
        skin_param: Rc<SkinParam>,
        use_vertical_if: bool,
        string_bounder: Rc<dyn StringBounder>,
    ) -> Self {
        Self {
            swimlanes,
            skin_param,
            use_vertical_if,
            string_bounder,
            cached_min_max: OnceCell::new(),
        }
    }

    fn get_ftile_factory(&self) -> Box<dyn FtileFactory> {
        vcompact::delegator_chain(
            Box::new(VCompactFactory::new(
                self.skin_param.clone(),
                self.string_bounder.clone(),
            )),
            self.use_vertical_if,
        )
    }

    fn get_min_max(&self) -> MinMax {
        *self
            .cached_min_max
            .get_or_init(|| LimitFinder::min_max_of(self, self.string_bounder.clone()))
    }
}

impl TextBlock for SwimlanesDrawing<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.get_min_max().dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let factory = self.get_ftile_factory();
        let full = self
            .swimlanes
            .instructions
            .create_ftile(Instructions::ROOT, factory.as_ref());
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Goto,
        ])
        .get_merged_style(&self.skin_param.current_style_builder());
        let goto_color = style.value(PName::LineColor).as_color();
        let ug = UGraphicForSnake::create(ug.clone());
        TextBlockInterceptorUDrawable::new(full, goto_color, false).draw_u(&ug);
        ug.flush_ug();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lane_is_smaller_than_later_lanes_but_not_than_itself_alone() {
        let lanes = |ids: &[usize]| ids.iter().map(|&id| Some(SwimlaneId(id))).collect();
        assert!(SwimlaneId(1).is_smaller_than_all_others(&lanes(&[1, 2])));
        assert!(SwimlaneId(1).is_smaller_than_all_others(&lanes(&[2, 3])));
        assert!(!SwimlaneId(1).is_smaller_than_all_others(&lanes(&[1])));
        assert!(!SwimlaneId(2).is_smaller_than_all_others(&lanes(&[1, 2])));
        assert!(SwimlaneId(2).is_smaller_than_all_others(&SwimlaneSet::new()));
    }
}
