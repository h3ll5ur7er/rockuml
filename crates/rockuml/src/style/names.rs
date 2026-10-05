//! The names a style selector and a style property can use.

macro_rules! style_names {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $java:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $($variant),*
        }

        impl $name {
            /// Every name with its Java spelling, in declaration order.
            const ALL: &[(Self, &'static str)] = &[$((Self::$variant, $java)),*];
        }
    };
}

style_names! {
    /// Selector names (`root`, `document`, `mindmapDiagram`, `node`...).
    SName {
        Action = "action", ActivationBox = "activationBox", Activity = "activity", ActivityBar = "activityBar",
        ActivityDiagram = "activityDiagram", Actor = "actor", Agent = "agent", Analog = "analog",
        Archimate = "archimate", Arrow = "arrow", Artifact = "artifact", Binary = "binary", Body = "body",
        Boundary = "boundary", Box = "box", Boxless = "boxless", Business = "business", Caption = "caption",
        Card = "card", Cardinality = "cardinality", Circle = "circle", ClassDiagram = "classDiagram",
        Class = "class_", Clickable = "clickable", Cloud = "cloud", Closed = "closed", Collection = "collection",
        Collections = "collections", Component = "component", Composite = "composite", Robust = "robust",
        ChenAttribute = "chenAttribute", ChenEerDiagram = "chenEerDiagram", ChenEntity = "chenEntity",
        ChenRelationship = "chenRelationship", Concise = "concise", Clock = "clock",
        ComponentDiagram = "componentDiagram", ConstraintArrow = "constraintArrow", Control = "control",
        Database = "database", Day = "day", Delay = "delay", Description = "description", Destroy = "destroy",
        Diamond = "diamond", Document = "document", Ebnf = "ebnf", Element = "element", Entity = "entity",
        End = "end", Start = "start", Stop = "stop", File = "file", FilesDiagram = "filesDiagram",
        Folder = "folder", Footer = "footer", Frame = "frame", GanttDiagram = "ganttDiagram",
        Generic = "generic", Goto = "goto_", Group = "group", GroupHeader = "groupHeader", Header = "header",
        Hexagon = "hexagon", Highlight = "highlight", Hnote = "hnote", Interface = "interface_", Json = "json",
        JsonDiagram = "jsonDiagram", GitDiagram = "gitDiagram", Label = "label", LeafNode = "leafNode",
        Legend = "legend", LifeLine = "lifeLine", Mainframe = "mainframe", Map = "map", Milestone = "milestone",
        MindmapDiagram = "mindmapDiagram", Month = "month", Name = "name", Network = "network",
        Newpage = "newpage", Node = "node", Note = "note", NwdiagDiagram = "nwdiagDiagram",
        PacketdiagDiagram = "packetdiagDiagram", ObjectDiagram = "objectDiagram", Object = "object",
        Package = "package_", Participant = "participant", Partition = "partition", Person = "person",
        Port = "port", Process = "process", Qualified = "qualified", Queue = "queue", Rectangle = "rectangle",
        Reference = "reference", ReferenceHeader = "referenceHeader", Regex = "regex",
        Requirement = "requirement", Rnote = "rnote", Root = "root", RootNode = "rootNode",
        SaltDiagram = "saltDiagram", Separator = "separator", SequenceDiagram = "sequenceDiagram",
        Server = "server", Stack = "stack", StateDiagram = "stateDiagram", State = "state",
        StateBody = "stateBody", Stereotype = "stereotype", Storage = "storage", Swimlane = "swimlane",
        Task = "task", Timegrid = "timegrid", Timeline = "timeline", TimingDiagram = "timingDiagram",
        Title = "title", Undone = "undone", Unstarted = "unstarted", Usecase = "usecase",
        VerticalSeparator = "verticalSeparator", Year = "year", VisibilityIcon = "visibilityIcon",
        Private = "private_", Protected = "protected_", Public = "public_", IeMandatory = "IEMandatory",
        Spot = "spot", SpotAnnotation = "spotAnnotation", SpotInterface = "spotInterface",
        SpotEnum = "spotEnum", SpotProtocol = "spotProtocol", SpotStruct = "spotStruct",
        SpotEntity = "spotEntity", SpotException = "spotException", SpotClass = "spotClass",
        SpotAbstractClass = "spotAbstractClass", SpotMetaClass = "spotMetaClass",
        SpotStereotype = "spotStereotype", SpotDataClass = "spotDataClass", SpotRecord = "spotRecord",
        WbsDiagram = "wbsDiagram", YamlDiagram = "yamlDiagram", ChartDiagram = "chartDiagram", Bar = "bar",
        Line = "line", Area = "area", Scatter = "scatter", Axis = "axis", HAxis = "hAxis", VAxis = "vAxis",
        Grid = "grid", Annotation = "annotation",
    }
}

