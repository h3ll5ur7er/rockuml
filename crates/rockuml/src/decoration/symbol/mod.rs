//! The shapes of description-diagram elements and of packages (PlantUML's `USymbol`, `USymbols` and their
//! subclasses). Each symbol has a small form, framed around an element's label, and a big form, a frame of a given
//! size around a cluster.

mod action;
mod artifact;
mod card;
mod cloud;
mod collections;
mod component1;
mod component2;
mod database;
mod file;
mod folder;
mod footprint;
mod frame;
mod hexagon;
mod label;
mod node;
mod package_style;
mod person;
mod process;
mod queue;
mod rectangle;
mod simple_abstract;
mod stack;
mod storage;
#[cfg(test)]
mod tests;
mod usecase;

use std::rc::Rc;

pub(crate) use package_style::PackageStyle;
pub(crate) use usecase::TextBlockInEllipse;

use crate::klimt::blocks::TextBlockVertical;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::actor::ActorStyle;
use crate::skin::component_style::ComponentStyle;
use crate::style::SName;

/// A text block a symbol is drawn around, which its owner may draw on its own too.
pub(crate) type Block = Rc<dyn TextBlock>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum USymbol {
    Action(SName),
    /// Never with [`ActorStyle::StickmanBusiness`], which is [`USymbol::ActorBusiness`].
    Actor(ActorStyle),
    ActorBusiness,
    Artifact,
    Boundary,
    Card,
    Cloud,
    Collections,
    Component1,
    Component2,
    Control,
    Database,
    EntityDomain,
    File,
    Folder {
        sname: SName,
        show_title: bool,
    },
    Frame(SName),
    Hexagon,
    Interface,
    Label,
    Node,
    Person,
    Process(SName),
    Queue,
    Rectangle(SName),
    Stack,
    Storage,
    Usecase {
        business: bool,
    },
}

impl USymbol {
    /// The style names of the symbol's elements, after `element`.
    pub(crate) fn get_s_names(self) -> Vec<SName> {
        let sname = match self {
            Self::Action(sname)
            | Self::Folder { sname, .. }
            | Self::Frame(sname)
            | Self::Process(sname)
            | Self::Rectangle(sname) => sname,
            Self::Actor(_) => SName::Actor,
            Self::ActorBusiness => SName::Business,
            Self::Artifact => SName::Artifact,
            Self::Boundary => SName::Boundary,
            Self::Card => SName::Card,
            Self::Cloud => SName::Cloud,
            Self::Collections => SName::Collections,
            Self::Component1 | Self::Component2 => SName::Component,
            Self::Control => SName::Control,
            Self::Database => SName::Database,
            Self::EntityDomain => SName::Entity,
            Self::File => SName::File,
            Self::Hexagon => SName::Hexagon,
            Self::Interface => SName::Interface,
            Self::Label => SName::Label,
            Self::Node => SName::Node,
            Self::Person => SName::Person,
            Self::Queue => SName::Queue,
            Self::Stack => SName::Stack,
            Self::Storage => SName::Storage,
            Self::Usecase { business: false } => SName::Usecase,
            Self::Usecase { business: true } => return vec![SName::Usecase, SName::Business],
        };
        vec![sname]
    }

