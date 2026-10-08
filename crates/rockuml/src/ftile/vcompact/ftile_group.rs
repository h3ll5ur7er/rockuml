//! A group, partition, package, rectangle or card around tiles, its name at the top (PlantUML's
//! `FtileGroup`).

use std::cell::OnceCell;
use std::rc::Rc;

use crate::color::HColor;
use crate::creole::{CreoleMode, Display};
use crate::decoration::symbol::USymbol;
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, FtileMarged, Swimable};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, UTranslate};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::ugraphic_dispatch_drawable::UGraphicDispatchDrawable;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::style::{PName, Style, ValueReading};
use crate::svek::UGraphicForSnake;

/// The room below the tiles.
const DIFF_YY2: f64 = 20.0;
/// The room left and right of the tiles.
const MARGIN: f64 = 10.0;

pub(crate) struct FtileGroup {
    base: AbstractFtile,
    inner: Rc<dyn Ftile>,
    name: Rc<dyn TextBlock>,
    border_color: HColor,
    back_color: HColor,
    stroke: UStroke,
    type_: USymbol,
    round_corner: f64,
    cached_inner_dimension: OnceCell<FtileGeometry>,
}

impl FtileGroup {
    /// `inner` framed as `type_` says, which must have a big form; `skin_param` is the one `inner` is
    /// drawn with.
    pub(crate) fn new(
        inner: Rc<dyn Ftile>,
        title: &Display,
        back_color: Option<HColor>,
        skin_param: Rc<SkinParam>,
        type_: USymbol,
        style: &Style,
    ) -> Self {
        let font_configuration = style.font_configuration();
        let name = title.create0(
            &font_configuration,
            HorizontalAlignment::Left,
            skin_param.as_ref(),
            0.0,
            CreoleMode::Full,
        );
        Self {
            inner: Rc::new(FtileMarged::new(inner, skin_param.clone(), MARGIN, MARGIN)),
            base: AbstractFtile::new(skin_param),
            name: Rc::new(name),
            border_color: style.value(PName::LineColor).as_color(),
            back_color: back_color
                .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            stroke: style.stroke(),
            type_,
            round_corner: style.value(PName::RoundCorner).as_double(),
            cached_inner_dimension: OnceCell::new(),
        }
    }

    fn diff_height_title(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim_title = self.name.calculate_dimension(string_bounder);
        f64::max(25.0, dim_title.height + 20.0)
    }

    fn get_translate(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        UTranslate::new(
            self.supp_width(string_bounder) / 2.0,
            self.diff_height_title(string_bounder),
        )
    }

    /// How far the tiles reach when drawn, arrows included.
    fn get_inner_min_max(&self, string_bounder: &dyn StringBounder) -> MinMax {
        let (limit_finder, min_max) =
            LimitFinder::surface(string_bounder.shared(), MinMax::empty());
        let interceptor = UGraphicForSnake::create(limit_finder);
        let interceptor2 = UGraphicDispatchDrawable::create(interceptor);
        self.inner.draw_u(&interceptor2);
        interceptor2.flush_ug();
        min_max.borrow().min_max()
    }

    /// The width the title needs beyond the tiles'.
    fn supp_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        let orig = self.get_inner_dimension(string_bounder);
        let dim_title = self.name.calculate_dimension(string_bounder);
        f64::max(orig.get_width(), dim_title.width + 20.0) - orig.get_width()
    }

    fn get_inner_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self
            .cached_inner_dimension
            .get_or_init(|| self.get_inner_dimension_slow(string_bounder))
    }

    /// The tiles' size, widened to hold whatever they draw beyond it, like arrow labels.
    fn get_inner_dimension_slow(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let orig = self.inner.calculate_dimension(string_bounder);
        let min_max = self.get_inner_min_max(string_bounder);
        let missing_width = min_max.max_x() - orig.get_width();
        if missing_width > 0.0 {
            return orig.add_dim(missing_width + 5.0, 0.0);
        }
        orig
    }
}

impl Swimable for FtileGroup {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.inner.get_swimlanes()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.inner.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.inner.get_swimlane_out()
    }
}

impl Ftile for FtileGroup {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.inner.get_in_link_rendering()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let orig = self.get_inner_dimension(string_bounder);
            let supp_width = self.supp_width(string_bounder);
            let width = orig.get_width() + supp_width;
            let title_height = self.diff_height_title(string_bounder);
            let height = orig.get_height() + title_height + DIFF_YY2;
            let left = orig.get_left() + supp_width / 2.0;
            let in_y = orig.get_in_y() + title_height;
            if orig.has_point_out() {
                return FtileGeometry::with_out(
                    width,
                    height,
                    left,
                    in_y,
                    orig.get_out_y() + title_height,
                );
            }
            FtileGeometry::new(width, height, left, in_y)
        })
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.inner.get_my_children()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let fashion = Fashion::new(self.back_color.clone(), self.border_color.clone())
            .with_stroke(self.stroke)
            .with_corner(self.round_corner, 0.0);
        let frame = self.type_.as_big(
            self.name.clone(),
            self.inner.skin_param().package_title_alignment(),
            Rc::new(TextBlockEmpty::default()),
            dim_total.get_width(),
            dim_total.get_height(),
            fashion,
            self.skin_param().stereotype_alignment(),
        );
        if let Some(frame) = frame {
            frame.draw_u(ug);
        }
        ug.apply(self.get_translate(string_bounder))
            .draw(&self.inner);
    }
}
