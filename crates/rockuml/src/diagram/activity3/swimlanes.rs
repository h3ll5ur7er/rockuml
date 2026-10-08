//! The swimlanes of an activity diagram, and the state its commands build the instructions with: the
//! instruction that takes the next ones, the lane they go to, the arrow leading to the next one (the model
//! half of PlantUML's `Swimlanes`, and `Swimlane`).

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::instruction::{InstructionId, Instructions, SwimlaneSet};
use super::link_rendering::LinkRendering;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::ftile::vcompact::{
    self, UGraphicInterceptorAllSwimlanes, UGraphicInterceptorOneSwimlane, VCompactFactory,
};
use crate::ftile::{
    ConnectionCross, Ftile, FtileFactory, LaneDivider, TextBlockInterceptorUDrawable,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, UTranslate, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::shape::{CenteredText, URectangle, UShape};
use crate::klimt::ugraphic::{AnyShape, UChange, UGraphic, UGraphicLayer};
use crate::klimt::{HorizontalAlignment, TextBlock, UDrawable};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading, max_width};
use crate::svek::UGraphicForSnake;

/// A swimlane, known by its order of declaration, as tiles, connections and layers name it; PlantUML compares
/// lanes by identity. The lane PlantUML adds after the last while drawing, ordered after all, is
/// `SwimlaneId(usize::MAX)`. What laying the lanes out sets on PlantUML's `Swimlane` (translation, width,
/// extent) is kept by the layout, keyed by these ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SwimlaneId(pub(crate) usize);

impl SwimlaneId {
    /// The lane drawing adds after the last (the last of `swimlanesSpecial`): it holds no tile.
    const LAST: Self = Self(usize::MAX);

    /// Whether this lane comes before all of `others`, which must not be this lane alone
    /// (`isSmallerThanAllOthers`). PlantUML fails on a set holding no lane; such a member is passed over.
    pub(crate) fn is_smaller_than_all_others(self, others: &SwimlaneSet) -> bool {
        if others.len() == 1 && others.contains(&Some(self)) {
            return false;
        }
        others.iter().flatten().all(|other| *other >= self)
    }
}

pub(crate) struct Swimlane {
    pub(crate) name: String,
    pub(crate) order: SwimlaneId,
    pub(crate) display: Display,
    pub(crate) colors: Colors,
}

pub(crate) struct Swimlanes {
    lanes: Vec<Swimlane>,
    current_swimlane: Option<SwimlaneId>,
    /// Every instruction, the root list first.
    pub(crate) instructions: Instructions,
    current_instruction: InstructionId,
    next_link_renderer: LinkRendering,
}

impl Swimlanes {
    pub(crate) fn new() -> Self {
        Self {
            lanes: Vec::new(),
            current_swimlane: None,
            instructions: Instructions::new(),
            current_instruction: Instructions::ROOT,
            next_link_renderer: LinkRendering::none(),
        }
    }

    /// Makes the lane `name` current, declaring it first if needed, and colours or labels it.
    pub(crate) fn swimlane(&mut self, name: &str, color: Option<HColor>, label: Option<Display>) {
        let id = self.get_or_create(name);
        self.current_swimlane = Some(id);
        let lane = &mut self.lanes[id.0];
        lane.colors = lane.colors.with(ColorType::Back, color);
        if let Some(label) = label {
            lane.display = label;
        }
    }

    fn get_or_create(&mut self, name: &str) -> SwimlaneId {
        if let Some(lane) = self.lanes.iter().find(|lane| lane.name == name) {
            return lane.order;
        }
        let order = SwimlaneId(self.lanes.len());
        self.lanes.push(Swimlane {
            name: name.to_owned(),
            order,
            display: Display::with_newlines(name),
            colors: Colors::default(),
        });
        order
    }

    /// The lanes in their order of declaration.
    pub(crate) fn swimlanes(&self) -> &[Swimlane] {
        &self.lanes
    }

    pub(crate) fn get_current(&self) -> InstructionId {
        self.current_instruction
    }