    /// The symbol framed around an element's `label` and `stereotype`. Only folders show the element's `name`,
    /// and only packages among them.
    pub(crate) fn as_small(
        self,
        name: Block,
        label: Block,
        stereotype: Block,
        fashion: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Box<dyn TextBlock> {
        let content = SmallContent {
            label,
            stereotype,
            fashion,
            stereo_alignment,
        };
        match self {
            Self::Action(_) => Small::boxed(action::USymbolAction, content),
            Self::Actor(style) => {
                simple_abstract::as_small(style.text_block(content.fashion.clone()), content)
            }
            Self::ActorBusiness => simple_abstract::as_small(
                ActorStyle::StickmanBusiness.text_block(content.fashion.clone()),
                content,
            ),
            Self::Artifact => Small::boxed(artifact::USymbolArtifact, content),
            Self::Boundary => simple_abstract::boundary(content),
            Self::Card => Small::boxed(card::USymbolCard, content),
            Self::Cloud => Small::boxed(cloud::USymbolCloud, content),
            Self::Collections => Small::boxed(collections::USymbolCollections, content),
            Self::Component1 => Small::boxed(component1::USymbolComponent1, content),
            Self::Component2 => Small::boxed(component2::USymbolComponent2, content),
            Self::Control => simple_abstract::control(content),
            Self::Database => Small::boxed(database::USymbolDatabase, content),
            Self::EntityDomain => simple_abstract::entity_domain(content),
            Self::File => Small::boxed(file::USymbolFile, content),
            Self::Folder { show_title, .. } => folder::as_small(name, show_title, content),
            Self::Frame(_) => Small::boxed(frame::USymbolFrame, content),
            Self::Hexagon => hexagon::as_small(content),
            Self::Interface => simple_abstract::interface(content),
            Self::Label => Small::boxed(label::USymbolLabel, content),
            Self::Node => Small::boxed(node::USymbolNode, content),
            Self::Person => person::as_small(content),
            Self::Process(_) => Small::boxed(process::USymbolProcess, content),
            Self::Queue => Small::boxed(queue::USymbolQueue, content),
            Self::Rectangle(_) => Small::boxed(rectangle::USymbolRectangle, content),
            Self::Stack => Small::boxed(stack::USymbolStack, content),
            Self::Storage => Small::boxed(storage::USymbolStorage, content),
            Self::Usecase { business } => usecase::as_small(business, content),
        }
    }

    /// The symbol as a frame `width` by `height` with `title` and `stereotype` at its top; `None` for actors,
    /// robustness symbols, interfaces, persons and use cases, which have no big form.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's asBig")]
    pub(crate) fn as_big(
        self,
        title: Block,
        label_alignment: HorizontalAlignment,
        stereotype: Block,
        width: f64,
        height: f64,
        fashion: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Option<Box<dyn TextBlock>> {
        let content = BigContent {
            title,
            label_alignment,
            stereotype,
            width,
            height,
            fashion,
            stereo_alignment,
        };
        Some(match self {
            Self::Action(_) => Big::boxed(action::USymbolAction, content),
            Self::Artifact => Big::boxed(artifact::USymbolArtifact, content),
            Self::Card => Big::boxed(card::USymbolCard, content),
            Self::Cloud => Big::boxed(cloud::USymbolCloud, content),
            Self::Collections => Big::boxed(collections::USymbolCollections, content),
            // UML 1 components are drawn big as UML 2 ones.
            Self::Component1 | Self::Component2 => {
                Big::boxed(component2::USymbolComponent2, content)
            }
            Self::Database => Big::boxed(database::USymbolDatabase, content),
            Self::File => Big::boxed(file::USymbolFile, content),
            Self::Folder { .. } => Big::boxed(folder::USymbolFolder, content),
            Self::Frame(_) => Big::boxed(frame::USymbolFrame, content),
            Self::Hexagon => Big::boxed(hexagon::USymbolHexagon, content),
            Self::Label => Big::boxed(label::USymbolLabel, content),
            Self::Node => Big::boxed(node::USymbolNode, content),
            Self::Process(_) => Big::boxed(process::USymbolProcess, content),
            Self::Queue => Big::boxed(queue::USymbolQueue, content),
            Self::Rectangle(_) => Big::boxed(rectangle::USymbolRectangle, content),
            Self::Stack => Big::boxed(stack::USymbolStack, content),
            Self::Storage => Big::boxed(storage::USymbolStorage, content),
            Self::Actor(_)
            | Self::ActorBusiness
            | Self::Boundary
            | Self::Control
            | Self::EntityDomain
            | Self::Interface
            | Self::Person
            | Self::Usecase { .. } => return None,
        })
    }

    /// The room a cluster's title leaves below it for the symbol's shape.
    pub(crate) fn supp_height_because_of_shape(self) -> i32 {
        match self {
            Self::Database => 15,
            Self::Node => 5,
            _ => 0,
        }
    }

    /// The room a cluster's title leaves beside it for the symbol's shape.
    pub(crate) fn supp_width_because_of_shape(self) -> i32 {
        match self {
            Self::Node => 60,
            _ => 0,
        }
    }
}

/// The symbols by name (PlantUML's `USymbols`).
pub(crate) struct USymbols;

impl USymbols {
    pub(crate) const ACTION: USymbol = USymbol::Action(SName::Action);
    pub(crate) const ACTOR_AWESOME: USymbol = USymbol::Actor(ActorStyle::Awesome);
    pub(crate) const ACTOR_HOLLOW: USymbol = USymbol::Actor(ActorStyle::Hollow);
    pub(crate) const ACTOR_STICKMAN: USymbol = USymbol::Actor(ActorStyle::Stickman);
    pub(crate) const ACTOR_STICKMAN_BUSINESS: USymbol = USymbol::ActorBusiness;
    pub(crate) const AGENT: USymbol = USymbol::Rectangle(SName::Agent);
    pub(crate) const ARCHIMATE: USymbol = USymbol::Rectangle(SName::Archimate);
    pub(crate) const ARTIFACT: USymbol = USymbol::Artifact;
    pub(crate) const BOUNDARY: USymbol = USymbol::Boundary;
    pub(crate) const CARD: USymbol = USymbol::Card;
    pub(crate) const CLOUD: USymbol = USymbol::Cloud;
    pub(crate) const COLLECTIONS: USymbol = USymbol::Collections;
    pub(crate) const COMPONENT_RECTANGLE: USymbol = USymbol::Rectangle(SName::Component);
    pub(crate) const COMPONENT1: USymbol = USymbol::Component1;
    pub(crate) const COMPONENT2: USymbol = USymbol::Component2;
    pub(crate) const CONTROL: USymbol = USymbol::Control;
    pub(crate) const DATABASE: USymbol = USymbol::Database;
    pub(crate) const ENTITY_DOMAIN: USymbol = USymbol::EntityDomain;
    pub(crate) const FILE: USymbol = USymbol::File;
    pub(crate) const FOLDER: USymbol = USymbol::Folder {
        sname: SName::Folder,
        show_title: false,
    };
    pub(crate) const FRAME: USymbol = USymbol::Frame(SName::Frame);
    pub(crate) const GROUP: USymbol = USymbol::Frame(SName::Group);
    pub(crate) const HEXAGON: USymbol = USymbol::Hexagon;
    pub(crate) const INTERFACE: USymbol = USymbol::Interface;
    pub(crate) const LABEL: USymbol = USymbol::Label;
    pub(crate) const NODE: USymbol = USymbol::Node;
    pub(crate) const PACKAGE: USymbol = USymbol::Folder {
        sname: SName::Package,
        show_title: true,
    };
    pub(crate) const PARTITION: USymbol = USymbol::Frame(SName::Partition);
    pub(crate) const PERSON: USymbol = USymbol::Person;
    pub(crate) const PROCESS: USymbol = USymbol::Process(SName::Process);
    pub(crate) const QUEUE: USymbol = USymbol::Queue;
    pub(crate) const RECTANGLE: USymbol = USymbol::Rectangle(SName::Rectangle);
    pub(crate) const STACK: USymbol = USymbol::Stack;
    pub(crate) const STORAGE: USymbol = USymbol::Storage;
    pub(crate) const USECASE: USymbol = USymbol::Usecase { business: false };
    pub(crate) const USECASE_BUSINESS: USymbol = USymbol::Usecase { business: true };

    /// The symbol a code like `CLOUD` names, once upper-cased (PlantUML's `all` map).
    pub(crate) fn by_code(code: &str) -> Option<USymbol> {
        Some(match code {
            "ACTION" => Self::ACTION,
            "ACTOR_AWESOME" => Self::ACTOR_AWESOME,
            "ACTOR_HOLLOW" => Self::ACTOR_HOLLOW,
            "ACTOR_STICKMAN" => Self::ACTOR_STICKMAN,
            "ACTOR_STICKMAN_BUSINESS" => Self::ACTOR_STICKMAN_BUSINESS,
            "AGENT" => Self::AGENT,
            "ARCHIMATE" => Self::ARCHIMATE,
            "ARTIFACT" => Self::ARTIFACT,
            "BOUNDARY" => Self::BOUNDARY,
            "CARD" => Self::CARD,
            "CLOUD" => Self::CLOUD,
            "COLLECTIONS" => Self::COLLECTIONS,
            "COMPONENT_RECTANGLE" => Self::COMPONENT_RECTANGLE,
            "COMPONENT1" => Self::COMPONENT1,
            "COMPONENT2" => Self::COMPONENT2,
            "CONTROL" => Self::CONTROL,
            "DATABASE" => Self::DATABASE,
            "ENTITY_DOMAIN" => Self::ENTITY_DOMAIN,
            "FILE" => Self::FILE,
            "FOLDER" => Self::FOLDER,
            "FRAME" => Self::FRAME,
            "GROUP" => Self::GROUP,
            "HEXAGON" => Self::HEXAGON,
            "INTERFACE" => Self::INTERFACE,
            "LABEL" => Self::LABEL,
            "NODE" => Self::NODE,
            "PACKAGE" => Self::PACKAGE,
            "PARTITION" => Self::PARTITION,
            "PERSON" => Self::PERSON,
            "PROCESS" => Self::PROCESS,
            "QUEUE" => Self::QUEUE,
            "RECTANGLE" => Self::RECTANGLE,
            "STACK" => Self::STACK,
            "STORAGE" => Self::STORAGE,
            "USECASE" => Self::USECASE,
            "USECASE_BUSINESS" => Self::USECASE_BUSINESS,
            _ => return None,
        })
    }

    /// The symbol a package-like declaration names, `package`, `actor` and `component` taking the diagram's
    /// styles.
    pub(crate) fn from_string(
        s: &str,
        actor_style: ActorStyle,
        component_style: ComponentStyle,
        package_style: PackageStyle,
    ) -> Option<USymbol> {
        if s.eq_ignore_ascii_case("package") {
            return package_style.to_u_symbol();
        }
        if s.eq_ignore_ascii_case("actor") {
            return Some(actor_style.to_u_symbol());
        }
        if s.eq_ignore_ascii_case("component") {
            return Some(component_style.to_u_symbol());
        }
        if s.eq_ignore_ascii_case("entity") {
            return Some(Self::ENTITY_DOMAIN);
        }
        if s.eq_ignore_ascii_case("circle") {
            return Some(Self::INTERFACE);
        }
        let word: String = s
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        Self::by_code(&word.to_ascii_uppercase())
    }

    /// The symbol an element declaration's keyword names, `actor` and `component` taking the skin's styles.
    pub(crate) fn from_string_skin_param(symbol: &str, skin_param: &SkinParam) -> Option<USymbol> {
        let named = [
            ("artifact", Self::ARTIFACT),
            ("folder", Self::FOLDER),
            ("file", Self::FILE),
            ("package", Self::PACKAGE),
            ("rectangle", Self::RECTANGLE),
            ("person", Self::PERSON),
            ("hexagon", Self::HEXAGON),
            ("label", Self::LABEL),
            ("collections", Self::COLLECTIONS),
            ("node", Self::NODE),
            ("frame", Self::FRAME),
            ("cloud", Self::CLOUD),
            ("action", Self::ACTION),
            ("process", Self::PROCESS),
            ("database", Self::DATABASE),
            ("queue", Self::QUEUE),
            ("stack", Self::STACK),
            ("storage", Self::STORAGE),
            ("agent", Self::AGENT),
            ("actor/", Self::ACTOR_STICKMAN_BUSINESS),
            ("actor", skin_param.actor_style().to_u_symbol()),
            ("component", skin_param.component_style().to_u_symbol()),
            ("boundary", Self::BOUNDARY),
            ("control", Self::CONTROL),
            ("entity", Self::ENTITY_DOMAIN),
            ("card", Self::CARD),
            ("interface", Self::INTERFACE),
            ("()", Self::INTERFACE),
        ];
        named
            .into_iter()
            .find(|(name, _)| symbol.eq_ignore_ascii_case(name))
            .map(|(_, usymbol)| usymbol)
    }
}

impl ActorStyle {
    pub(crate) fn to_u_symbol(self) -> USymbol {
        match self {
            Self::Stickman => USymbols::ACTOR_STICKMAN,
            Self::StickmanBusiness => USymbols::ACTOR_STICKMAN_BUSINESS,
            Self::Awesome => USymbols::ACTOR_AWESOME,
            Self::Hollow => USymbols::ACTOR_HOLLOW,
        }
    }
}

impl ComponentStyle {
    pub(crate) fn to_u_symbol(self) -> USymbol {
        match self {
            Self::Uml1 => USymbols::COMPONENT1,
            Self::Uml2 => USymbols::COMPONENT2,
            Self::Rectangle => USymbols::COMPONENT_RECTANGLE,
        }
    }
}

/// The room a symbol leaves around its text (`USymbol.Margin`).
#[derive(Clone, Copy)]
struct Margin {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Margin {
    const fn new(x1: f64, x2: f64, y1: f64, y2: f64) -> Self {
        Self { x1, x2, y1, y2 }
    }

    fn add_dimension(self, dimension: XDimension2D) -> XDimension2D {
        XDimension2D::new(
            dimension.width + self.x1 + self.x2,
            dimension.height + self.y1 + self.y2,
        )
    }
}

/// `top` above `bottom` (`TextBlockUtils.mergeTB`).
fn merge_tb<'a>(
    top: &'a dyn TextBlock,
    bottom: &'a dyn TextBlock,
    alignment: HorizontalAlignment,
) -> TextBlockVertical<'a> {
    TextBlockVertical::new(vec![Box::new(top), Box::new(bottom)], alignment)
}

/// What a small symbol is drawn around: `asSmall`'s arguments.
struct SmallContent {
    label: Block,
    stereotype: Block,
    fashion: Fashion,
    stereo_alignment: HorizontalAlignment,
}

impl SmallContent {
    /// The stereotype above the label.
    fn text(&self, alignment: HorizontalAlignment) -> TextBlockVertical<'_> {
        merge_tb(&*self.stereotype, &*self.label, alignment)
    }