style_names! {
    /// Style properties (`FontSize`, `BackGroundColor`...).
    PName {
        Shadowing = "Shadowing", FontName = "FontName", FontColor = "FontColor", FontSize = "FontSize",
        FontStyle = "FontStyle", FontWeight = "FontWeight", BackGroundColor = "BackGroundColor",
        RoundCorner = "RoundCorner", LineThickness = "LineThickness", DiagonalCorner = "DiagonalCorner",
        HyperLinkColor = "HyperLinkColor", HyperlinkUnderlineStyle = "HyperlinkUnderlineStyle",
        HyperlinkUnderlineThickness = "HyperlinkUnderlineThickness", HeadColor = "HeadColor",
        LineColor = "LineColor", LineStyle = "LineStyle", Padding = "Padding", Margin = "Margin",
        MaximumWidth = "MaximumWidth", MinimumWidth = "MinimumWidth", ExportedName = "ExportedName",
        Image = "Image", HorizontalAlignment = "HorizontalAlignment", ShowStereotype = "ShowStereotype",
        ImagePosition = "ImagePosition", MarkerShape = "MarkerShape", MarkerSize = "MarkerSize",
        MarkerColor = "MarkerColor", BarWidth = "BarWidth", Width = "Width",
    }
}

impl SName {
    pub fn java_name(self) -> &'static str {
        Self::ALL[self as usize].1
    }

    /// Selectors ignore case and the underscore that keeps Java keywords like `class_` legal.
    pub fn retrieve(name: &str) -> Option<Self> {
        let wanted = name.to_lowercase();
        Self::ALL
            .iter()
            .find(|(_, java)| java.replace('_', "").to_lowercase() == wanted)
            .map(|&(sname, _)| sname)
    }

    fn bit(self) -> (usize, u64) {
        let index = self as usize;
        (index / 64, 1 << (index % 64))
    }
}

impl PName {
    pub fn retrieve(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, java)| java.eq_ignore_ascii_case(name))
            .map(|&(pname, _)| pname)
    }
}

/// A set of selector names, like Java's `EnumSet<SName>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SNames([u64; 3]);

impl SNames {
    pub fn of(names: &[SName]) -> Self {
        names
            .iter()
            .fold(Self::default(), |set, &name| set.with(name))
    }

    #[must_use]
    pub fn with(mut self, name: SName) -> Self {
        let (word, mask) = name.bit();
        self.0[word] |= mask;
        self
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self([
            self.0[0] | other.0[0],
            self.0[1] | other.0[1],
            self.0[2] | other.0[2],
        ])
    }

    pub fn contains_all(self, other: Self) -> bool {
        self.0
            .iter()
            .zip(other.0)
            .all(|(mine, theirs)| mine & theirs == theirs)
    }

    pub fn is_empty(self) -> bool {
        self.0 == [0; 3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_found_ignoring_case_and_underscores() {
        assert_eq!(
            SName::retrieve("MindMapDiagram"),
            Some(SName::MindmapDiagram)
        );
        assert_eq!(SName::retrieve("class"), Some(SName::Class));
        assert_eq!(SName::retrieve("class_"), None);
        assert_eq!(SName::retrieve("nothing"), None);
        assert_eq!(
            PName::retrieve("backgroundcolor"),
            Some(PName::BackGroundColor)
        );
    }

    #[test]
    fn name_sets_cover_every_selector() {
        let last = SName::Annotation;
        let set = SNames::of(&[SName::Root, last]);
        assert!(set.contains_all(SNames::of(&[last])));
        assert!(!SNames::of(&[SName::Root]).contains_all(set));
        assert!(SNames::default().is_empty());
    }
}