    pub(crate) fn set_current(&mut self, current: InstructionId) {
        self.current_instruction = current;
    }

    pub(crate) fn next_link_renderer(&self) -> &LinkRendering {
        &self.next_link_renderer
    }

    pub(crate) fn set_next_link_renderer(&mut self, link: LinkRendering) {
        self.next_link_renderer = link;
    }

    pub(crate) fn get_current_swimlane(&self) -> Option<SwimlaneId> {
        self.current_swimlane
    }
}

/// The drawing half of PlantUML's `Swimlanes`: the tiles built from the instructions, drawn in the lanes.
/// With more than one lane, the lanes are laid out once (`ensureSizeComputed`), then the tile tree is
/// drawn once per lane, keeping what lies in it, and once more for the arrows crossing lanes.
pub(super) struct SwimlanesDrawing<'a> {
    swimlanes: &'a Swimlanes,
    skin_param: Rc<SkinParam>,
    /// `!pragma useVerticalIf on`.
    use_vertical_if: bool,
    string_bounder: Rc<dyn StringBounder>,
    style: Style,
    /// The lane after the last, as `swimlanesSpecial` adds it.
    last: Swimlane,
    layout: OnceCell<Rc<SwimlanesLayout>>,
    cached_min_max: OnceCell<MinMax>,
}

/// Where laying the lanes out put them (what `computeSizeInternal` sets on PlantUML's `Swimlane`s), and
/// the dividers between them.
struct SwimlanesLayout {
    /// Every lane, then the lane after the last.
    lanes: BTreeMap<SwimlaneId, LaneLayout>,
    /// The divider left of each lane of `lanes`, in their order.
    dividers: Vec<LaneDivider>,
}

struct LaneLayout {
    /// How far the lane's tiles reach, before it is moved into place.
    min_max: MinMax,
    actual_width: f64,
    translate: UTranslate,
}

impl LaneLayout {
    /// `getWidthWithoutTitle`.
    fn get_width_without_title(&self) -> f64 {
        self.min_max.dimension().width
    }
}

impl SwimlanesLayout {
    fn translate(&self, swimlane: SwimlaneId) -> UTranslate {
        self.lanes
            .get(&swimlane)
            .map_or_else(UTranslate::default, |lane| lane.translate)
    }
}