    fn text_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.stereotype
            .calculate_dimension(string_bounder)
            .merge_top_bottom(self.label.calculate_dimension(string_bounder))
    }
}

/// What sets apart the symbols whose small form frames their text with a margin.
trait SmallShape {
    fn margin(&self) -> Margin;

    /// Draws the frame on a surface in the symbol's colours.
    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, fashion: &Fashion);

    fn text_alignment(&self, _stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        HorizontalAlignment::Center
    }

    /// The surface the text is drawn on, which decides how separators in the text are drawn.
    fn text_surface(&self, ug: &UGraphic, dimension: XDimension2D) -> UGraphic {
        ug.with_stencil(Rc::new(RectangleStencil {
            width: dimension.width,
        }))
    }

    fn text_position(&self) -> (f64, f64) {
        let margin = self.margin();
        (margin.x1, margin.y1)
    }
}

/// A small symbol that frames its text with a margin.
struct Small<S> {
    shape: S,
    content: SmallContent,
}

impl<S: SmallShape + 'static> Small<S> {
    fn boxed(shape: S, content: SmallContent) -> Box<dyn TextBlock> {
        Box::new(Self { shape, content })
    }
}

impl<S: SmallShape> TextBlock for Small<S> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.shape
            .margin()
            .add_dimension(self.content.text_dimension(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dimension = self.calculate_dimension(ug.string_bounder());
        let ug = self.content.fashion.apply(ug);
        self.shape.draw_shape(&ug, dimension, &self.content.fashion);
        let alignment = self.shape.text_alignment(self.content.stereo_alignment);
        let (x, y) = self.shape.text_position();
        self.content
            .text(alignment)
            .draw_u(&self.shape.text_surface(&ug, dimension).translated(x, y));
    }
}

