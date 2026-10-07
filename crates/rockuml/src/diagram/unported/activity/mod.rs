//! Legacy activity diagrams, `(*) --> "First"` (PlantUML's `activitydiagram` package). Their
//! commands only recognise their lines, so that such sources are not read as other diagrams.

mod commands;

use super::UnportedDiagram;
use crate::command::Command;
use crate::command::ParserPass;
use crate::command::unported::{self, NotPortedCommands};
use crate::diagram::common_commands::add_common_commands1;
use crate::diagram::cuca_commands::note;
use crate::ubrex::builder::UBrexPart;

/// `ActivityDiagramFactory.initCommandsList`.
pub(super) fn init_commands_list() -> Vec<Box<dyn Command<UnportedDiagram>>> {
    let mut commands = vec![footbox_ignored()];
    commands.extend(add_common_commands1());
    commands.extend([
        rank_dir(),
        partition(),
        end_partition(),
        commands::link_long_activity(),
        commands::note_activity(),
        commands::note_activity_multi_line(),
        note::note_on_link(ParserPass::One),
        note::note_on_link_multi_line(ParserPass::One),
        if_command(),
        else_command(),
        endif(),
        commands::link_activity(),
        hide_show2(),
    ]);
    commands
}

/// `CommandLinkElement.UBREX_LINE_STYLE`: the styles of a link, like `#red,dashed`.
const LINE_STYLE: &str = "【 #〇+〴w ┇dotted┇dashed┇plain┇bold┇hidden┇norank┇single┇node┇thickness=〇+〴d】 〇*【 ,#〇+〴w ┇,dotted┇,dashed┇,plain┇,bold┇,hidden┇,norank┇,single┇,node┇,thickness=〇+〴d】";

/// PlantUML's `UBrexCommandFootboxIgnored`.
fn footbox_ignored<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandFootboxIgnored",
        UBrexPart::concat(vec![
            UBrexPart::leaf("【 hide┇show 】"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::leaf("footbox"),
            UBrexPart::end(),
        ])
        .build(),
    )
}

/// PlantUML's `UBrexCommandRankDir`.
fn rank_dir<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandRankDir",
        UBrexPart::concat(vec![
            UBrexPart::named(
                "DIRECTION",
                UBrexPart::leaf("【 left∙to∙right ┇ top∙to∙bottom 】"),
            ),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("direction"),
            UBrexPart::end(),
        ])
        .build(),
    )
}

/// PlantUML's `UBrexCommandPartition`.
fn partition<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandPartition",
        UBrexPart::concat(vec![
            UBrexPart::leaf("partition"),
            UBrexPart::space_one_or_more(),
            UBrexPart::leaf("【 〃 〶$NAME=〇+「〤〃」 〃 ┇ 〶$NAME=〇+〴S   】"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::or(vec![
                // UBrexColorParser.simpleColor(ColorType.BACK)
                UBrexPart::named(
                    "COLOR",
                    UBrexPart::leaf("# 〇+〴w 〇?〘 「-\\|/」 〇+〴w 〙"),
                ),
                UBrexPart::optional(UBrexPart::leaf(
                    "【 # 〇{6}「0〜9a〜fA〜F」┇ 〇?# 〇+〴w 】",
                )),
            ]),
            UBrexPart::optional(UBrexPart::named(
                "STEREOTYPE",
                UBrexPart::leaf("<<  〄+〴. ->〘 >>〙"),
            )),
            UBrexPart::space_zero_or_more(),
            UBrexPart::optional(UBrexPart::leaf("{")),
            UBrexPart::end(),
        ])
        .build(),
    )
}

/// PlantUML's `UBrexCommandEndPartition`.
fn end_partition<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandEndPartition",
        UBrexPart::concat(vec![UBrexPart::or(vec![
            UBrexPart::concat(vec![
                UBrexPart::leaf("end"),
                UBrexPart::space_zero_or_more(),
                UBrexPart::leaf("partition"),
            ]),
            UBrexPart::leaf("}"),
            UBrexPart::end(),
        ])])
        .build(),
    )
}