impl<'a> SwimlanesDrawing<'a> {
    pub(super) fn new(
        swimlanes: &'a Swimlanes,
        skin_param: Rc<SkinParam>,
        use_vertical_if: bool,
        string_bounder: Rc<dyn StringBounder>,
    ) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Swimlane,
        ])
        .get_merged_style(&skin_param.current_style_builder());
        Self {
            swimlanes,
            skin_param,
            use_vertical_if,
            string_bounder,
            style,
            last: Swimlane {
                name: String::new(),
                order: SwimlaneId::LAST,
                display: Display::with_newlines(""),
                colors: Colors::default(),
            },
            layout: OnceCell::new(),
            cached_min_max: OnceCell::new(),
        }
    }

    fn get_ftile_factory(&self) -> Box<dyn FtileFactory> {
        vcompact::delegator_chain(
            Box::new(VCompactFactory::new(
                self.skin_param.clone(),
                self.string_bounder.clone(),
            )),
            self.use_vertical_if,
        )
    }

    fn create_ftile(&self) -> Rc<dyn Ftile> {
        let factory = self.get_ftile_factory();
        self.swimlanes
            .instructions
            .create_ftile(Instructions::ROOT, factory.as_ref())
    }

    fn get_min_max(&self) -> MinMax {
        *self
            .cached_min_max
            .get_or_init(|| LimitFinder::min_max_of(self, self.string_bounder.clone()))
    }

    /// The lanes, then the lane after the last (`swimlanesSpecial`).
    fn swimlanes_special(&self) -> impl Iterator<Item = &Swimlane> {
        self.swimlanes.swimlanes().iter().chain([&self.last])
    }

    fn ensure_size_computed(&self) -> Rc<SwimlanesLayout> {
        self.layout
            .get_or_init(|| Rc::new(self.compute_size_internal(&self.create_ftile())))
            .clone()
    }

    fn draw_when_swimlanes(
        &self,
        ug: &UGraphic,
        full: &Rc<dyn Ftile>,
        layout: &Rc<SwimlanesLayout>,
    ) {
        let string_bounder = ug.string_bounder();
        let title_height_translate = self.get_title_height_translate(string_bounder, layout);

        self.draw_titles_background(ug, layout);

        let dimension_full = full.calculate_dimension(string_bounder);
        let ordered_list: Rc<[SwimlaneId]> = self
            .swimlanes
            .swimlanes()
            .iter()
            .map(|swimlane| swimlane.order)
            .collect();
        for (i, swimlane) in self.swimlanes_special().enumerate() {
            let lane = &layout.lanes[&swimlane.order];
            let divider1 = &layout.dividers[i];

            let xpos = lane.translate.dx + lane.min_max.min_x();
            if let Some(back) = swimlane.colors.get(ColorType::Back)
                && !back.is_transparent()
            {
                // Only declared lanes have colours, so a divider follows.
                let divider2 = &layout.dividers[i + 1];
                let background = ug
                    .with_backcolor(back.clone())
                    .with_color(back.clone())
                    .apply(UTranslate::new(xpos - divider1.get_x2(), 0.0));
                let width = lane.actual_width + divider1.get_x2() + divider2.get_x1();
                let height = dimension_full.get_height() + title_height_translate.dy;
                background.draw(&UShape::Rectangle(
                    URectangle::new(width, height)
                        .ignore_for_compression_on_x()
                        .ignore_for_compression_on_y(),
                ));
            }

            full.draw_u(
                &UGraphicInterceptorOneSwimlane::create(
                    ug.clone(),
                    swimlane.order,
                    ordered_list.clone(),
                )
                .apply(lane.translate)
                .apply(title_height_translate),
            );

            let divider_width = divider1.calculate_dimension(string_bounder).width;
            divider1.draw_u(&ug.apply(UTranslate::new(xpos - divider_width, 0.0)));
        }

        let cross = Cross::create(ug.apply(title_height_translate), layout.clone());
        full.draw_u(&cross);
        cross.flush_ug();

        self.draw_titles(ug, layout);
    }

    fn draw_titles_background(&self, ug: &UGraphic, layout: &SwimlanesLayout) {
        let color = self.style.value(PName::BackGroundColor).as_color();
        let title_height = self.get_titles_height(ug.string_bounder(), layout);
        let full_width = layout.translate(SwimlaneId::LAST).dx - 2.0 * 5.0 - 1.0;
        let back = URectangle::new(full_width, title_height)
            .ignore_for_compression_on_x()
            .ignore_for_compression_on_y();
        ug.apply(UTranslate::new(5.0, 0.0))
            .with_backcolor(color.clone())
            .with_color(color)
            .draw(&UShape::Rectangle(back));
    }

    fn draw_titles(&self, ug: &UGraphic, layout: &SwimlanesLayout) {
        for swimlane in self.swimlanes.swimlanes() {
            let lane = &layout.lanes[&swimlane.order];
            let sw_title = self.get_title(swimlane, lane.actual_width);
            let x2 = lane.translate.dx + lane.min_max.min_x();
            let centered_text = CenteredText {
                text: Rc::new(sw_title),
                total_width: lane.get_width_without_title(),
            };
            ug.apply(UTranslate::new(x2, 0.0))
                .draw(&UShape::CenteredText(centered_text));
        }
    }

    fn get_title(&self, swimlane: &Swimlane, actual_width: f64) -> SheetBlock2 {
        swimlane.display.create0(
            &self.style.font_configuration(),
            HorizontalAlignment::Left,
            self.skin_param.as_ref(),
            self.get_wrap(actual_width),
            CreoleMode::Full,
        )
    }

    /// How wide a title may grow before it wraps, `auto` meaning as wide as its lane. PlantUML means to fall
    /// back on the style's `MaximumWidth` without the skinparam, but compares with `LineBreakStrategy.NONE`
    /// by identity, which a fresh strategy never is, so the style never applies.
    fn get_wrap(&self, actual_width: f64) -> f64 {
        let wrap = self
            .skin_param
            .swimlane_wrap_title_width()
            .unwrap_or_default();
        if wrap.eq_ignore_ascii_case("auto") {
            return actual_width.trunc();
        }
        max_width(&wrap)
    }

    fn get_title_height_translate(
        &self,
        string_bounder: &dyn StringBounder,
        layout: &SwimlanesLayout,
    ) -> UTranslate {
        let titles_height = self.get_titles_height(string_bounder, layout);
        UTranslate::new(
            0.0,
            if titles_height > 0.0 {
                titles_height + 5.0
            } else {
                0.0
            },
        )
    }

    fn get_titles_height(
        &self,
        string_bounder: &dyn StringBounder,
        layout: &SwimlanesLayout,
    ) -> f64 {
        let mut titles_height: f64 = 0.0;
        for swimlane in self.swimlanes.swimlanes() {
            let sw_title = self.get_title(swimlane, layout.lanes[&swimlane.order].actual_width);
            titles_height = titles_height.max(sw_title.calculate_dimension(string_bounder).height);
        }
        titles_height
    }

    /// How far the tiles of each lane reach, measured in one walk of the tree.
    fn compute_drawing_widths(&self, full: &Rc<dyn Ftile>) -> Vec<MinMax> {
        let mut finders = Vec::new();
        let mut swimlane_to_ug = Vec::new();
        for swimlane in self.swimlanes.swimlanes() {
            let (ug, limit_finder) =
                LimitFinder::surface(self.string_bounder.clone(), MinMax::empty());
            finders.push(limit_finder);
            swimlane_to_ug.push((swimlane.order, UGraphicForSnake::create(ug)));
        }
        let interceptor = UGraphicInterceptorAllSwimlanes::create(swimlane_to_ug);
        full.draw_u(&interceptor);
        interceptor.flush_ug();
        finders
            .iter()
            .map(|limit_finder| limit_finder.borrow().min_max())
            .collect()
    }

    fn compute_size_internal(&self, full: &Rc<dyn Ftile>) -> SwimlanesLayout {
        let string_bounder = self.string_bounder.as_ref();
        let mut min_maxes = self.compute_drawing_widths(full);
        min_maxes.push(MinMax::from_origin());

        let mut min = self.skin_param.swimlane_width();
        if min == SkinParam::SWIMLANE_WIDTH_SAME {
            for min_max in &min_maxes[..min_maxes.len() - 1] {
                min = min.max(min_max.dimension().width);
            }
        }

        let lanes = self
            .swimlanes_special()
            .zip(min_maxes)
            .map(|(swimlane, min_max)| {
                let lane = LaneLayout {
                    min_max,
                    actual_width: min.max(min_max.dimension().width),
                    translate: UTranslate::default(),
                };
                (swimlane.order, lane)
            })
            .collect();
        let mut layout = SwimlanesLayout {
            lanes,
            dividers: Vec::new(),
        };

        let title_height_translate = self.get_title_height_translate(string_bounder, &layout);
        let dimension_full = full.calculate_dimension(string_bounder);

        let special: Vec<&Swimlane> = self.swimlanes_special().collect();
        let mut translates = Vec::new();
        let mut xpos = 0.0;
        for (i, swimlane) in special.iter().enumerate() {
            let x1 = self.get_half_missing_space(string_bounder, &special, &layout, i, min);
            let x2 = self.get_half_missing_space(string_bounder, &special, &layout, i + 1, min);
            let lane_divider = LaneDivider::new(
                &self.skin_param,
                x1,
                x2,
                dimension_full.get_height() + title_height_translate.dy,
            );
            let lane = &layout.lanes[&swimlane.order];
            let xx = xpos + lane_divider.get_width() - lane.min_max.min_x()
                + (lane.actual_width - lane.get_width_without_title()) / 2.0;
            translates.push(UTranslate::new(xx, 0.0));
            xpos += lane.actual_width + lane_divider.get_width();
            layout.dividers.push(lane_divider);
        }
        // The map orders the lanes as `swimlanesSpecial` does.
        for (lane, translate) in layout.lanes.values_mut().zip(translates) {
            lane.translate = translate;
        }
        layout
    }

    /// The room beside the divider left of the `i`th lane that its title, when wider than the lane, needs.
    fn get_half_missing_space(
        &self,
        string_bounder: &dyn StringBounder,
        special: &[&Swimlane],
        layout: &SwimlanesLayout,
        i: usize,
        min: f64,
    ) -> f64 {
        if i == 0 || i > special.len() {
            return 5.0;
        }
        let swimlane = special[i - 1];
        let lane = &layout.lanes[&swimlane.order];
        let swimlane_actual_width = min.max(lane.get_width_without_title());
        let title_width = self
            .get_title(swimlane, lane.actual_width)
            .calculate_dimension(string_bounder)
            .width;
        if title_width <= swimlane_actual_width {
            return 5.0;
        }
        f64::max(5.0, 5.0 + (title_width - swimlane_actual_width) / 2.0)
    }
}

