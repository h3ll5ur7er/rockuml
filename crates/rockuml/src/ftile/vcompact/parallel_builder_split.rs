//! PlantUML's `ParallelBuilderSplit`: flows between two thin lines; with no flow going on, the split
//! ends the flow.

use std::rc::Rc;

use super::abstract_parallel_ftiles_builder::{
    AbstractParallelFtilesBuilder, ConnectionIn, ParallelFtilesBuilder, get_text_block,
};
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::ftile::vertical::FtileThinSplit;
use crate::ftile::{
    AbstractConnection, Connection, ConnectionTranslatable, Ftile, FtileAssemblySimple,
    FtileKilled, Snake, ftile_utils,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::style::Style;

pub(super) struct ParallelBuilderSplit<'a> {
    base: AbstractParallelFtilesBuilder<'a>,
}

impl<'a> ParallelBuilderSplit<'a> {
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

    /// Splits are styled as arrows (`getStyleSignature`).
    fn style(&self) -> Style {
        self.base
            .merged_style(&AbstractParallelFtilesBuilder::get_style_signature_arrow())
    }

    /// The colour of the first line, unless all arrows into the flows are hidden.
    fn get_thin1_color(&self, thin_color: &Rainbow) -> Option<HColor> {
        let def = Rainbow::build_from_style(&self.style());
        self.base
            .list99
            .iter()
            .any(|tmp| {
                !tmp.get_in_link_rendering()
                    .get_rainbow_or(&def)
                    .is_invisible()
            })
            .then(|| thin_color.get_color().clone())
    }

    fn has_out(&self) -> bool {
        self.base.list99.iter().any(|tmp| {
            tmp.calculate_dimension(self.base.string_bounder)
                .has_point_out()
        })
    }
}

impl ParallelFtilesBuilder for ParallelBuilderSplit<'_> {
    fn list99(&self) -> &[Rc<dyn Ftile>] {
        &self.base.list99
    }

    fn do_step1(&self, inner: &Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let base = &self.base;
        let def = Rainbow::build_from_style(&self.style());
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let mut x = 0.0;
        let mut first = 0.0;
        let mut last = 0.0;
        // The line goes from the first flow to the last, and the arrows from it into the flows; it is
        // built before them here, PlantUML places it after.
        let mut entries = Vec::new();
        for tmp in &base.list99 {
            let dim = tmp.calculate_dimension(base.string_bounder);
            if first == 0.0 {
                first = x + dim.get_left();
            }
            last = x + dim.get_left();
            entries.push((tmp, x, tmp.get_in_link_rendering().get_rainbow_or(&def)));
            x += dim.get_width();
        }
        // Arrows leave a tile's geometry as it is.
        let geom = inner.calculate_dimension(base.string_bounder);
        if last < geom.get_left() {
            last = geom.get_left();
        }
        if first > geom.get_left() {
            first = geom.get_left();
        }
        let thin: Rc<dyn Ftile> = Rc::new(FtileThinSplit::new(
            base.skin_param.clone(),
            self.get_thin1_color(&def),
            base.list99[0].get_swimlane_in(),
            first,
            last,
            geom.get_width(),
        ));
        for (tmp, x, rainbow) in entries {
            conns.push(Rc::new(ConnectionIn::new(
                base.skin_param.clone(),
                thin.clone(),
                tmp.clone(),
                x,
                rainbow,
                false,
            )));
        }
        let result = ftile_utils::add_connections(inner.clone(), conns);
        Rc::new(FtileAssemblySimple::new(thin, result))
    }

    fn do_step2(&self, inner: &Rc<dyn Ftile>, result: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let base = &self.base;
        let geom = result.calculate_dimension(base.string_bounder);
        if !self.has_out() {
            return Rc::new(FtileKilled::new(result));
        }
        let def = Rainbow::build_from_style(&self.style());
        let thin_color = result.get_in_link_rendering().get_rainbow_or(&def);
        let mut x = 0.0;
        let mut first = 0.0;
        let mut last = 0.0;
        let mut exits = Vec::new();
        for tmp in &base.list99 {
            let dim = tmp.calculate_dimension(base.string_bounder);
            if dim.has_point_out() {
                if first == 0.0 {
                    first = x + dim.get_left();
                }
                last = x + dim.get_left();
                exits.push((tmp, x, tmp.get_out_link_rendering().get_rainbow_or(&def)));
            }
            x += dim.get_width();
        }
        if last < geom.get_left() {
            last = geom.get_left();
        }
        if first > geom.get_left() {
            first = geom.get_left();
        }
        let out: Rc<dyn Ftile> = Rc::new(FtileThinSplit::new(
            base.skin_param.clone(),
            Some(thin_color.get_color().clone()),
            base.swimlane_out_for_step2(),
            first,
            last,
            geom.get_width(),
        ));
        let result: Rc<dyn Ftile> = Rc::new(FtileAssemblySimple::new(result, out.clone()));
        let height = base.get_height_of_middle(inner.as_ref());
        let conns: Vec<Rc<dyn Connection>> = exits
            .into_iter()
            .map(|(tmp, x, rainbow)| -> Rc<dyn Connection> {
                Rc::new(ConnectionOut::new(
                    base.skin_param.clone(),
                    UTranslate::new(0.0, 1.5),
                    tmp.clone(),
                    out.clone(),
                    x,
                    rainbow,
                    height,
                ))
            })
            .collect();
        ftile_utils::add_connections(result, conns)
    }
}

/// The arrow from a flow into the line below it.
struct ConnectionOut {
    base: AbstractConnection,
    skin_param: Rc<SkinParam>,
    x: f64,
    arrow_color: Rainbow,
    height: f64,
    label: Option<Display>,
    translate0: UTranslate,
}

impl ConnectionOut {
    fn new(
        skin_param: Rc<SkinParam>,
        translate0: UTranslate,
        ftile1: Rc<dyn Ftile>,
        ftile2: Rc<dyn Ftile>,
        x: f64,
        arrow_color: Rainbow,
        height: f64,
    ) -> Self {
        let label = ftile1.get_out_link_rendering().display;
        Self {
            base: AbstractConnection::new(Some(ftile1), Some(ftile2)),
            skin_param,
            x,
            arrow_color,
            height,
            label,
            translate0,
        }
    }

    fn snake(&self) -> Snake {
        Snake::create_with_end(
            &self.skin_param,
            self.arrow_color.clone(),
            self.skin_param.arrows().as_to_down(),
        )
        .with_label(
            get_text_block(&self.skin_param, self.label.as_ref()),
            self.base.arrow_horizontal_alignment(),
        )
    }

    /// Where the arrow starts and ends, unless the flow ends the flow.
    fn points(&self, string_bounder: &dyn StringBounder) -> Option<(XPoint2D, XPoint2D)> {
        let geo = self
            .base
            .get_ftile1()
            .expect("an arrow out of a flow has the flow")
            .calculate_dimension(string_bounder);
        if !geo.has_point_out() {
            return None;
        }
        Some((
            self.translate0
                .get_translated(XPoint2D::new(geo.get_left(), geo.get_out_y())),
            self.translate0
                .get_translated(XPoint2D::new(geo.get_left(), self.height)),
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
        let mut snake = self.snake();
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
        let mut snake = self.snake();
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
