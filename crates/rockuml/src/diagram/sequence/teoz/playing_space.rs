//! The tiles of the whole diagram, and the diagram with its participants' heads and lifelines around them
//! (PlantUML's `PlayingSpace` and `PlayingSpaceWithParticipants`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::living_space::VerticalAlignment;
use super::tile::{Tile, TileArguments, build_several};
use super::y_gauge::YGauge;
use crate::diagram::NotYetPorted;
use crate::klimt::clip::UClip;
use crate::klimt::geom::XDimension2D;
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::Context2D;

/// How far below the heads the first tile starts.
const STARTING_Y: f64 = 8.0;

pub(super) struct PlayingSpace<'a> {
    arguments: Rc<TileArguments<'a>>,
    tiles: Vec<Box<dyn Tile<'a> + 'a>>,
    min: Real,
    max: Real,
    show_footbox: bool,
}

impl<'a> PlayingSpace<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        dolls_extent: Option<(Real, Real)>,
        show_footbox: bool,
    ) -> Result<Self, NotYetPorted> {
        let mut min = vec![arguments.x_origin.clone()];
        let mut max = vec![arguments.x_origin.clone()];
        if let Some((dolls_min, dolls_max)) = dolls_extent {
            min.push(dolls_min);
            max.push(dolls_max);
        }
        let current_y = YGauge::create(&arguments.y_origin.add_fixed(STARTING_Y), 0.0);
        let mut events = (0..arguments.diagram.events().len()).peekable();
        let tiles = build_several(&arguments, &mut events, current_y)?;
        for tile in &tiles {
            min.push(tile.min_x());
            max.push(tile.max_x());
        }
        let string_bounder = arguments.string_bounder();
        for living_space in arguments.living_spaces.values() {
            max.push(living_space.pos_d(string_bounder));
            max.push(living_space.pos_c2(string_bounder));
        }
        Ok(Self {
            min: Real::min(min),
            max: Real::max(max),
            tiles,
            arguments,
            show_footbox,
        })
    }

    pub(super) fn add_constraints(&self) {
        for tile in &self.tiles {
            tile.add_constraints();
        }
    }

    pub(super) fn min(&self) -> &Real {
        &self.min
    }

    pub(super) fn max(&self) -> &Real {
        &self.max
    }

    /// Draws every tile, after telling them their vertical positions are known; returns where the last ends.
    fn draw_internal(&self, ug: &UGraphic, context: Context2D) -> f64 {
        for tile in &self.tiles {
            tile.on_gauge_resolved();
        }
        for tile in &self.tiles {
            tile.draw_u(ug, context);
        }
        self.tiles
            .last()
            .map_or(STARTING_Y, |tile| tile.y_gauge().max.current_value())
    }

    pub(super) fn draw_background(&self, ug: &UGraphic) {
        self.draw_internal(
            ug,
            Context2D {
                is_background: true,
            },
        );
    }

    pub(super) fn draw_foreground(&self, ug: &UGraphic) {
        self.draw_internal(
            ug,
            Context2D {
                is_background: false,
            },
        );
    }

    /// The height of everything drawn, found by drawing it.
    /// Where each page break starts.
    fn newpage_tops(&self) -> Vec<(f64, f64)> {
        self.tiles
            .iter()
            .filter_map(|tile| tile.as_newpage())
            .map(|newpage| {
                (
                    newpage.y_gauge().min.current_value(),
                    newpage.preferred_height(),
                )
            })
            .collect()
    }

    pub(super) fn preferred_height(&self) -> f64 {
        let (ug, finder) = LimitFinder::surface(self.arguments.string_bounder.clone());
        let final_y = self.draw_internal(
            &ug,
            Context2D {
                is_background: false,
            },
        );
        let height = finder.borrow().min_max().dimension().height;
        height.max(final_y) + 10.0
    }
}

/// The diagram's body: heads, lifelines, tiles and tails (PlantUML's `PlayingSpaceWithParticipants`).
pub(super) struct PlayingSpaceWithParticipants<'a> {
    playing_space: PlayingSpace<'a>,
    page: usize,
    dimension: OnceCell<XDimension2D>,
}

impl<'a> PlayingSpaceWithParticipants<'a> {
    pub(super) fn new(playing_space: PlayingSpace<'a>, page: usize) -> Self {
        Self {
            playing_space,
            page,
            dimension: OnceCell::new(),
        }
    }

    fn y_min(&self) -> f64 {
        match self.page {
            0 => 0.0,
            page => self.playing_space.newpage_tops()[page - 1].0,
        }
    }

    /// The page ends after its page break, or with the diagram.
    fn y_max(&self, full_height: f64) -> f64 {
        match self.playing_space.newpage_tops().get(self.page) {
            None => full_height,
            Some(&(top, height)) => (top + height).min(full_height),
        }
    }

    pub(super) fn min_x(&self) -> &Real {
        self.playing_space.min()
    }

    fn head_height(&self) -> f64 {
        let arguments = &self.playing_space.arguments;
        arguments
            .living_spaces
            .head_height(arguments.string_bounder())
    }

    pub(super) fn calculate_dimension(&self) -> XDimension2D {
        *self.dimension.get_or_init(|| {
            let space = &self.playing_space;
            let width = space.max().current_value() - space.min().current_value();
            let full_height = space.preferred_height();
            let page_height = self.y_max(full_height) - self.y_min();
            let factor = if space.show_footbox { 2.0 } else { 1.0 };
            XDimension2D::new(width, page_height + factor * self.head_height())
        })
    }

    pub(super) fn draw_u(&self, ug: &UGraphic) {
        let context = Context2D {
            is_background: false,
        };
        let space = &self.playing_space;
        let living_spaces = &space.arguments.living_spaces;
        let head_height = self.head_height();
        let full_height = space.preferred_height();
        let y_min = self.y_min();
        let page_height = self.y_max(full_height) - y_min;
        let mut body = ug.translated(0.0, head_height - y_min);
        if !space.newpage_tops().is_empty() {
            body = body.with_clip(UClip::new(-1000.0, y_min, f64::MAX, page_height + 1.0));
        }
        space.draw_background(&body);
        living_spaces.draw_life_lines(&body, full_height, context);
        living_spaces.draw_heads(ug, context, VerticalAlignment::Bottom);
        if space.show_footbox {
            living_spaces.draw_heads(
                &ug.translated(0.0, page_height + head_height),
                context,
                VerticalAlignment::Top,
            );
        }
        space.draw_foreground(&body);
    }
}
