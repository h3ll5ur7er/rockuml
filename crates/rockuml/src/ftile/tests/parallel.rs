//! The tiles forks and splits are built from.

use std::rc::Rc;

use super::{Innermost, Tile};
use crate::color::Colors;
use crate::diagram::activity3::{ForkStyle, SwimlaneId};
use crate::ftile::vcompact::{FtileFactoryDelegatorCreateParallel, FtileForkInner};
use crate::ftile::vertical::{FtileBlackBlock, FtileThinSplit};
use crate::ftile::{
    Ftile, FtileFactory, FtileGeometry, FtileHeightFixedCentered, FtileHeightFixedMarged, Swimable,
};
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::tests::recording;
use crate::skin::SkinParam;

fn geometry(tile: &dyn Ftile) -> FtileGeometry {
    tile.calculate_dimension(&StringBounderDebug)
}

#[test]
fn a_tile_of_fixed_height_centres_its_tile_down_it() {
    let tile = FtileHeightFixedCentered::new(Tile::create(20.0, vec![]), 60.0);
    assert_eq!(
        geometry(&tile),
        FtileGeometry::with_out(10.0, 60.0, 5.0, 20.0, 40.0)
    );
    let (ug, recorder) = recording();
    tile.draw_u(&ug);
    assert_eq!(recorder.borrow().lines, ["rect 0,20 10x20"]);
}

#[test]
fn a_marged_tile_of_fixed_height_adds_room_above_and_below() {
    let tile = FtileHeightFixedMarged::new(3.0, Tile::create(20.0, vec![]), 4.0);
    assert_eq!(
        geometry(&tile),
        FtileGeometry::with_out(10.0, 27.0, 5.0, 3.0, 23.0)
    );
    let (ug, recorder) = recording();
    tile.draw_u(&ug);
    assert_eq!(recorder.borrow().lines, ["rect 0,3 10x20"]);
}

#[test]
fn the_flows_of_a_fork_stand_side_by_side_as_high_as_the_highest() {
    let short = Tile::create(5.0, vec![]);
    let tall = Tile::create(20.0, vec![]);
    let inner = FtileForkInner::new(vec![short.clone(), tall.clone()]);
    assert_eq!(
        geometry(&inner),
        FtileGeometry::with_out(20.0, 20.0, 10.0, 0.0, 20.0)
    );
    assert_eq!(
        inner.get_translate_for(tall.as_ref(), &StringBounderDebug),
        UTranslate::new(10.0, 0.0)
    );
    let (ug, recorder) = recording();
    inner.draw_u(&ug);
    assert_eq!(
        recorder.borrow().lines,
        ["rect 0,0 10x5", "rect 10,0 10x20"]
    );
}

#[test]
fn a_black_bar_is_entered_and_left_in_its_middle() {
    let bar = FtileBlackBlock::new(
        Rc::new(SkinParam::default()),
        Some(SwimlaneId(1)),
        Colors::default(),
        100.0,
        6.0,
        None,
    );
    assert_eq!(
        geometry(&bar),
        FtileGeometry::with_out(100.0, 6.0, 50.0, 0.0, 6.0)
    );
    assert_eq!(bar.get_swimlane_out(), Some(SwimlaneId(1)));
    let (ug, recorder) = recording();
    bar.draw_u(&ug);
    assert_eq!(recorder.borrow().lines, ["rect 0,0 100x6"]);
}

#[test]
fn a_thin_split_line_runs_from_the_first_flow_to_the_last() {
    let line = FtileThinSplit::new(Rc::new(SkinParam::default()), None, None, 20.0, 70.0, 100.0);
    assert_eq!(
        geometry(&line),
        FtileGeometry::with_out(100.0, 1.5, 50.0, 0.0, 1.5)
    );
    let (ug, recorder) = recording();
    line.draw_u(&ug);
    assert_eq!(
        recorder.borrow().lines,
        ["line 20,0 50,0 stroke 0.0-0.0-1.5"]
    );
}

#[test]
fn arrows_into_a_join_bar_from_another_lane_go_down_across_and_down() {
    let factory = FtileFactoryDelegatorCreateParallel::new(Box::new(Rc::new(Innermost::default())));
    let fork = factory.create_parallel(
        vec![Tile::create(5.0, vec![]), Tile::create(5.0, vec![])],
        ForkStyle::Fork,
        None,
        None,
        None,
        &Colors::default(),
    );
    let connections = fork.get_inner_connections();
    assert_eq!(connections.len(), 2);
    let (ug, recorder) = recording();
    connections[1]
        .as_translatable()
        .expect("arrows out of fork flows cross lanes")
        .draw_translate(&ug, UTranslate::new(0.0, 0.0), UTranslate::new(100.0, 50.0));
    // The second flow, 38 wide once marged, is left 38 + 19 across; it ends 25 down its 45-high slot,
    // below the 6-high bar. The join bar is 6 + 5 down (bar and flows), 50 more in the other lane, and
    // the arrow turns 14 above it.
    assert_eq!(
        recorder.borrow().lines[..3],
        ["line 57,31 0,16", "line 57,47 100,0", "line 157,47 0,14"]
    );
}
