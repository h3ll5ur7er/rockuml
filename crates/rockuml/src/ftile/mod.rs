//! The tiles activity diagrams are drawn with (PlantUML's `activitydiagram3.ftile`, with its `vcompact`
//! and `vertical` packages).
//!
//! # How a diagram is drawn
//!
//! Each instruction of the model builds an [`Ftile`] through an [`FtileFactory`]; the tiles nest into one
//! tree, which is rebuilt every time it is measured or drawn, so building must not change the model.
//! Tiles draw their children with `ug.draw(&child)`, their [`Connection`]s with `ug.draw(&connection)`
//! and arrows ([`Snake`]) with `ug.draw(&snake)`, never by calling `draw_u` themselves: the
//! [`UGraphic`] they draw on is a stack of
//! [layers](crate::klimt::ugraphic::UGraphicLayer) deciding what is drawn where.
//!
//! - `UGraphicForSnake` (in `svek`) collects arrows, merges those that continue each other, and draws
//!   them on `flush_ug`, after everything else.
//! - [`UGraphicDispatchFtile`] draws tiles and connections back through itself and records where labels
//!   are, for `goto`. [`TextBlockInterceptorUDrawable`] draws the tree through it on a diagram without
//!   swimlanes.
//! - `UGraphicDispatchDrawable` (in `klimt`) does the same without labels, to measure a group's inside.
//! - The swimlane interceptors draw the tree once per lane, keeping what lies in that lane, then once more
//!   for the arrows crossing lanes (still to come, see below).
//! - The compression layer squeezes empty space out across, then down; `SlotFinder` and `LimitFinder` are
//!   surfaces that measure.
//!
//! Tiles ask about the layer they are handed with `ug.layer::<UGraphicInterceptorOneSwimlane>()`, PlantUML's
//! `ug instanceof UGraphicInterceptorOneSwimlane`.
//!
//! # Identity
//!
//! PlantUML compares tiles by identity (`getTranslateFor(child)`, `Genealogy`, maps keyed by tile).
//! Tiles are shared as `Rc<dyn Ftile>` and compared with [`same`]. Swimlanes are
//! [`SwimlaneId`](crate::diagram::activity3::SwimlaneId)s.
//!
//! # Factories
//!
//! [`vcompact::delegator_chain`] wraps the innermost factory, `VCompactFactory`, in PlantUML's chain of
//! [`FtileFactoryDelegator`]s; each delegator overrides the methods it changes and passes the others on.
//!
//! # Still to come
//!
//! - `VCompactFactory` and the tiles it builds; building tiles from instructions (`createFtile`);
//!   `Swimlanes`' drawing with its `Cross` layer.
//! - The swimlane layers `UGraphicInterceptorOneSwimlane` and `UGraphicInterceptorAllSwimlanes`, in
//!   `ftile/vcompact/`, implementing [`UGraphicLayer`](crate::klimt::ugraphic::UGraphicLayer). The
//!   latter fans out to one surface per lane: its `ug` is the first lane's, for queries, and it overrides
//!   groups and links (ignored) and `flush_ug` (every lane), as PlantUML's does.
//! - The compression layer and its slot finder, in `klimt::compress`.
//! - The delegators' own methods, each in its file under `vcompact/`.

mod abstract_connection;
mod abstract_ftile;
mod arrows;
mod box_style;
mod connection;
mod ftile_factory;
mod ftile_factory_delegator;
mod ftile_geometry;
mod ftile_geometry_merger;
pub(crate) mod hexagon;
mod merge_strategy;
mod snake;
mod swimable;
mod text_block_interceptor_udrawable;
mod ugraphic_dispatch_ftile;
pub(crate) mod vcompact;
pub(crate) mod vertical;
mod worm;
mod worm_mutation;

use std::any::Any;
use std::rc::Rc;

pub(crate) use abstract_connection::AbstractConnection;
pub(crate) use abstract_ftile::AbstractFtile;
pub(crate) use arrows::Arrows;
pub(crate) use box_style::BoxStyle;
pub(crate) use connection::{Connection, ConnectionTranslatable};
pub(crate) use ftile_factory::FtileFactory;
pub(crate) use ftile_factory_delegator::FtileFactoryDelegator;
pub(crate) use ftile_geometry::FtileGeometry;
pub(crate) use ftile_geometry_merger::FtileGeometryMerger;
pub(crate) use merge_strategy::MergeStrategy;
pub(crate) use snake::Snake;
pub(crate) use swimable::Swimable;
pub(crate) use text_block_interceptor_udrawable::TextBlockInterceptorUDrawable;
pub(crate) use ugraphic_dispatch_ftile::UGraphicDispatchFtile;
pub(crate) use worm::Worm;
pub(crate) use worm_mutation::WormMutation;

use crate::diagram::activity3::LinkRendering;
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

/// A tile of an activity diagram (PlantUML's `Ftile`): a block with a point where arrows enter it and,
/// unless it ends the flow, one where they leave it.
///
/// The defaults are those of PlantUML's `AbstractFtile`. Where it fails because a tile has no children,
/// these answer that it has none.
///
/// A tile is [`Any`] so that tiles can ask what another tile is, as PlantUML's `instanceof` does, with
/// [`downcast`].
pub(crate) trait Ftile: Swimable + Any {
    fn skin_param(&self) -> &SkinParam;

    /// How the arrow into the tile is drawn.
    fn get_in_link_rendering(&self) -> LinkRendering {
        LinkRendering::none()
    }

    /// How the arrow out of the tile is drawn.
    fn get_out_link_rendering(&self) -> LinkRendering {
        LinkRendering::none()
    }

    /// The tile's size and entry and exit points; most tiles compute it once, through
    /// [`AbstractFtile::calculate_dimension`].
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry;

    /// Where the tile draws `child`, one of its children.
    fn get_translate_for(
        &self,
        _child: &dyn Ftile,
        _string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        UTranslate::default()
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        Vec::new()
    }

    fn get_inner_connections(&self) -> Vec<Rc<dyn Connection>> {
        Vec::new()
    }

    /// The `break`s inside, which the enclosing loop joins to its exit (PlantUML's `WeldingPoint`s, which
    /// are always `FtileBreak`s).
    fn get_welding_points(&self) -> Vec<Rc<dyn Ftile>> {
        Vec::new()
    }

    /// Where the labels of arrows from the tile go: `skinparam arrowMessageAlignment`.
    fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        self.skin_param().arrow_message_alignment()
    }

    /// Draws the tile: its shapes, its children with `ug.draw(&child)` and its connections and arrows with
    /// `ug.draw(...)`.
    fn draw_u(&self, ug: &UGraphic);
}

/// Whether `a` and `b` are the same tile, as PlantUML compares tiles.
pub(crate) fn same(a: &dyn Ftile, b: &dyn Ftile) -> bool {
    std::ptr::addr_eq(a, b)
}

/// `tile` as a `T`, when it is one (PlantUML's `tile instanceof T`).
pub(crate) fn downcast<T: Ftile>(tile: &dyn Ftile) -> Option<&T> {
    (tile as &dyn Any).downcast_ref()
}

#[cfg(test)]
mod tests;