impl TextBlock for SwimlanesDrawing<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.get_min_max().dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let layout = (self.swimlanes.swimlanes().len() > 1).then(|| self.ensure_size_computed());
        let full = self.create_ftile();
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Goto,
        ])
        .get_merged_style(&self.skin_param.current_style_builder());
        let goto_color = style.value(PName::LineColor).as_color();
        let ug = UGraphicForSnake::create(ug.clone());
        if let Some(layout) = layout {
            self.draw_when_swimlanes(&ug, &full, &layout);
        } else {
            TextBlockInterceptorUDrawable::new(full, goto_color).draw_u(&ug);
            ug.flush_ug();
        }
    }
}

/// The layer drawing, of the whole tile tree, only the connections from one lane into another, each end
/// moved into its lane (PlantUML's `Swimlanes.Cross`).
struct Cross {
    ug: UGraphic,
    layout: Rc<SwimlanesLayout>,
}

impl Cross {
    fn create(ug: UGraphic, layout: Rc<SwimlanesLayout>) -> UGraphic {
        UGraphic::from_layer(Self { ug, layout })
    }
}

impl UGraphicLayer for Cross {
    fn ug(&self) -> &UGraphic {
        &self.ug
    }

    fn apply(&self, change: UChange) -> UGraphic {
        Self::create(self.ug.apply(change), self.layout.clone())
    }

    fn draw(&self, this: &UGraphic, shape: AnyShape<'_>) {
        match shape {
            AnyShape::Ftile(tile) => tile.draw_u(this),
            AnyShape::Connection(connection) => {
                let (Some(tile1), Some(tile2)) = (connection.get_ftile1(), connection.get_ftile2())
                else {
                    return;
                };
                if tile1.get_swimlane_out() != tile2.get_swimlane_in() {
                    ConnectionCross::new(connection)
                        .draw_u(&self.ug, |swimlane| self.layout.translate(swimlane));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lane_is_smaller_than_later_lanes_but_not_than_itself_alone() {
        let lanes = |ids: &[usize]| ids.iter().map(|&id| Some(SwimlaneId(id))).collect();
        assert!(SwimlaneId(1).is_smaller_than_all_others(&lanes(&[1, 2])));
        assert!(SwimlaneId(1).is_smaller_than_all_others(&lanes(&[2, 3])));
        assert!(!SwimlaneId(1).is_smaller_than_all_others(&lanes(&[1])));
        assert!(!SwimlaneId(2).is_smaller_than_all_others(&lanes(&[1, 2])));
        assert!(SwimlaneId(2).is_smaller_than_all_others(&SwimlaneSet::new()));
    }
}