/// What a big symbol is drawn with: `asBig`'s arguments.
struct BigContent {
    title: Block,
    label_alignment: HorizontalAlignment,
    stereotype: Block,
    width: f64,
    height: f64,
    fashion: Fashion,
    stereo_alignment: HorizontalAlignment,
}

impl BigContent {
    /// Draws the stereotype centred at `y`, and the title centred below it.
    fn draw_centered_stereotype_and_title(&self, ug: &UGraphic, y: f64) {
        let string_bounder = ug.string_bounder();
        let dim_stereo = self.stereotype.calculate_dimension(string_bounder);
        self.stereotype
            .draw_u(&ug.translated((self.width - dim_stereo.width) / 2.0, y));
        let dim_title = self.title.calculate_dimension(string_bounder);
        self.title
            .draw_u(&ug.translated((self.width - dim_title.width) / 2.0, y + dim_stereo.height));
    }

    /// Draws the title centred at the top, and the stereotype centred below it, a little to the right.
    fn draw_title_then_stereotype(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_title = self.title.calculate_dimension(string_bounder);
        self.title
            .draw_u(&ug.translated((self.width - dim_title.width) / 2.0, 2.0));
        let dim_stereo = self.stereotype.calculate_dimension(string_bounder);
        let h_title = if dim_title.width == 0.0 {
            10.0
        } else {
            dim_title.height
        };
        self.stereotype
            .draw_u(&ug.translated(4.0 + (self.width - dim_stereo.width) / 2.0, 2.0 + h_title));
    }