/// PlantUML's `UBrexCommandIf`: `if "test" then`, after an optional source and arrow.
fn if_command<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    let simple = || {
        UBrexPart::concat(vec![
            UBrexPart::space_zero_or_more(),
            UBrexPart::optional(UBrexPart::concat(vec![
                UBrexPart::named("ARROW_BODY1", UBrexPart::leaf("〇+「-.」")),
                UBrexPart::leaf(&format!("〇?〘 [ 〶$ARROW_STYLE1=〘{LINE_STYLE}〙] 〙")),
                UBrexPart::named(
                    "ARROW_DIRECTION",
                    UBrexPart::leaf("〇?【 *┇left┇right┇up┇down┇l〇?e┇r〇?i┇u〇?p┇d〇?o】"),
                ),
                UBrexPart::leaf(&format!("〇?〘 [  〶$ARROW_STYLE2=〘{LINE_STYLE}〙] 〙")),
                UBrexPart::named("ARROW_BODY2", UBrexPart::leaf("〇*「-.」")),
                UBrexPart::leaf(">"),
            ])),
            UBrexPart::space_zero_or_more(),
            UBrexPart::optional(UBrexPart::leaf(
                "[ 〶$BRACKET=〘〇+「〤]*」  〇*「〤]」〙 ]",
            )),
            UBrexPart::space_zero_or_more(),
            UBrexPart::or(vec![
                UBrexPart::leaf(
                    "if 〇*〴s 〴g  〶$IF1=〇*〴G 〴g 〇*〴s 〇?〘as 〇+〴s 〶$ASIF1=〇+「〴an_.」 〇+〴s 〙〘then〙",
                ),
                UBrexPart::leaf("if 〇+〴s 〶$IF2=〇l+〴. 〘then〙 "),
                UBrexPart::leaf(
                    "if 〇*〴s 〴g  〶$IF1=〇*〴G 〴g 〇*〴s 〇?〘as 〇+〴s 〶$ASIF1=〇+「〴an_.」 〇+〴s 〙",
                ),
                UBrexPart::leaf("if 〇+〴s 〶$IF2=〇+〴."),
            ]),
        ])
    };
    unported::ubrex_single_line(
        "UBrexCommandIf",
        UBrexPart::concat(vec![
            UBrexPart::or(vec![
                simple(),
                UBrexPart::concat(vec![
                    UBrexPart::named("STAR", UBrexPart::leaf("(* 〇?〘top〙)")),
                    simple(),
                ]),
                UBrexPart::concat(vec![
                    UBrexPart::named("BAR", UBrexPart::leaf("=〇+= 〇*〴s 〇+「〴an_.」〇*〴s =〇+=")),
                    simple(),
                ]),
                UBrexPart::concat(vec![
                    UBrexPart::leaf("〴g 〶$QUOTED1=〇*〴G 〴g 〇?〘〇+〴s as 〇+〴s 〶$QUOTED2=〇+「〴an_.」 〙 "),
                    simple(),
                ]),
                UBrexPart::concat(vec![
                    UBrexPart::named("CODE", UBrexPart::leaf("〇+「〴an_.」")),
                    simple(),
                ]),
            ]),
            UBrexPart::end(),
        ])
        .build(),
    )
}

/// PlantUML's `UBrexCommandElse`.
fn else_command<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandElse",
        UBrexPart::concat(vec![UBrexPart::leaf("else"), UBrexPart::end()]).build(),
    )
}

/// PlantUML's `UBrexCommandEndif`.
fn endif<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandEndif",
        UBrexPart::concat(vec![
            UBrexPart::leaf("end"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::leaf("if"),
            UBrexPart::end(),
        ])
        .build(),
    )
}

/// PlantUML's `UBrexCommandHideShow2`.
fn hide_show2<D: NotPortedCommands>() -> Box<dyn Command<D>> {
    unported::ubrex_single_line(
        "UBrexCommandHideShow2",
        UBrexPart::concat(vec![
            UBrexPart::named(
                "COMMAND",
                UBrexPart::leaf("【hide-class┇hide┇show-class┇show】"),
            ),
            UBrexPart::space_one_or_more(),
            UBrexPart::named("WHAT", UBrexPart::leaf("【 << 〇*「〤<>」>> ┇ 〇+〴S 】 ")),
            UBrexPart::end(),
        ])
        .build(),
    )
}
