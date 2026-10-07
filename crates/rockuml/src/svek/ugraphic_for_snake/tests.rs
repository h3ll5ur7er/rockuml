use super::*;
use crate::color::HColor;
use crate::decoration::Rainbow;
use crate::ftile::MergeStrategy;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::tests::recording;
use crate::skin::SkinParam;

fn arrow(skin: &SkinParam, from: (f64, f64), to: (f64, f64)) -> Snake {
    let mut snake = Snake::create_with_end(
        skin,
        Rainbow::from_color(Some(HColor::RED), None),
        skin.arrows().as_to_down(),
    );
    snake.add_point(from.0, from.1);
    snake.add_point(to.0, to.1);
    snake
}

const ARROWHEAD: &str = "[(-4.0, -10.0), (0.0, 0.0), (4.0, -10.0), (0.0, -6.0)]";

#[test]
fn arrows_are_drawn_after_everything_else_merged_in_first_drawn_order() {
    let skin = SkinParam::default();
    let (surface, recorder) = recording();
    let ug = UGraphicForSnake::create(surface);
    let box_ = UShape::Rectangle(URectangle::new(10.0, 5.0));
    ug.draw(&box_);
    ug.draw(&arrow(&skin, (0.0, 0.0), (0.0, 10.0)));
    ug.draw(&arrow(&skin, (50.0, 0.0), (50.0, 10.0)));
    // Continues the first arrow, drawn where it starts.
    ug.translated(0.0, 10.0)
        .draw(&arrow(&skin, (0.0, 0.0), (0.0, 10.0)));
    ug.translated(0.0, 30.0).draw(&box_);
    assert_eq!(recorder.borrow().lines.len(), 2);
    ug.flush_ug();
    assert_eq!(
        recorder.borrow().lines,
        [
            "rect 0,0 10x5".to_owned(),
            "rect 0,30 10x5".to_owned(),
            "line 0,0 0,20".to_owned(),
            format!("polygon 0,20 {ARROWHEAD}"),
            "line 50,0 0,10".to_owned(),
            format!("polygon 50,10 {ARROWHEAD}"),
        ]
    );
    // Flushing again draws nothing more.
    ug.flush_ug();
    assert_eq!(recorder.borrow().lines.len(), 6);
}

#[test]
fn an_arrow_ending_where_a_mergeable_one_starts_loses_its_arrowhead() {
    let skin = SkinParam::default();
    let (surface, recorder) = recording();
    let ug = UGraphicForSnake::create(surface);
    ug.draw(&arrow(&skin, (0.0, 0.0), (0.0, 10.0)).with_merge(MergeStrategy::None));
    ug.draw(&arrow(&skin, (0.0, 10.0), (0.0, 20.0)));
    ug.flush_ug();
    assert_eq!(
        recorder.borrow().lines,
        [
            "line 0,0 0,10".to_owned(),
            "line 0,10 0,10".to_owned(),
            format!("polygon 0,20 {ARROWHEAD}"),
        ]
    );
}

#[test]
fn copies_know_how_far_they_moved_since_the_layer_was_set() {
    let (surface, _) = recording();
    let ug = UGraphicForSnake::create(surface.translated(100.0, 100.0));
    let moved = ug
        .translated(3.0, 4.0)
        .with_color(HColor::RED)
        .translated(1.0, 1.0);
    let layer = moved.layer::<UGraphicForSnake>().unwrap();
    assert_eq!(layer.get_translation(), UTranslate::new(4.0, 5.0));
    assert_eq!(moved.param().color, HColor::RED);
}
