//! What the builders of forks, merges and splits share (PlantUML's `AbstractParallelFtilesBuilder`): the
//! flows, padded to stand side by side, and the arrows into them.

use std::rc::Rc;

use crate::color::Colors;
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::diagram::activity3::SwimlaneId;
use crate::ftile::{
    AbstractConnection, Connection, ConnectionTranslatable, Ftile, FtileHeightFixedCentered,
    FtileHeightFixedMarged, Snake, ftile_utils,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{SName, Style, StyleSignature};

/// How high the bars of a fork are (`barHeight`).
pub(super) const BAR_HEIGHT: f64 = 6.0;

/// Builds the tile of flows side by side around the tile of the flows themselves (`build`).
pub(super) trait ParallelFtilesBuilder {
    /// The flows, padded (`list99`); the factory lays them side by side.
    fn list99(&self) -> &[Rc<dyn Ftile>];

    /// What leads into the flows.
    fn do_step1(&self, inner: &Rc<dyn Ftile>) -> Rc<dyn Ftile>;

    /// What leads out of the flows, below `step1`.
    fn do_step2(&self, inner: &Rc<dyn Ftile>, step1: Rc<dyn Ftile>) -> Rc<dyn Ftile>;

    fn build(&self, inner: &Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let step1 = self.do_step1(inner);
        self.do_step2(inner, step1)
    }
}

pub(super) struct AbstractParallelFtilesBuilder<'a> {
    pub(super) skin_param: Rc<SkinParam>,
    pub(super) string_bounder: &'a dyn StringBounder,
    pub(super) list99: Vec<Rc<dyn Ftile>>,
    pub(super) colors: Colors,
}

impl<'a> AbstractParallelFtilesBuilder<'a> {
    pub(super) fn new(
        skin_param: Rc<SkinParam>,
        string_bounder: &'a dyn StringBounder,
        all: &[Rc<dyn Ftile>],
        colors: Colors,
    ) -> Self {
        let mut builder = Self {
            skin_param,
            string_bounder,
            list99: Vec::new(),
            colors,
        };
        builder.list99 = builder.decorate_all_tiles(all);
        builder
    }

    fn decorate_all_tiles(&self, all: &[Rc<dyn Ftile>]) -> Vec<Rc<dyn Ftile>> {
        let max_height = self.compute_max_height(all);
        let ymargin1 = self.get_supp_space(all, |tile| tile.get_in_link_rendering().display);
        let ymargin2 = self.get_supp_space(all, |tile| tile.get_out_link_rendering().display);
        all.iter()
            .map(|ftile| self.compute_new_ftile(ftile, max_height, ymargin1, ymargin2))
            .collect()
    }

    /// The height of the highest label `label` gives the flows (`getSuppSpace1`, `getSuppSpace2`).
    fn get_supp_space(
        &self,
        all: &[Rc<dyn Ftile>],
        label: impl Fn(&dyn Ftile) -> Option<Display>,
    ) -> f64 {
        let mut result: f64 = 0.0;
        for child in all {
            let Some(text) = self.get_text_block(label(child.as_ref()).as_ref()) else {
                continue;
            };
            result = result.max(text.calculate_dimension(self.string_bounder).height);
        }
        result
    }

    fn compute_new_ftile(
        &self,
        ftile: &Rc<dyn Ftile>,
        max_height: f64,
        ymargin1: f64,
        ymargin2: f64,
    ) -> Rc<dyn Ftile> {
        let space_arround_black_bar = 20.0;
        let x_margin = 14.0;
        let tmp = ftile_utils::add_horizontal_margin(
            ftile.clone(),
            x_margin,
            x_margin + self.get_supp_for_incoming_arrow(ftile.as_ref()),
        );
        let tmp = Rc::new(FtileHeightFixedCentered::new(
            tmp,
            max_height + 2.0 * space_arround_black_bar,
        ));
        Rc::new(FtileHeightFixedMarged::new(ymargin1, tmp, ymargin2))
    }

    fn get_supp_for_incoming_arrow(&self, ftile: &dyn Ftile) -> f64 {
        let x1 = self.get_x_supp_for_display(ftile, ftile.get_in_link_rendering().display.as_ref());
        let x2 =
            self.get_x_supp_for_display(ftile, ftile.get_out_link_rendering().display.as_ref());
        x1.max(x2)
    }

