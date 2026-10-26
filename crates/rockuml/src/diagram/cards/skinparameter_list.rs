//! The skin parameter list, `skinparameters`: the names `skinparam` knows, in five columns.

use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::{Card, UgDiagram};
use crate::diagram::builder::{PSystemSingleLineFactory, whole_line};
use crate::diagram::{Diagram, NotYetPorted, UmlSource};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::graphic_strings::GraphicStrings;
use crate::klimt::typeface::FontRegistry;
use crate::klimt::ugraphic::UGraphic;

/// `skinparameters` (PlantUML's `PSystemSkinparameterListFactory`).
pub(in crate::diagram) struct PSystemSkinparameterListFactory;

impl PSystemSingleLineFactory for PSystemSkinparameterListFactory {
    fn execute_line(
        source: &Rc<UmlSource>,
        line: &str,
    ) -> Option<Result<Box<dyn Diagram>, NotYetPorted>> {
        static PATTERN: LazyLock<Regex> = LazyLock::new(|| whole_line(r"(?i)(skinparameters)\s*"));
        PATTERN
            .is_match(line)
            .then(|| Ok(UgDiagram::boxed(source, PSystemSkinparameterList)))
    }
}

/// PlantUML's `PSystemSkinparameterList`.
struct PSystemSkinparameterList;

impl Card for PSystemSkinparameterList {
    fn get_text_block(&self, _fonts: &FontRegistry) -> Box<dyn TextBlock + '_> {
        Box::new(Columns(get_cols(&get_possible_values(), 5)))
    }
}

/// The lines cut into `nb_col` columns of equal length, the last one shorter.
fn get_cols(lines: &[String], nb_col: usize) -> Vec<GraphicStrings> {
    lines
        .chunks(lines.len().div_ceil(nb_col))
        .map(GraphicStrings::create_black_on_white)
        .collect()
}

/// Blocks side by side, each followed by a gap.
struct Columns(Vec<GraphicStrings>);

impl Columns {
    const GAP: f64 = 10.0;
}

impl TextBlock for Columns {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.0
            .iter()
            .map(|column| column.calculate_dimension(string_bounder))
            .fold(XDimension2D::default(), |total, dim| {
                XDimension2D::new(
                    total.width + dim.width + Self::GAP,
                    total.height.max(dim.height),
                )
            })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let mut x = 0.0;
        for column in &self.0 {
            column.draw_u(&ug.translated(x, 0.0));
            x += column.calculate_dimension(ug.string_bounder()).width + Self::GAP;
        }
    }
}

/// The constants of PlantUML's `FontParam`, each the start of four parameters.
const FONT_PARAM: [&str; 88] = [
    "TIMING",
    "ACTIVITY",
    "ACTIVITY_DIAMOND",
    "ARROW",
    "CIRCLED_CHARACTER",
    "OBJECT_ATTRIBUTE",
    "OBJECT",
    "OBJECT_STEREOTYPE",
    "CLASS_ATTRIBUTE",
    "CLASS",
    "CLASS_STEREOTYPE",
    "COMPONENT",
    "INTERFACE",
    "INTERFACE_STEREOTYPE",
    "COMPONENT_STEREOTYPE",
    "NOTE",
    "PACKAGE",
    "PACKAGE_STEREOTYPE",
    "ACTOR",
    "ARTIFACT",
    "CLOUD",
    "FOLDER",
    "FILE",
    "FRAME",
    "STORAGE",
    "BOUNDARY",
    "CONTROL",
    "ENTITY",
    "AGENT",
    "RECTANGLE",
    "LABEL",
    "HEXAGON",
    "PERSON",
    "ARCHIMATE",
    "CARD",
    "NODE",
    "DATABASE",
    "QUEUE",
    "STACK",
    "SEQUENCE_BOX",
    "SEQUENCE_DIVIDER",
    "SEQUENCE_REFERENCE",
    "SEQUENCE_DELAY",
    "SEQUENCE_GROUP",
    "SEQUENCE_GROUP_HEADER",
    "PARTICIPANT",
    "PARTICIPANT_STEREOTYPE",
    "STATE",
    "STATE_ATTRIBUTE",
    "LEGEND",
    "TITLE",
    "CAPTION",
    "SWIMLANE_TITLE",
    "FOOTER",
    "HEADER",
    "USECASE",
    "USECASE_STEREOTYPE",
    "ARTIFACT_STEREOTYPE",
    "CLOUD_STEREOTYPE",
    "STORAGE_STEREOTYPE",
    "BOUNDARY_STEREOTYPE",
    "CONTROL_STEREOTYPE",
    "ENTITY_STEREOTYPE",
    "AGENT_STEREOTYPE",
    "RECTANGLE_STEREOTYPE",
    "LABEL_STEREOTYPE",
    "PERSON_STEREOTYPE",
    "HEXAGON_STEREOTYPE",
    "ARCHIMATE_STEREOTYPE",
    "CARD_STEREOTYPE",
    "NODE_STEREOTYPE",
    "FOLDER_STEREOTYPE",
    "FILE_STEREOTYPE",
    "FRAME_STEREOTYPE",
    "DATABASE_STEREOTYPE",
    "QUEUE_STEREOTYPE",
    "STACK_STEREOTYPE",
    "ACTOR_STEREOTYPE",
    "SEQUENCE_STEREOTYPE",
    "PARTITION",
    "DESIGNED_DOMAIN",
    "DESIGNED_DOMAIN_STEREOTYPE",
    "DOMAIN",
    "DOMAIN_STEREOTYPE",
    "MACHINE",
    "MACHINE_STEREOTYPE",
    "REQUIREMENT",
    "REQUIREMENT_STEREOTYPE",
];

