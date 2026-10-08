//! The loops, measured and drawn around tiles of known sizes.

use std::rc::Rc;

use super::ftile_repeat::{FtileRepeat, RepeatStyle};
use super::ftile_while::{FtileWhile, WhileStyle};
use crate::color::HColor;
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::diagram::activity3::LinkRendering;
use crate::ftile::tests::{Innermost, Tile};
use crate::ftile::{Ftile, FtileFactory, TextBlockInterceptorUDrawable};
use crate::klimt::UDrawable;
use crate::klimt::debug::StringBounderDebug;
use crate::klimt::font::FontConfiguration;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::tests::recording;
use crate::style::{SName, StyleSignature};
use crate::svek::{ConditionStyle, UGraphicForSnake};

/// What drawing `tile` as a diagram draws.
fn drawn(tile: Rc<dyn Ftile>) -> Vec<String> {
    let (surface, recorder) = recording();
    TextBlockInterceptorUDrawable::new(tile, HColor::RED, false)
        .draw_u(&UGraphicForSnake::create(surface));
    recorder.take().lines
}

fn font(factory: &dyn FtileFactory) -> FontConfiguration {
    StyleSignature::of(&[
        SName::Root,
        SName::Element,
        SName::ActivityDiagram,
        SName::Arrow,
    ])
    .get_merged_style(&factory.skin_param().current_style_builder())
    .font_configuration()
}

fn red() -> Rainbow {
    Rainbow::from_color(Some(HColor::RED), None)
}

fn while_style(factory: &dyn FtileFactory) -> WhileStyle {
    WhileStyle {
        border_color: HColor::RED,
        back_color: HColor::WHITE,
        arrow_color: red(),
        font_arrow: font(factory),
        condition_style: ConditionStyle::InsideHexagon,
        fc_test: font(factory),
    }
}

/// A `while` with an empty test around a body 10 wide and 30 high.
fn simple_while(factory: &dyn FtileFactory, body: &Rc<dyn Ftile>) -> Rc<dyn Ftile> {
    FtileWhile::create(
        &LinkRendering::none().with_rainbow(red()),
        None,
        body.clone(),
        &Display::create([""]),
        None,
        &while_style(factory),
        factory,
        None,
        None,
        &LinkRendering::none().with_rainbow(red()),
        &LinkRendering::none().with_rainbow(red()),
    )
}

#[test]
fn a_while_puts_its_test_above_its_body_with_room_for_the_arrows_around() {
    let factory = Rc::new(Innermost::default());
    let body = Tile::create(30.0, vec![]);
    let tile = simple_while(&factory, &body);
    let string_bounder = StringBounderDebug;

    let geometry = tile.calculate_dimension(&string_bounder);
    assert_eq!(
        (
            geometry.get_width(),
            geometry.get_height(),
            geometry.get_left(),
            geometry.get_in_y(),
            geometry.get_out_y()
        ),
        (60.0, 102.0, 36.0, 0.0, 102.0)
    );
    let loop_tile = &tile.get_my_children()[0];
    assert_eq!(
        loop_tile.get_translate_for(body.as_ref(), &string_bounder),
        UTranslate::new(31.0, 48.0)
    );
    let diamond = &loop_tile.get_my_children()[1];
    assert_eq!(
        loop_tile.get_translate_for(diamond.as_ref(), &string_bounder),
        UTranslate::new(24.0, 0.0)
    );
    let connections = tile.get_inner_connections();
    assert_eq!(connections.len(), 3);
    assert_eq!(
        connections
            .iter()
            .map(|connection| connection.as_translatable().is_some())
            .collect::<Vec<_>>(),
        [true, true, false]
    );
}