    /// How far `label`, written from where arrows enter `ftile`, sticks out on its right.
    fn get_x_supp_for_display(&self, ftile: &dyn Ftile, label: Option<&Display>) -> f64 {
        let Some(text) = self.get_text_block(label) else {
            return 0.0;
        };
        let text_width = text.calculate_dimension(self.string_bounder).width;
        let ftile_dim = ftile.calculate_dimension(self.string_bounder);
        let pos2 = ftile_dim.get_left() + text_width;
        if pos2 > ftile_dim.get_width() {
            return pos2 - ftile_dim.get_width();
        }
        0.0
    }

    fn compute_max_height(&self, all: &[Rc<dyn Ftile>]) -> f64 {
        all.iter().fold(0.0, |height: f64, tmp| {
            height.max(tmp.calculate_dimension(self.string_bounder).get_height())
        })
    }

    pub(super) fn get_style_signature_arrow() -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
    }

    /// The style `signature` gives in this diagram.
    pub(super) fn merged_style(&self, signature: &StyleSignature) -> Style {
        self.skin_param
            .merged_style(signature)
            .expect("the skin styles activity diagrams")
    }

    pub(super) fn get_text_block(&self, display: Option<&Display>) -> Option<Rc<dyn TextBlock>> {
        get_text_block(&self.skin_param, display)
    }

    pub(super) fn get_height_of_middle(&self, middle: &dyn Ftile) -> f64 {
        middle.calculate_dimension(self.string_bounder).get_height()
    }

    /// The lane the arrows out of the flows end in (`swimlaneOutForStep2`).
    pub(super) fn swimlane_out_for_step2(&self) -> Option<SwimlaneId> {
        self.list99
            .last()
            .expect("a fork has a first flow")
            .get_swimlane_out()
    }
}

/// An arrow label in the arrow style's font; none without a label (`getTextBlock`).
pub(super) fn get_text_block(
    skin_param: &SkinParam,
    display: Option<&Display>,
) -> Option<Rc<dyn TextBlock>> {
    let display = display?;
    let style = skin_param
        .merged_style(&AbstractParallelFtilesBuilder::get_style_signature_arrow())
        .expect("the skin styles activity arrows");
    Some(Rc::new(display.create0(
        &style.font_configuration(),
        HorizontalAlignment::Left,
        skin_param,
        0.0,
        CreoleMode::SimpleLine,
    )))
}

/// The arrow from the bar into a flow, `x` across (each builder's `ConnectionIn`, all alike but for
/// compressing an arrow across lanes, which only forks leave out).
pub(super) struct ConnectionIn {
    base: AbstractConnection,
    skin_param: Rc<SkinParam>,
    x: f64,
    arrow_color: Rainbow,
    label: Option<Display>,
    ignore_for_compression_across_lanes: bool,
}

impl ConnectionIn {
    pub(super) fn new(
        skin_param: Rc<SkinParam>,
        ftile1: Rc<dyn Ftile>,
        ftile2: Rc<dyn Ftile>,
        x: f64,
        arrow_color: Rainbow,
        ignore_for_compression_across_lanes: bool,
    ) -> Self {
        let label = ftile2.get_in_link_rendering().display;
        Self {
            base: AbstractConnection::new(Some(ftile1), Some(ftile2)),
            skin_param,
            x,
            arrow_color,
            label,
            ignore_for_compression_across_lanes,
        }
    }

    fn ftile2(&self) -> &Rc<dyn Ftile> {
        self.base
            .get_ftile2()
            .expect("an arrow into a flow has the flow")
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
}

impl Connection for ConnectionIn {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug.apply(UTranslate::new(self.x, 0.0));
        let geo2 = self.ftile2().calculate_dimension(ug.string_bounder());
        let mut snake = self.with_label(self.snake());
        snake.add_point(geo2.get_left(), 0.0);
        snake.add_point(geo2.get_left(), geo2.get_in_y());
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionIn {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let ug = ug.apply(UTranslate::new(self.x, 0.0));
        let geo2 = self.ftile2().calculate_dimension(ug.string_bounder());
        let p1 = XPoint2D::new(geo2.get_left(), 0.0);
        let p2 = XPoint2D::new(geo2.get_left(), geo2.get_in_y());
        let snake = self.snake();
        let snake = if self.ignore_for_compression_across_lanes {
            snake.ignore_for_compression()
        } else {
            snake
        };
        let mut snake = self.with_label(snake);
        let start = translate1.get_translated(p1);
        let end = translate2.get_translated(p2);
        let middle = start.y + 4.0;
        snake.add_point_at(start);
        snake.add_point(start.x, middle);
        snake.add_point(end.x, middle);
        snake.add_point_at(end);
        ug.draw(&snake);
    }
}
