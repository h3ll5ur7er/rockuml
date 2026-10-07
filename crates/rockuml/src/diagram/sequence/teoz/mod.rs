//! Teoz, PlantUML's sequence diagram layout engine: tiles stacked down the page, positioned by
//! constraints that the `real` solver resolves.

mod communication;
mod communication_exo;
mod components;
mod dolls;
mod key;
mod life_event;
mod living_space;
mod note_tiles;
mod playing_space;
mod self_tile;
mod span_tiles;
mod tile;
mod y_gauge;

use std::cell::OnceCell;
use std::rc::Rc;

use dolls::Dolls;
use living_space::{LivingSpace, LivingSpaces};
use playing_space::{PlayingSpace, PlayingSpaceWithParticipants};
use tile::TileArguments;

use super::SequenceDiagram;
use crate::diagram::NotYetPorted;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::Context2D;

/// One page of a sequence diagram (PlantUML's `SequenceDiagramFileMakerTeoz`).
pub(super) struct SequenceDiagramFileMakerTeoz<'a> {
    body: PlayingSpaceWithParticipants<'a>,
    min1: Real,
    dolls: Dolls<'a>,
    /// Room above the participants for the box titles, and below them for the boxes' bottoms.
    height_englober1: f64,
    height_englober2: f64,
}

impl<'a> SequenceDiagramFileMakerTeoz<'a> {
    pub(super) fn new(
        diagram: &'a SequenceDiagram,
        string_bounder: Rc<dyn StringBounder>,
        page: usize,
    ) -> Result<Self, NotYetPorted> {
        let x_origin = Real::origin();
        let mut x_current = x_origin.add_at_least(0.0);
        let y_origin = Real::origin();
        let mut living_spaces = LivingSpaces::new();
        for &participant in diagram.participants() {
            let living_space = LivingSpace::new(
                diagram,
                participant,
                diagram.englober_of(participant),
                x_current,
            );
            x_current = living_space
                .pos_d(string_bounder.as_ref())
                .add_at_least(0.0);
            living_spaces.push(living_space);
        }
        let arguments = Rc::new(TileArguments {
            diagram,
            string_bounder: string_bounder.clone(),
            living_spaces,
            x_origin: x_origin.clone(),
            y_origin: y_origin.clone(),
            borders: OnceCell::new(),
        });
        let dolls = Dolls::new(&arguments);
        // Before the playing space, which freezes the margins the box titles need.
        dolls.add_constraints();
        let main_tile =
            PlayingSpace::new(arguments.clone(), dolls.extent(), diagram.is_show_footbox())?;
        arguments
            .living_spaces
            .add_constraints(string_bounder.as_ref());
        main_tile.add_constraints();
        x_origin.compile_now();
        y_origin.compile_now();
        let _ = arguments
            .borders
            .set((main_tile.min().clone(), main_tile.max().clone()));
        let body = PlayingSpaceWithParticipants::new(main_tile, page);
        let min1 = body.min_x().clone();
        let height_englober1 = dolls.offset_for_englobers();
        let height_englober2 = if height_englober1 == 0.0 { 0.0 } else { 10.0 };
        Ok(Self {
            body,
            min1,
            dolls,
            height_englober1,
            height_englober2,
        })
    }
}

impl TextBlock for SequenceDiagramFileMakerTeoz<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        let dimension = self.body.calculate_dimension();
        let height = dimension.height + self.height_englober1 + self.height_englober2;
        XDimension2D::new(dimension.width + 10.0, height + 10.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.translated(5.0 - self.min1.current_value(), 5.0);
        let body_height = self.body.calculate_dimension().height;
        self.dolls.draw_englobers(
            &ug,
            body_height + self.height_englober1 + self.height_englober2 / 2.0,
            Context2D {
                is_background: true,
            },
        );
        self.body.draw_u(&ug.translated(0.0, self.height_englober1));
    }
}
