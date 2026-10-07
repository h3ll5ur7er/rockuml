//! Teoz, PlantUML's sequence diagram layout engine: tiles stacked down the page, positioned by
//! constraints that the `real` solver resolves.

mod communication;
mod components;
mod key;
mod life_event;
mod living_space;
mod note_tiles;
mod playing_space;
mod self_tile;
mod tile;
mod y_gauge;

use std::cell::OnceCell;
use std::rc::Rc;

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

/// One page of a sequence diagram (PlantUML's `SequenceDiagramFileMakerTeoz`).
pub(super) struct SequenceDiagramFileMakerTeoz<'a> {
    body: PlayingSpaceWithParticipants<'a>,
    min1: Real,
}

impl<'a> SequenceDiagramFileMakerTeoz<'a> {
    pub(super) fn new(
        diagram: &'a SequenceDiagram,
        string_bounder: Rc<dyn StringBounder>,
        page: usize,
    ) -> Result<Self, NotYetPorted> {
        if page > 0 {
            return Err(NotYetPorted("newpage"));
        }
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
        if diagram
            .participants()
            .iter()
            .any(|&p| diagram.englober_of(p).is_some())
        {
            return Err(NotYetPorted("boxes around participants"));
        }
        let arguments = Rc::new(TileArguments {
            diagram,
            string_bounder: string_bounder.clone(),
            living_spaces,
            x_origin: x_origin.clone(),
            y_origin: y_origin.clone(),
            borders: OnceCell::new(),
        });
        let main_tile = PlayingSpace::new(arguments.clone(), None, diagram.is_show_footbox())?;
        arguments
            .living_spaces
            .add_constraints(string_bounder.as_ref());
        main_tile.add_constraints();
        x_origin.compile_now();
        y_origin.compile_now();
        let _ = arguments
            .borders
            .set((main_tile.min().clone(), main_tile.max().clone()));
        let body = PlayingSpaceWithParticipants::new(main_tile);
        let min1 = body.min_x().clone();
        Ok(Self { body, min1 })
    }
}

impl TextBlock for SequenceDiagramFileMakerTeoz<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.body.calculate_dimension().delta(10.0, 10.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.translated(5.0 - self.min1.current_value(), 5.0);
        self.body.draw_u(&ug);
    }
}
