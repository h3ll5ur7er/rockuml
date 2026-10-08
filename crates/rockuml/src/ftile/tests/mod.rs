mod arrows;

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::decoration::symbol::USymbol;
use crate::diagram::activity3::{
    BranchFtile, ForkStyle, InstructionId, Instructions, PositionedNote, SwimlaneId, SwimlaneSet,
};
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::tests::recording;
use crate::klimt::url::Url;
use crate::klimt::{UDrawable, VerticalAlignment};
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{Style, StyleBuilder};
use crate::svek::UGraphicForSnake;

/// A box with children below it, joined to each by an arrow.
pub(super) struct Tile {
    base: AbstractFtile,
    height: f64,
    children: Vec<Rc<dyn Ftile>>,
}

impl Tile {
    pub(super) fn create(height: f64, children: Vec<Rc<dyn Ftile>>) -> Rc<dyn Ftile> {
        Rc::new(Self {
            base: AbstractFtile::new(Rc::new(SkinParam::default())),
            height,
            children,
        })
    }
}

impl Swimable for Tile {
    fn get_swimlanes(&self) -> SwimlaneSet {
        SwimlaneSet::new()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        None
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        None
    }
}

impl Ftile for Tile {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            FtileGeometry::with_out(10.0, self.height, 5.0, 0.0, self.height)
        })
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        _string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let index = self
            .children
            .iter()
            .position(|tile| same(tile.as_ref(), child))
            .unwrap_or_default();
        UTranslate::new(0.0, 20.0 * (index + 1) as f64)
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.children.clone()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.draw(&UShape::Rectangle(URectangle::new(10.0, self.height)));
        for child in &self.children {
            let translate = self.get_translate_for(child.as_ref(), string_bounder);
            ug.apply(translate).draw(child);
            let mut snake = Snake::create(
                self.skin_param(),
                Rainbow::from_color(Some(HColor::RED), None),
            );
            snake.add_point(5.0, self.height);
            snake.add_point(5.0, translate.dy);
            ug.draw(&snake);
        }
    }
}

#[test]
fn tiles_are_dispatched_through_the_layers_and_arrows_drawn_last() {
    let (surface, recorder) = recording();
    let tree = Tile::create(
        5.0,
        vec![Tile::create(5.0, vec![]), Tile::create(5.0, vec![])],
    );
    TextBlockInterceptorUDrawable::new(tree, HColor::RED, false)
        .draw_u(&UGraphicForSnake::create(surface.translated(1.0, 1.0)));
    assert_eq!(
        recorder.borrow().lines,
        [
            "rect 1,1 10x5",
            "rect 1,21 10x5",
            "rect 1,41 10x5",
            "line 6,6 0,15",
            "line 6,6 0,35",
        ]
    );
}

#[test]
fn tiles_find_their_children_by_identity() {
    let child: Rc<dyn Ftile> = Tile::create(5.0, vec![]);
    let twin: Rc<dyn Ftile> = Tile::create(5.0, vec![]);
    let parent = Tile::create(5.0, vec![twin.clone(), child.clone()]);
    let string_bounder = StringBounderDebug;
    assert_eq!(
        parent.get_translate_for(child.as_ref(), &string_bounder),
        UTranslate::new(0.0, 40.0)
    );
    assert!(same(child.as_ref(), parent.get_my_children()[1].as_ref()));
    assert!(!same(child.as_ref(), twin.as_ref()));
}

/// Builds plain tiles and counts what it was asked.
#[derive(Default)]
pub(super) struct Innermost {
    skin_param: Rc<SkinParam>,
    calls: RefCell<Vec<&'static str>>,
}

impl Innermost {
    fn tile(&self, call: &'static str) -> Rc<dyn Ftile> {
        self.calls.borrow_mut().push(call);
        Tile::create(5.0, vec![])
    }
}

impl FtileFactory for Rc<Innermost> {
    fn get_string_bounder(&self) -> &dyn StringBounder {
        &StringBounderDebug
    }

    fn skin_param(&self) -> &Rc<SkinParam> {
        &self.skin_param
    }

    fn start(&self, _swimlane: Option<SwimlaneId>, _colors: &Colors) -> Rc<dyn Ftile> {
        self.tile("start")
    }

    fn stop(&self, _swimlane: Option<SwimlaneId>, _colors: &Colors) -> Rc<dyn Ftile> {
        self.tile("stop")
    }

    fn end(&self, _swimlane: Option<SwimlaneId>, _colors: &Colors) -> Rc<dyn Ftile> {
        self.tile("end")
    }

    fn spot(
        &self,
        _swimlane: Option<SwimlaneId>,
        _spot: &str,
        _color: Option<HColor>,
    ) -> Rc<dyn Ftile> {
        self.tile("spot")
    }