/// The constants of PlantUML's `ColorParam`, each a `…Color` parameter.
const COLOR_PARAM: [&str; 41] = [
    "background",
    "hyperlink",
    "activityBackground",
    "activityBorder",
    "classBackground",
    "classBorder",
    "arrowHead",
    "stateBorder",
    "noteBackground",
    "noteBorder",
    "diagramBorder",
    "actorBackground",
    "actorBorder",
    "sequenceGroupBodyBackground",
    "sequenceReferenceHeaderBackground",
    "sequenceReferenceBackground",
    "sequenceLifeLineBorder",
    "sequenceNewpageSeparator",
    "sequenceBoxBorder",
    "iconPrivate",
    "iconPrivateBackground",
    "iconPackage",
    "iconPackageBackground",
    "iconProtected",
    "iconProtectedBackground",
    "iconPublic",
    "iconPublicBackground",
    "iconIEMandatory",
    "arrowLollipop",
    "machineBackground",
    "machineBorder",
    "requirementBackground",
    "requirementBorder",
    "designedBackground",
    "designedBorder",
    "domainBackground",
    "domainBorder",
    "lexicalBackground",
    "lexicalBorder",
    "biddableBackground",
    "biddableBorder",
];

/// The constants of PlantUML's `LineParam`, each a `…Thickness` parameter.
const LINE_PARAM: [&str; 31] = [
    "sequenceActorBorder",
    "sequenceDividerBorder",
    "sequenceLifeLineBorder",
    "sequenceParticipantBorder",
    "noteBorder",
    "sequenceGroupBorder",
    "sequenceReferenceBorder",
    "legendBorder",
    "sequenceArrow",
    "arrow",
    "classBorder",
    "objectBorder",
    "usecaseBorder",
    "partitionBorder",
    "packageBorder",
    "swimlaneBorder",
    "activityBorder",
    "titleBorder",
    "diagramBorder",
    "rectangleBorder",
    "hexagonBorder",
    "personBorder",
    "archimateBorder",
    "componentBorder",
    "cardBorder",
    "queueBorder",
    "agentBorder",
    "domainBorder",
    "designedDomainBorder",
    "machineBorder",
    "requirementBorder",
];

/// The constants of PlantUML's `AlignmentParam`, each a parameter.
const ALIGNMENT_PARAM: [&str; 7] = [
    "arrowMessageAlignment",
    "stateMessageAlignment",
    "sequenceMessageAlignment",
    "sequenceMessageTextAlignment",
    "sequenceReferenceAlignment",
    "packageTitleAlignment",
    "noteTextAlignment",
];

/// The parameters with names of their own (`SkinParam.getPossibleValues`).
const NAMED: [&str; 47] = [
    "Monochrome",
    "CircledCharacterRadius",
    "ClassAttributeIconSize",
    "DefaultFontName",
    "DefaultFontStyle",
    "DefaultFontSize",
    "DefaultFontColor",
    "MinClassWidth",
    "Dpi",
    "DefaultTextAlignment",
    "Shadowing",
    "NoteShadowing",
    "Handwritten",
    "Linetype",
    "PackageStyle",
    "ComponentStyle",
    "StereotypePosition",
    "Nodesep",
    "Ranksep",
    "RoundCorner",
    "TitleBorderRoundCorner",
    "MaxMessageSize",
    "Style",
    "SequenceParticipant",
    "ConditionStyle",
    "ConditionEndStyle",
    "SameClassWidth",
    "HyperlinkUnderline",
    "Padding",
    "BoxPadding",
    "ParticipantPadding",
    "Guillemet",
    "SvglinkTarget",
    "DefaultMonospacedFontName",
    "TabSize",
    "MaxAsciiMessageLength",
    "ColorArrowSeparationSpace",
    "ResponseMessageBelowArrow",
    "GenericDisplay",
    "PathHoverColor",
    "SwimlaneWidth",
    "PageBorderColor",
    "PageExternalColor",
    "PageMargin",
    "WrapWidth",
    "SwimlaneWrapTitleWidth",
    "FixCircleLabelOverlapping",
];

/// Every skin parameter name, sorted (`SkinParam.getPossibleValues`).
fn get_possible_values() -> Vec<String> {
    let mut result: BTreeSet<String> = NAMED.iter().map(|&name| name.to_owned()).collect();
    for param in FONT_PARAM {
        let h = human_name(param);
        for suffix in ["FontStyle", "FontName", "FontSize", "FontColor"] {
            result.insert(format!("{h}{suffix}"));
        }
    }
    result.extend(
        COLOR_PARAM
            .iter()
            .map(|param| format!("{}Color", capitalize(param))),
    );
    result.extend(
        LINE_PARAM
            .iter()
            .map(|param| format!("{}Thickness", capitalize(param))),
    );
    result.extend(ALIGNMENT_PARAM.iter().map(|param| capitalize(param)));
    result.into_iter().collect()
}

/// `ACTIVITY_DIAMOND` as `ActivityDiamond` (`SkinParam.humanName`).
fn human_name(key: &str) -> String {
    key.split('_')
        .flat_map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase())
                .into_iter()
                .chain(chars.map(|c| c.to_ascii_lowercase()))
        })
        .collect()
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase())
        .into_iter()
        .chain(chars)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_names_follow_the_java_constants() {
        assert_eq!(human_name("ACTIVITY_DIAMOND"), "ActivityDiamond");
        assert_eq!(capitalize("iconIEMandatory"), "IconIEMandatory");
        let values = get_possible_values();
        assert_eq!(values.len(), 478);
        assert_eq!(
            values.first().map(String::as_str),
            Some("ActivityBackgroundColor")
        );
    }
}
