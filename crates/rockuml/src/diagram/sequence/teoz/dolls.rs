//! Boxes around participants (`box ... end box`), possibly nested (PlantUML's `Doll` and `Dolls`).

use std::rc::Rc;

use super::components;
use super::living_space::LivingSpace;
use super::tile::TileArguments;
use crate::diagram::sequence::model::ParticipantId;
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, Component, Context2D};

/// One box and the participants inside it, nested boxes' included.
struct Doll<'a> {
    arguments: Rc<TileArguments<'a>>,
    englober: usize,
    participants: Vec<ParticipantId>,
}

impl<'a> Doll<'a> {
    fn first_living_space(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.participants[0])
    }

    fn last_living_space(&self) -> &LivingSpace<'a> {
        let last = *self.participants.last().expect("a box has participants");
        self.arguments.living_space(last)
    }

    fn pos_a(&self) -> Real {
        self.first_living_space().pos_a()
    }

    fn pos_e(&self) -> Real {
        self.last_living_space()
            .pos_e(self.arguments.string_bounder())
    }

    /// Where the participant before the box ends, or the origin.
    fn pos_aa(&self) -> Real {
        let first = self.participants[0];
        match self.arguments.living_spaces.previous(first) {
            Some(previous) => previous.pos_d(self.arguments.string_bounder()),
            None => self.arguments.x_origin.clone(),
        }
    }

    fn title_width(&self) -> f64 {
        components::englober(self.arguments.diagram, self.englober)
            .preferred_width(self.arguments.string_bounder())
    }

    /// Widens the first and last participants' margins so that the title fits.
    fn add_internal_constraints(&self) {
        let string_bounder = self.arguments.string_bounder();
        let title_width = self.title_width();
        let x1 = self.first_living_space().pos_b().current_value();
        let x2 = self
            .last_living_space()
            .pos_d(string_bounder)
            .current_value();
        let actual_width = x2 - x1;
        let margin_x = (title_width + 10.0 - actual_width) / 2.0;
        if margin_x > 0.0 {
            self.first_living_space().ensure_margin_before(margin_x);
            self.last_living_space().ensure_margin_after(margin_x);
        }
    }

    /// Keeps the box clear of the participant before it, whatever its margin grows to.
    fn add_neighbor_constraint_before(&self) {
        let padding = self.arguments.diagram.skin().box_padding();
        self.first_living_space()
            .pos_a_live()
            .ensure_bigger_than(&self.pos_aa().add_fixed(10.0 + padding));
    }

    fn add_constraint_after(&self) {
        let string_bounder = self.arguments.string_bounder();
        let Some(next) = self
            .arguments
            .living_spaces
            .next(*self.participants.last().expect("a box has participants"))
        else {
            return;
        };
        let padding = self.arguments.diagram.skin().box_padding();
        next.pos_a_live().ensure_bigger_than(
            &self
                .last_living_space()
                .pos_e_live(string_bounder)
                .add_fixed(20.0 + padding + padding),
        );
    }

    fn draw_me(&self, ug: &UGraphic, height: f64, context: Context2D) {
        let x1 = self.pos_a().current_value() - 4.0;
        let x2 = self.pos_e().current_value() + 4.0;
        let mut ug = ug.clone();
        let mut height = height;
        let mut ancestor = self.arguments.diagram.englober(self.englober).parent;
        while let Some(current) = ancestor {
            let title_height = title_preferred_height(&self.arguments, current);
            ug = ug.translated(0.0, title_height);
            height -= title_height;
            ancestor = self.arguments.diagram.englober(current).parent;
        }
        components::englober(self.arguments.diagram, self.englober).draw_u(
            &ug.translated(x1, 1.0),
            &Area::new(x2 - x1, height),
            context,
        );
    }
}

fn title_preferred_height(arguments: &TileArguments<'_>, englober: usize) -> f64 {
    components::englober(arguments.diagram, englober).preferred_height(arguments.string_bounder())
}

/// Every box, in the order their first participants come.
pub(super) struct Dolls<'a> {
    alls: Vec<Doll<'a>>,
}

impl<'a> Dolls<'a> {
    pub(super) fn new(arguments: &Rc<TileArguments<'a>>) -> Self {
        let diagram = arguments.diagram;
        let mut alls: Vec<Doll<'a>> = Vec::new();
        for living_space in arguments.living_spaces.values() {
            let mut genealogy = Vec::new();
            let mut current = living_space.englober;
            while let Some(englober) = current {
                genealogy.push(englober);
                current = diagram.englober(englober).parent;
            }
            for &englober in genealogy.iter().rev() {
                let index = alls
                    .iter()
                    .position(|doll| doll.englober == englober)
                    .unwrap_or_else(|| {
                        alls.push(Doll {
                            arguments: arguments.clone(),
                            englober,
                            participants: Vec::new(),
                        });
                        alls.len() - 1
                    });
                alls[index].participants.push(living_space.participant);
            }
        }
        Self { alls }
    }

    /// The height of the deepest stack of titles.
    pub(super) fn offset_for_englobers(&self) -> f64 {
        let mut result: f64 = 0.0;
        for doll in &self.alls {
            let mut height = 0.0;
            let mut current = Some(doll.englober);
            while let Some(englober) = current {
                height += title_preferred_height(&doll.arguments, englober);
                current = doll.arguments.diagram.englober(englober).parent;
            }
            if height > result {
                result = height;
            }
        }
        result
    }

    /// The title margins first, then the room before and after each box: the order the solver sees them in
    /// is part of the layout.
    pub(super) fn add_constraints(&self) {
        for doll in &self.alls {
            doll.add_internal_constraints();
        }
        for doll in &self.alls {
            doll.add_neighbor_constraint_before();
        }
        for doll in &self.alls {
            doll.add_constraint_after();
        }
    }

    pub(super) fn draw_englobers(&self, ug: &UGraphic, height: f64, context: Context2D) {
        for doll in &self.alls {
            doll.draw_me(ug, height, context);
        }
    }

    /// How far the boxes reach on both sides, if there are any.
    pub(super) fn extent(&self) -> Option<(Real, Real)> {
        if self.alls.is_empty() {
            return None;
        }
        let min = self.alls.iter().map(Doll::pos_a).collect();
        let max = self
            .alls
            .iter()
            .map(|doll| doll.pos_e().add_fixed(10.0))
            .collect();
        Some((Real::min(min), Real::max(max)))
    }
}