#[test]
fn a_while_draws_its_body_and_test_then_the_arrows_in_and_around() {
    let factory = Rc::new(Innermost::default());
    let body = Tile::create(30.0, vec![]);
    let tile = simple_while(&factory, &body);
    assert_eq!(
        drawn(tile),
        [
            "rect 31,48 10x30",
            "polygon 24,0 [(12.0, 0.0), (12.0, 0.0), (24.0, 12.0), (12.0, 24.0), (12.0, 24.0), (0.0, 12.0), (12.0, 0.0)] stroke 0.0-0.0-0.5",
            "UEmpty 36,90",
            // In, from the test down into the body.
            "line 36,24 0,24",
            "polygon 36,48 [(-4.0, -10.0), (0.0, 0.0), (4.0, -10.0), (0.0, -6.0)]",
            // Back, from the body around the right into the test, its upward direction emphasized.
            "line 36,78 0,12",
            "line 36,90 24,0",
            "polygon 60,51 [(-4.0, 10.0), (0.0, 0.0), (4.0, 10.0), (0.0, 6.0)]",
            "line 60,90 0,-78",
            "line 60,12 -12,0",
            "polygon 48,12 [(10.0, -4.0), (0.0, 0.0), (10.0, 4.0), (6.0, 0.0)]",
            // Out, from the test around the left to the bottom.
            "line 24,12 -12,0",
            "polygon 12,57 [(-4.0, -10.0), (0.0, 0.0), (4.0, -10.0), (0.0, -6.0)]",
            "line 12,12 0,90",
            "line 12,102 24,0",
        ]
    );
}

fn repeat_style(factory: &dyn FtileFactory) -> RepeatStyle {
    RepeatStyle {
        border_color: HColor::RED,
        diamond_color1: HColor::WHITE,
        diamond_color2: HColor::WHITE,
        arrow_color: red(),
        end_repeat_link_color: Rainbow::none(),
        condition_style: ConditionStyle::InsideHexagon,
        fc_diamond: font(factory),
        fc_arrow: font(factory),
    }
}

#[test]
fn a_repeat_puts_its_entry_above_its_body_and_its_test_below() {
    let factory = Rc::new(Innermost::default());
    let body = Tile::create(30.0, vec![]);
    let tile = FtileRepeat::create(
        None,
        None,
        None,
        body.clone(),
        None,
        None,
        None,
        &repeat_style(&factory),
        factory.skin_param(),
        None,
        false,
        &LinkRendering::none(),
        &LinkRendering::none(),
    );
    let string_bounder = StringBounderDebug;

    let geometry = tile.calculate_dimension(&string_bounder);
    assert_eq!(
        (
            geometry.get_width(),
            geometry.get_height(),
            geometry.get_left(),
            geometry.get_in_y(),
            geometry.get_out_y()
        ),
        (48.0, 174.0, 12.0, 0.0, 174.0)
    );
    let loop_tile = &tile.get_my_children()[0];
    assert_eq!(
        loop_tile.get_translate_for(body.as_ref(), &string_bounder),
        UTranslate::new(7.0, 72.0)
    );

    assert_eq!(
        drawn(tile),
        [
            "rect 7,72 10x30",
            "polygon 0,0 [(12.0, 0.0), (24.0, 12.0), (12.0, 24.0), (0.0, 12.0), (12.0, 0.0)] stroke 0.0-0.0-0.5",
            "polygon 0,150 [(12.0, 0.0), (12.0, 0.0), (24.0, 12.0), (12.0, 24.0), (12.0, 24.0), (0.0, 12.0), (12.0, 0.0)] stroke 0.0-0.0-0.5",
            // In, from the entry down into the body.
            "line 12,24 0,48",
            "polygon 12,72 [(-4.0, -10.0), (0.0, 0.0), (4.0, -10.0), (0.0, -6.0)]",
            // Back, from the test around the right into the entry.
            "line 24,162 12,0",
            "polygon 36,87 [(-4.0, 10.0), (0.0, 0.0), (4.0, 10.0), (0.0, 6.0)]",
            "line 36,162 0,-150",
            "line 36,12 -12,0",
            "polygon 24,12 [(10.0, -4.0), (0.0, 0.0), (10.0, 4.0), (6.0, 0.0)]",
            // Out, from the body down into the test.
            "line 12,102 0,48",
            "polygon 12,150 [(-4.0, -10.0), (0.0, 0.0), (4.0, -10.0), (0.0, -6.0)]",
        ]
    );
}