    fn activity(
        &self,
        _label: &Display,
        _swimlane: Option<SwimlaneId>,
        _style: BoxStyle,
        _colors: &Colors,
        _stereotype: Option<&Stereotype>,
        _style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        self.tile("activity")
    }

    fn add_url(&self, ftile: Rc<dyn Ftile>, _url: &Url) -> Rc<dyn Ftile> {
        self.calls.borrow_mut().push("add_url");
        ftile
    }

    fn decorate_in(&self, ftile: Rc<dyn Ftile>, _link: &LinkRendering) -> Rc<dyn Ftile> {
        self.calls.borrow_mut().push("decorate_in");
        ftile
    }

    fn decorate_out(&self, ftile: Rc<dyn Ftile>, _link: &LinkRendering) -> Rc<dyn Ftile> {
        self.calls.borrow_mut().push("decorate_out");
        ftile
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        self.calls.borrow_mut().push("assembly");
        Tile::create(5.0, vec![tile1, tile2])
    }

    fn add_note(
        &self,
        _ftile: Option<Rc<dyn Ftile>>,
        _swimlane: Option<SwimlaneId>,
        _notes: &[PositionedNote],
        _vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile> {
        self.tile("add_note")
    }

    fn repeat(
        &self,
        _stereotype: &Stereogroup,
        _stereotype2: &Stereogroup,
        _box_style_in: BoxStyle,
        _swimlane: Option<SwimlaneId>,
        _swimlane_out: Option<SwimlaneId>,
        _start_label: Option<&Display>,
        _repeat: Rc<dyn Ftile>,
        _test: Option<&Display>,
        _yes: Option<&Display>,
        _out: Option<&Display>,
        _backward: Option<Rc<dyn Ftile>>,
        _no_out: bool,
        _incoming1: &LinkRendering,
        _incoming2: &LinkRendering,
        _current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        self.tile("repeat")
    }

    fn create_while(
        &self,
        _instructions: &Instructions,
        _out_color: &LinkRendering,
        _swimlane: Option<SwimlaneId>,
        _while_block: Rc<dyn Ftile>,
        _test: &Display,
        _yes: Option<&Display>,
        _color: Option<HColor>,
        _special_out: Option<InstructionId>,
        _backward: Option<Rc<dyn Ftile>>,
        _incoming1: &LinkRendering,
        _incoming2: &LinkRendering,
        _current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        self.tile("create_while")
    }

    fn create_if(
        &self,
        _instructions: &Instructions,
        _swimlane: Option<SwimlaneId>,
        _thens: &[BranchFtile<'_>],
        _else_branch: &BranchFtile<'_>,
        _out_color: &LinkRendering,
        _top_inlink_rendering: &LinkRendering,
        _url: Option<&Url>,
        _notes: &[PositionedNote],
        _stereotype: Option<&Stereotype>,
        _current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        self.tile("create_if")
    }

    fn create_switch(
        &self,
        _instructions: &Instructions,
        _swimlane: Option<SwimlaneId>,
        _branches: &[BranchFtile<'_>],
        _after_endwhile: &LinkRendering,
        _top_inlink_rendering: &LinkRendering,
        _label_test: Option<&Display>,
        _colors: &Colors,
        _end_colors: Option<&Colors>,
    ) -> Rc<dyn Ftile> {
        self.tile("create_switch")
    }

    fn create_parallel(
        &self,
        _all: Vec<Rc<dyn Ftile>>,
        _style: ForkStyle,
        _label: Option<&str>,
        _swimlane_in: Option<SwimlaneId>,
        _swimlane_out: Option<SwimlaneId>,
        _colors: &Colors,
    ) -> Rc<dyn Ftile> {
        self.tile("create_parallel")
    }

    fn create_group(
        &self,
        _list: Rc<dyn Ftile>,
        _name: &Display,
        _back_color: Option<HColor>,
        _note: Option<&PositionedNote>,
        _type_: USymbol,
        _style: &Style,
    ) -> Rc<dyn Ftile> {
        self.tile("create_group")
    }
}

#[test]
fn the_delegator_chain_passes_what_it_does_not_change_inwards() {
    let innermost = Rc::new(Innermost::default());
    let factory = vcompact::delegator_chain(Box::new(innermost.clone()), false);
    let start = factory.start(None, &Colors::default());
    let stop = factory.stop(Some(SwimlaneId(0)), &Colors::default());
    let assembled = factory.assembly(start, stop);
    let decorated = factory.decorate_out(assembled, &LinkRendering::none());
    factory.create_parallel(
        vec![decorated],
        ForkStyle::Fork,
        None,
        None,
        None,
        &Colors::default(),
    );
    assert_eq!(
        *innermost.calls.borrow(),
        [
            "start",
            "stop",
            "assembly",
            "decorate_out",
            "create_parallel"
        ]
    );
    assert!(Rc::ptr_eq(factory.skin_param(), &innermost.skin_param));
}
