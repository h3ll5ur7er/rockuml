//! PlantUML's `FtileFactoryDelegatorRepeat`: builds `repeat` loops, and joins the `break`s inside to a
//! diamond below the loop.

use std::rc::Rc;

use super::ftile_repeat::{FtileRepeat, RepeatStyle};
use crate::color::{ColorType, Colors};
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::diagram::activity3::{LinkRendering, SwimlaneId};
use crate::direction::Direction;
use crate::ftile::vertical::FtileDiamond;
// Only the delegator's methods are in scope: the factory's are the same ones, through it.
use crate::ftile::{
    self, BoxStyle, Connection, Ftile, FtileFactoryDelegator, Genealogy, Snake, ftile_utils,
};
use crate::klimt::ugraphic::UGraphic;
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{PName, StyleBuilder, ValueReading};

pub(crate) struct FtileFactoryDelegatorRepeat {
    factory: Box<dyn ftile::FtileFactory>,
}

impl FtileFactoryDelegatorRepeat {
    pub(crate) fn new(factory: Box<dyn ftile::FtileFactory>) -> Self {
        Self { factory }
    }

    /// The activity of `repeat :label;`, which the loop enters instead of a diamond.
    fn get_entry(
        &self,
        swimlane: Option<SwimlaneId>,
        start_label: Option<&Display>,
        colors: &Colors,
        box_style_in: BoxStyle,
        stereotype: Option<&Stereotype>,
    ) -> Option<Rc<dyn Ftile>> {
        let start_label = start_label?;
        Some(self.activity(
            start_label,
            swimlane,
            box_style_in,
            colors,
            stereotype,
            &self.skin_param().current_style_builder(),
        ))
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorRepeat {
    fn get_factory(&self) -> &dyn ftile::FtileFactory {
        self.factory.as_ref()
    }

    fn repeat(
        &self,
        stereogroup1: &Stereogroup,
        stereogroup2: &Stereogroup,
        box_style_in: BoxStyle,
        swimlane: Option<SwimlaneId>,
        swimlane_out: Option<SwimlaneId>,
        start_label: Option<&Display>,
        repeat: Rc<dyn Ftile>,
        test: Option<&Display>,
        yes: Option<&Display>,
        out: Option<&Display>,
        backward: Option<Rc<dyn Ftile>>,
        no_out: bool,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
        current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        let skin_param = self.skin_param();
        let style_arrow = self
            .get_default_style_definition_arrow()
            .get_merged_style(current_style_builder);
        let diamond_signature = self.get_default_style_definition_diamond();
        let style_diamond = diamond_signature.get_merged_style(current_style_builder);

        let border_color = style_diamond.value(PName::LineColor).as_color();
        let diamond_color1 = stereogroup1.get_hcolor(
            &diamond_signature,
            PName::BackGroundColor,
            current_style_builder,
        );
        let diamond_color2 = if stereogroup2
            .get_inner_colors()
            .unwrap_or_default()
            .is_empty()
            && stereogroup2.is_empty()
        {
            diamond_color1.clone()
        } else {
            stereogroup2.get_hcolor(
                &diamond_signature,
                PName::BackGroundColor,
                current_style_builder,
            )
        };

        let colors_entry = Colors::default()
            .apply_style(
                &self
                    .get_default_style_definition_activity()
                    .get_merged_style(current_style_builder),
            )
            .with(ColorType::Back, Some(diamond_color1.clone()));

        let arrow_color = Rainbow::build_from_style(&style_arrow);
        let style = RepeatStyle {
            border_color: border_color.clone(),
            diamond_color1,
            diamond_color2: diamond_color2.clone(),
            arrow_color: arrow_color.clone(),
            end_repeat_link_color: repeat.get_out_link_rendering().rainbow,
            condition_style: skin_param.get_condition_style(),
            fc_diamond: style_diamond.font_configuration(),
            fc_arrow: style_arrow.font_configuration(),
        };

        let entry = self.get_entry(
            swimlane,
            start_label,
            &colors_entry,
            box_style_in,
            stereogroup1.build_stereotype().as_ref(),
        );

        let mut result = FtileRepeat::create(
            swimlane,
            swimlane_out,
            entry,
            repeat.clone(),
            test,
            yes,
            out,
            &style,
            skin_param,
            backward,
            no_out,
            incoming1,
            incoming2,
        );

        let welding_points = repeat.get_welding_points();
        if !welding_points.is_empty() {
            let diamond_break: Rc<dyn Ftile> = Rc::new(FtileDiamond::new(
                skin_param.clone(),
                diamond_color2,
                border_color,
                swimlane,
            ));
            result = self.assembly(
                ftile_utils::add_horizontal_margin(result, 10.0, 0.0),
                diamond_break.clone(),
            );
            let genealogy = Rc::new(Genealogy::new(&result));

            let connections = welding_points
                .into_iter()
                .enumerate()
                .map(|(i, ftile_break)| {
                    Rc::new(ConnectionBreak {
                        ftile_break,
                        diamond_break: diamond_break.clone(),
                        genealogy: genealogy.clone(),
                        arrow_color: arrow_color.clone(),
                        first: i == 0,
                    }) as Rc<dyn Connection>
                })
                .collect();
            result = ftile_utils::add_connections(result, connections);
        }
        result
    }
}

/// From a `break` left to the side of the loop, the first one then down into the diamond below it.
struct ConnectionBreak {
    ftile_break: Rc<dyn Ftile>,
    diamond_break: Rc<dyn Ftile>,
    genealogy: Rc<Genealogy>,
    arrow_color: Rainbow,
    first: bool,
}

impl Connection for ConnectionBreak {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        Some(&self.ftile_break)
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        Some(&self.diamond_break)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tr1 = self
            .genealogy
            .get_translate(self.ftile_break.as_ref(), string_bounder);
        let tr2 = self
            .genealogy
            .get_translate(self.diamond_break.as_ref(), string_bounder);
        let dim_diamond = self.diamond_break.calculate_dimension(string_bounder);

        let skin_param = self.diamond_break.skin_param();
        let direction = if self.first {
            Direction::Right
        } else {
            Direction::Left
        };
        let mut snake = Snake::create_with_end(
            skin_param,
            self.arrow_color.clone(),
            skin_param.arrows().as_to(direction),
        );
        snake.add_point(tr1.dx, tr1.dy);
        snake.add_point(0.0, tr1.dy);
        if self.first {
            let y = tr2.dy + dim_diamond.get_height() / 2.0;
            snake.add_point(0.0, y);
            snake.add_point(tr2.dx, y);
        }
        ug.draw(&snake);
    }
}
