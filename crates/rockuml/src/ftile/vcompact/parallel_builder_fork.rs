//! PlantUML's `ParallelBuilderFork`: flows between two black bars, the second one labelled with the join
//! specification.

use std::rc::Rc;

use super::abstract_parallel_ftiles_builder::{
    AbstractParallelFtilesBuilder, BAR_HEIGHT, ConnectionIn, ParallelFtilesBuilder, get_text_block,
};
use crate::color::Colors;
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::diagram::activity3::SwimlaneId;
use crate::ftile::vertical::FtileBlackBlock;
use crate::ftile::{
    AbstractConnection, Connection, ConnectionTranslatable, Ftile, FtileAssemblySimple, Snake,
    ftile_utils,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

pub(super) struct ParallelBuilderFork<'a> {
    base: AbstractParallelFtilesBuilder<'a>,
    label: Option<&'a str>,
    in_: Option<SwimlaneId>,
    out: Option<SwimlaneId>,
}

impl<'a> ParallelBuilderFork<'a> {
    #[allow(clippy::too_many_arguments, reason = "PlantUML's ParallelBuilderFork")]
    pub(super) fn new(
        skin_param: Rc<SkinParam>,
        string_bounder: &'a dyn StringBounder,
        label: Option<&'a str>,
        in_: Option<SwimlaneId>,
        out: Option<SwimlaneId>,
        all: &[Rc<dyn Ftile>],
        colors: Colors,
    ) -> Self {
        Self {
            base: AbstractParallelFtilesBuilder::new(skin_param, string_bounder, all, colors),
            label,
            in_,
            out,
        }
    }

    fn arrow_rainbow(&self) -> Rainbow {
        let style = self
            .base
            .merged_style(&AbstractParallelFtilesBuilder::get_style_signature_arrow());
        Rainbow::build_from_style(&style)
    }

    fn get_just_before_bar2(&self, middle: &dyn Ftile) -> f64 {
        BAR_HEIGHT + self.base.get_height_of_middle(middle)
    }
}

impl ParallelFtilesBuilder for ParallelBuilderFork<'_> {
    fn list99(&self) -> &[Rc<dyn Ftile>] {
        &self.base.list99
    }

    fn do_step1(&self, middle: &Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let base = &self.base;
        // Arrows leave a tile's geometry as it is, so the bar spans the flows with or without them.
        let width = middle.calculate_dimension(base.string_bounder).get_width();
        let black: Rc<dyn Ftile> = Rc::new(FtileBlackBlock::new(
            base.skin_param.clone(),
            self.in_,
            base.colors.clone(),
            width,
            BAR_HEIGHT,
            None,
        ));
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let mut x = 0.0;
        for tmp in &base.list99 {
            let dim = tmp.calculate_dimension(base.string_bounder);
            let rainbow = tmp
                .get_in_link_rendering()
                .get_rainbow_or(&self.arrow_rainbow());
            conns.push(Rc::new(ConnectionIn::new(
                base.skin_param.clone(),
                black.clone(),
                tmp.clone(),
                x,
                rainbow,
                true,
            )));
            x += dim.get_width();
        }
        let result = ftile_utils::add_connections(middle.clone(), conns);
        Rc::new(FtileAssemblySimple::new(black, result))
    }

    fn do_step2(&self, middle: &Rc<dyn Ftile>, result: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let base = &self.base;
        let label = self.label.and_then(|label| {
            get_text_block(&base.skin_param, Some(&Display::with_newlines(label)))
        });
        let out: Rc<dyn Ftile> = Rc::new(FtileBlackBlock::new(
            base.skin_param.clone(),
            self.out,
            base.colors.clone(),
            result.calculate_dimension(base.string_bounder).get_width(),
            BAR_HEIGHT,
            label,
        ));
        let result: Rc<dyn Ftile> = Rc::new(FtileAssemblySimple::new(result, out.clone()));
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let mut x = 0.0;
        for tmp in &base.list99 {
            let dim = tmp.calculate_dimension(base.string_bounder);
            let rainbow = tmp
                .get_out_link_rendering()
                .get_rainbow_or(&self.arrow_rainbow());
            if dim.has_point_out() {
                conns.push(Rc::new(ConnectionOut::new(
                    base.skin_param.clone(),
                    tmp.clone(),
                    out.clone(),
                    x,
                    rainbow,
                    self.get_just_before_bar2(middle.as_ref()),
                )));
            }
            x += dim.get_width();
        }
        ftile_utils::add_connections(result, conns)
    }
}

/// The arrow from a flow into the bar below it.
struct ConnectionOut {
    base: AbstractConnection,
    skin_param: Rc<SkinParam>,
    x: f64,
    arrow_color: Rainbow,
    label: Option<Display>,
    just_before_bar2: f64,
}

impl ConnectionOut {
    fn new(
        skin_param: Rc<SkinParam>,
        ftile1: Rc<dyn Ftile>,
        ftile2: Rc<dyn Ftile>,
        x: f64,
        arrow_color: Rainbow,
        just_before_bar2: f64,
    ) -> Self {
        let label = ftile1.get_out_link_rendering().display;
        Self {
            base: AbstractConnection::new(Some(ftile1), Some(ftile2)),
            skin_param,
            x,
            arrow_color,
            label,
            just_before_bar2,
        }
    }

    fn snake(&self) -> Snake {
        Snake::create_with_end(
            &self.skin_param,
            self.arrow_color.clone(),
            self.skin_param.arrows().as_to_down(),
        )
    }

    fn with_label(&self, snake: Snake) -> Snake {
        snake.with_label(
            get_text_block(&self.skin_param, self.label.as_ref()),
            self.base.arrow_horizontal_alignment(),
        )
    }

    /// Where the arrow starts and ends, unless the flow ends the flow.
    fn points(&self, string_bounder: &dyn StringBounder) -> Option<(XPoint2D, XPoint2D)> {
        let geo1 = self
            .base
            .get_ftile1()
            .expect("an arrow out of a flow has the flow")
            .calculate_dimension(string_bounder);
        if !geo1.has_point_out() {
            return None;
        }
        Some((
            XPoint2D::new(geo1.get_left(), BAR_HEIGHT + geo1.get_out_y()),
            XPoint2D::new(geo1.get_left(), self.just_before_bar2),
        ))
    }
}

impl Connection for ConnectionOut {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.apply(UTranslate::new(self.x, 0.0));
        let Some((p1, p2)) = self.points(ug.string_bounder()) else {
            return;
        };
        let mut snake = self.with_label(self.snake());
        snake.add_point_at(p1);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionOut {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let ug = ug.apply(UTranslate::new(self.x, 0.0));
        let Some((p1, p2)) = self.points(ug.string_bounder()) else {
            return;
        };
        let mut snake = self.with_label(self.snake().ignore_for_compression());
        let start = translate1.get_translated(p1);
        let end = translate2.get_translated(p2);
        let middle = end.y - 14.0;
        snake.add_point_at(start);
        snake.add_point(start.x, middle);
        snake.add_point(end.x, middle);
        snake.add_point_at(end);
        ug.draw(&snake);
    }
}
