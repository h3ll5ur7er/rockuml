//! PlantUML's `ParallelBuilderMerge`: flows from a black bar into a diamond (`end merge`).

use std::rc::Rc;

use super::abstract_parallel_ftiles_builder::{
    AbstractParallelFtilesBuilder, BAR_HEIGHT, ConnectionIn, ParallelFtilesBuilder,
};
use crate::color::Colors;
use crate::decoration::Rainbow;
use crate::ftile::vertical::{FtileBlackBlock, FtileDiamond};
use crate::ftile::{
    AbstractConnection, Connection, Ftile, FtileAssemblySimple, Snake, ftile_utils,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

pub(super) struct ParallelBuilderMerge<'a> {
    base: AbstractParallelFtilesBuilder<'a>,
}

impl<'a> ParallelBuilderMerge<'a> {
    pub(super) fn new(
        skin_param: Rc<SkinParam>,
        string_bounder: &'a dyn StringBounder,
        all: &[Rc<dyn Ftile>],
        colors: Colors,
    ) -> Self {
        Self {
            base: AbstractParallelFtilesBuilder::new(skin_param, string_bounder, all, colors),
        }
    }

    fn style(&self) -> Style {
        self.base.merged_style(&StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
        ]))
    }
}

impl ParallelFtilesBuilder for ParallelBuilderMerge<'_> {
    fn list99(&self) -> &[Rc<dyn Ftile>] {
        &self.base.list99
    }

    fn do_step1(&self, inner: &Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let base = &self.base;
        // Arrows leave a tile's geometry as it is, so the bar spans the flows with or without them.
        let width = inner.calculate_dimension(base.string_bounder).get_width();
        let black: Rc<dyn Ftile> = Rc::new(FtileBlackBlock::new(
            base.skin_param.clone(),
            base.list99[0].get_swimlane_in(),
            base.colors.clone(),
            width,
            BAR_HEIGHT,
            None,
        ));
        let def = Rainbow::build_from_style(&self.style());
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let mut x = 0.0;
        for tmp in &base.list99 {
            let dim = tmp.calculate_dimension(base.string_bounder);
            let rainbow = tmp.get_in_link_rendering().get_rainbow_or(&def);
            conns.push(Rc::new(ConnectionIn::new(
                base.skin_param.clone(),
                black.clone(),
                tmp.clone(),
                x,
                rainbow,
                false,
            )));
            x += dim.get_width();
        }
        let result = ftile_utils::add_connections(inner.clone(), conns);
        Rc::new(FtileAssemblySimple::new(black, result))
    }

    fn do_step2(&self, _inner: &Rc<dyn Ftile>, result: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let base = &self.base;
        let style = self.style();
        let border_color = style.value(PName::LineColor).as_color();
        let back_color = style.value(PName::BackGroundColor).as_color();
        let out: Rc<dyn Ftile> = Rc::new(FtileDiamond::new(
            base.skin_param.clone(),
            back_color,
            border_color,
            base.swimlane_out_for_step2(),
        ));
        let result: Rc<dyn Ftile> = Rc::new(FtileAssemblySimple::new(result, out.clone()));
        let diamond_translate = result.get_translate_for(out.as_ref(), base.string_bounder);
        let def = Rainbow::build_from_style(&style);
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let mut x = 0.0;
        for tmp in &base.list99 {
            let dim = tmp.calculate_dimension(base.string_bounder);
            let translate0 = UTranslate::new(x, BAR_HEIGHT);
            let rainbow = tmp.get_out_link_rendering().get_rainbow_or(&def);
            if dim.has_point_out() {
                conns.push(Rc::new(ConnectionHorizontalThenVertical {
                    base: AbstractConnection::new(Some(tmp.clone()), Some(out.clone())),
                    skin_param: base.skin_param.clone(),
                    arrow_color: rainbow,
                    translate0,
                    diamond_translate,
                }));
            }
            x += dim.get_width();
        }
        ftile_utils::add_connections(result, conns)
    }
}

/// The arrow from a flow down, then across into the diamond.
struct ConnectionHorizontalThenVertical {
    base: AbstractConnection,
    skin_param: Rc<SkinParam>,
    arrow_color: Rainbow,
    translate0: UTranslate,
    diamond_translate: UTranslate,
}

impl ConnectionHorizontalThenVertical {
    fn tile(&self) -> &Rc<dyn Ftile> {
        self.base.get_ftile1().expect("the arrow leaves a flow")
    }

    fn diamond(&self) -> &Rc<dyn Ftile> {
        self.base
            .get_ftile2()
            .expect("the arrow enters the diamond")
    }

    fn get_p1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        self.translate0.get_translated(
            self.tile()
                .calculate_dimension(string_bounder)
                .get_point_out(),
        )
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder, start_x: f64) -> XPoint2D {
        self.arrival_on_diamond(string_bounder, start_x)
            .get_translated(self.get_diamond_out(string_bounder))
    }

    fn get_diamond_out(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        self.diamond_translate.get_translated(
            self.diamond()
                .calculate_dimension(string_bounder)
                .get_point_out(),
        )
    }

    /// Where the arrow enters the diamond, from its point out: from the side `start_x` lies on, or
    /// from above.
    fn arrival_on_diamond(&self, string_bounder: &dyn StringBounder, start_x: f64) -> UTranslate {
        let result = self.get_diamond_out(string_bounder);
        let dim = self.diamond().calculate_dimension(string_bounder);
        let (width, height) = (dim.get_width(), dim.get_height());
        let a = result.x - width / 2.0;
        let b = result.x + width / 2.0;
        if start_x < a {
            UTranslate::new(-width / 2.0, -height / 2.0)
        } else if start_x > b {
            UTranslate::new(width / 2.0, -height / 2.0)
        } else {
            UTranslate::new(0.0, -height)
        }
    }
}

impl Connection for ConnectionHorizontalThenVertical {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let p1 = self.get_p1(string_bounder);
        let p2 = self.get_p2(string_bounder, p1.x);
        let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
        let arrival = self.arrival_on_diamond(string_bounder, p1.x);
        let arrows = self.skin_param.arrows();
        let end_decoration = if arrival.dx < 0.0 {
            arrows.as_to_right()
        } else if arrival.dx > 0.0 {
            arrows.as_to_left()
        } else {
            arrows.as_to_down()
        };
        let mut snake =
            Snake::create_with_end(&self.skin_param, self.arrow_color.clone(), end_decoration);
        snake.add_point(x1, y1);
        snake.add_point(x1, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }
}