    /// Draws the stereotype at the top, centred or in the right corner, and the title below it, starting at
    /// `title_x` of its width.
    fn draw_stereotype_top_and_title(
        &self,
        ug: &UGraphic,
        margin: Margin,
        title_x: impl Fn(&Self, f64) -> f64,
    ) {
        let string_bounder = ug.string_bounder();
        let dim_stereo = self.stereotype.calculate_dimension(string_bounder);
        let (x, y) = if self.stereo_alignment == HorizontalAlignment::Right {
            (
                self.width - dim_stereo.width - margin.x1 / 2.0,
                margin.y1 / 2.0,
            )
        } else {
            ((self.width - dim_stereo.width) / 2.0, 2.0)
        };
        self.stereotype.draw_u(&ug.translated(x, y));
        let title_width = self.title.calculate_dimension(string_bounder).width;
        self.title
            .draw_u(&ug.translated(title_x(self, title_width), 2.0 + dim_stereo.height));
    }

    fn centered_x(&self, width: f64) -> f64 {
        (self.width - width) / 2.0
    }

    /// Where the title starts given the label alignment, 3 pixels in from the sides.
    fn aligned_title_x(&self, title_width: f64) -> f64 {
        match self.label_alignment {
            HorizontalAlignment::Left => 3.0,
            HorizontalAlignment::Right => self.width - title_width - 3.0,
            HorizontalAlignment::Center => (self.width - title_width) / 2.0,
        }
    }
}

/// What sets apart the big forms of symbols.
trait BigShape {
    /// Draws the frame, title and stereotype on a surface in the symbol's colours.
    fn draw_big(&self, ug: &UGraphic, content: &BigContent);

    fn magnetic_border_force_at(
        &self,
        _content: &BigContent,
        _string_bounder: &dyn StringBounder,
        _position: XPoint2D,
    ) -> UTranslate {
        UTranslate::default()
    }
}

struct Big<S> {
    shape: S,
    content: BigContent,
}

impl<S: BigShape + 'static> Big<S> {
    fn boxed(shape: S, content: BigContent) -> Box<dyn TextBlock> {
        Box::new(Self { shape, content })
    }
}

impl<S: BigShape> TextBlock for Big<S> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(self.content.width, self.content.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.shape
            .draw_big(&self.content.fashion.apply(ug), &self.content);
    }

    fn magnetic_border_force_at(
        &self,
        string_bounder: &dyn StringBounder,
        position: XPoint2D,
    ) -> UTranslate {
        self.shape
            .magnetic_border_force_at(&self.content, string_bounder, position)
    }
}
