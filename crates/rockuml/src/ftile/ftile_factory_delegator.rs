//! A factory wrapping another, overriding some methods and passing the others on (PlantUML's
//! `FtileFactoryDelegator`), with the helpers its subclasses share.

use std::rc::Rc;

use super::{BoxStyle, Ftile, FtileFactory};
use crate::color::{Colors, HColor};
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{LinkRendering, SwimlaneId};
use crate::klimt::font::StringBounder;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{SName, Style, StyleBuilder, StyleSignature};

/// A delegator implements [`Self::get_factory`] and overrides what it changes; it is an [`FtileFactory`]
/// through the blanket implementation below. Overrides reach the wrapped factory (PlantUML's
/// `super.method(...)`) with `self.get_factory().method(...)`.
pub(crate) trait FtileFactoryDelegator {
    /// The factory wrapped, which builds what this one does not change (`getFactory`). Builders that
    /// PlantUML hands `getFactory()` get it too, so they skip this delegator and those outside it.
    fn get_factory(&self) -> &dyn FtileFactory;

    fn get_string_bounder(&self) -> &dyn StringBounder {
        self.get_factory().get_string_bounder()
    }

    fn skin_param(&self) -> &Rc<SkinParam> {
        self.get_factory().skin_param()
    }

    fn start(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        self.get_factory().start(swimlane, colors)
    }

    fn stop(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        self.get_factory().stop(swimlane, colors)
    }

    fn end(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        self.get_factory().end(swimlane, colors)
    }

    fn spot(
        &self,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        color: Option<HColor>,
    ) -> Rc<dyn Ftile> {
        self.get_factory().spot(swimlane, spot, color)
    }

    fn activity(
        &self,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<&Stereotype>,
        style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        self.get_factory()
            .activity(label, swimlane, style, colors, stereotype, style_builder)
    }

    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile> {
        self.get_factory().add_url(ftile, url)
    }

    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        self.get_factory().decorate_in(ftile, link_rendering)
    }

    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        self.get_factory().decorate_out(ftile, link_rendering)
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        self.get_factory().assembly(tile1, tile2)
    }

    fn get_default_style_definition_activity(&self) -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
        ])
    }

    fn get_default_style_definition_diamond(&self) -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
            SName::Diamond,
        ])
    }

    fn get_default_style_definition_arrow(&self) -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
    }

    /// The style of activity arrows.
    fn arrow_style(&self) -> Style {
        self.skin_param()
            .merged_style(&self.get_default_style_definition_arrow())
            .expect("the skin styles activity arrows")
    }

    /// The colours of the arrow into `tile`, or else those of the arrow style (`getInLinkRenderingColor`).
    fn get_in_link_rendering_color(&self, tile: &dyn Ftile) -> Rainbow {
        let color = tile.get_in_link_rendering().get_rainbow().clone();
        if color.size() == 0 {
            return Rainbow::build_from_style(&self.arrow_style());
        }
        color
    }

    /// An arrow label in the arrow style's font; none without a label (`getTextBlock`).
    fn get_text_block(&self, display: Option<&Display>) -> Option<Rc<dyn TextBlock>> {
        let display = display?;
        let font_configuration = self.arrow_style().font_configuration();
        Some(Rc::new(display.create0(
            &font_configuration,
            HorizontalAlignment::Left,
            self.skin_param().as_ref(),
            0.0,
            CreoleMode::SimpleLine,
        )))
    }

    /// The label of the arrow into `tile` (`getInLinkRenderingDisplay`).
    fn get_in_link_rendering_display(&self, tile: &dyn Ftile) -> Option<Display> {
        tile.get_in_link_rendering().get_display().cloned()
    }
}

impl<T: FtileFactoryDelegator> FtileFactory for T {
    fn get_string_bounder(&self) -> &dyn StringBounder {
        FtileFactoryDelegator::get_string_bounder(self)
    }

    fn skin_param(&self) -> &Rc<SkinParam> {
        FtileFactoryDelegator::skin_param(self)
    }

    fn start(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::start(self, swimlane, colors)
    }

    fn stop(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::stop(self, swimlane, colors)
    }

    fn end(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::end(self, swimlane, colors)
    }

    fn spot(
        &self,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        color: Option<HColor>,
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::spot(self, swimlane, spot, color)
    }

    fn activity(
        &self,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<&Stereotype>,
        style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::activity(
            self,
            label,
            swimlane,
            style,
            colors,
            stereotype,
            style_builder,
        )
    }

    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::add_url(self, ftile, url)
    }

    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::decorate_in(self, ftile, link_rendering)
    }

    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::decorate_out(self, ftile, link_rendering)
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::assembly(self, tile1, tile2)
    }
}
