//! `sprite` definitions (PlantUML's `CommandFactorySprite` and `CommandSprite*`).

use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PatternCommand, SingleLine,
};
use crate::diagram::BASE64_TAG_REPLACEMENT;
use crate::diagram::titled::TitledDiagram;
use crate::klimt::image::{PortableImage, is_acceptable_size};
use crate::klimt::sprite::{
    Sprite, SpriteColorBuilder4096, SpriteContainer, SpriteGrayLevel, SpriteImage, SpriteMonochrome,
};
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::stdlib::Stdlib;
use crate::svg_parser::SvgNanoParser;
use crate::text::{LineLocation, StringLocated};

const NAME: &str = "([-%pLN_]+)";
/// Sprites declared with their data, inline or by MD5, may have dots in their names.
const NAME_WITH_DOTS: &str = "([-.%pLN_]+)";
const OPTIONAL_DOLLAR: &str = r"\$?";

/// `sprite $name [16x16/8] {` ... `}`, the size optional for 16 gray levels.
pub(super) fn multi_line<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    static START: LazyLock<RegexTree> = LazyLock::new(|| {
        declaration(
            r"\[(\d+)x(\d+)/(?:(\d+)(z)?|(color))\]",
            vec![RegexTree::spaces_zero_or_more(), RegexTree::leaf(r"\{")],
        )
    });
    static END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"^end[%s]?sprite|\}$"));
    Box::new(
        Multiline::starting_with(&START, &END, |diagram: &mut D, lines: &BlocLines| {
            let lines = lines.trimmed();
            let lines: Vec<&str> = lines
                .iter()
                .map(StringLocated::text)
                .filter(|text| !text.is_empty())
                .collect();
            let arg = START
                .matcher(lines[0])
                .expect("checked when the block was recognised");
            let data: Vec<String> = lines[1..lines.len() - 1]
                .iter()
                .map(|&text| text.to_owned())
                .collect();
            if data.is_empty() {
                return Err(CommandError::new("No sprite defined."));
            }
            execute_internal(diagram, &arg, &data)
        })
        .skipping_quote_lines(),
    )
}

/// `sprite $name [16x16/16z] data`: compressed or colour data on one line.
pub(super) fn single_line<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    let pattern = declaration(
        r"\[(\d+)x(\d+)/(?:(\d+)(z)|(color))\]",
        vec![
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "DATA", "([-_A-Za-z0-9]+)"),
        ],
    );
    Box::new(SingleLine(PatternCommand::new(
        pattern,
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let data = arg.get("DATA", 0).unwrap_or_default().to_owned();
            execute_internal(diagram, arg, &[data])
        },
    )))
}

/// `sprite $name data:image/png;base64,...`, which reaches the commands as `data:image/png;md5,...` once
/// [`crate::diagram::source::UmlSource::patch_base64`] has taken the data out (PlantUML's
/// `CommandSpriteMd5`). PlantUML's `CommandSpriteBase64`, for the same lines, never sees them for that reason.
pub(super) fn md5<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    let pattern = sprite_pattern(
        OPTIONAL_DOLLAR,
        NAME_WITH_DOTS,
        vec![
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(BASE64_TAG_REPLACEMENT),
            RegexTree::named(1, "MD5", "([0-9a-f]+)"),
        ],
    );
    Box::new(SingleLine(PatternCommand::new(
        pattern,
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let md5 = arg.get("MD5", 0).unwrap_or_default();
            let base64 =
                diagram.titled().skin.get_from_md5(md5).ok_or_else(|| {
                    CommandError::new(format!("Unknown MD5 sprite reference: {md5}"))
                })?;
            let image = PortableImage::read_base64(base64)
                .ok_or_else(|| CommandError::new("Cannot decode Base64 PNG sprite."))?;
            add_sprite(diagram, arg, Rc::new(SpriteImage::new(image)));
            Ok(())
        },
    )))
}

/// `sprite $name <svg ...>...</svg>` on one line (PlantUML's `CommandSpriteSvg`).
pub(super) fn svg<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    single_line_named(
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "SVG", r"(\<svg\b.*\</svg\>)"),
        |diagram: &mut D, arg: &RegexResult| {
            let svg = arg.get("SVG", 0).unwrap_or_default().to_owned();
            add_sprite(diagram, arg, Rc::new(SvgNanoParser::new(svg)));
            Ok(())
        },
    )
}

/// `sprite $name #library#exported`, a gray-level sprite of the standard library, which its `!include`s
/// declare (PlantUML's `CommandSpriteStdlib`).
pub(super) fn stdlib<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    let pattern = sprite_pattern(
        r"\$",
        NAME,
        vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(2, "STDLIB", "#([^#]+)#([^%s]+)"),
        ],
    );
    Box::new(SingleLine(PatternCommand::new(
        pattern,
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            add_stdlib_sprite(diagram, arg, Stdlib::read_sprite)
        },
    )))
}

/// `sprite $name :library:exported`, an SVG sprite of the standard library (PlantUML's
/// `CommandSpriteStdlibSvg`).
pub(super) fn stdlib_svg<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    single_line_named(
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(2, "STDLIB", ":([^:]+):([^%s]+)"),
        |diagram: &mut D, arg: &RegexResult| {
            add_stdlib_sprite(diagram, arg, Stdlib::read_svg_sprite)
        },
    )
}

/// `sprite $name <svg ...>` up to a line ending in `</svg>` (PlantUML's `CommandSpriteSvgMultiline`). The lines
/// join without separators, the first trimmed and the others as written.
pub(super) fn svg_multi_line<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    static START: LazyLock<RegexTree> = LazyLock::new(|| {
        sprite_named(
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "SVGSTART", r"(\<svg\b.*)"),
        )
    });
    static END: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"(.*\</svg\>)$"));
    Box::new(Multiline::starting_with(
        &START,
        &END,
        |diagram: &mut D, lines: &BlocLines| {
            let first = lines.first().expect("a block has lines").trimmed();
            let arg = START
                .matcher(first.text())
                .expect("checked when the block was recognised");
            let mut svg = arg.get("SVGSTART", 0).unwrap_or_default().to_owned();
            for line in lines.sub_extract(1, 0).iter() {
                svg.push_str(line.text());
            }
            add_sprite(diagram, &arg, Rc::new(SvgNanoParser::new(svg)));
            Ok(())
        },
    ))
}

/// `sprite $name jar:archimate/actor`, one of PlantUML's built-in sprites (PlantUML's `CommandSpriteFile`).
/// Image files and zip entries, the command's other sources, are not ported yet.
pub(super) fn file<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    single_line_named(
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "FILE", "([^<>%g#]*)"),
        |diagram: &mut D, arg: &RegexResult| {
            let src = arg.get("FILE", 0).unwrap_or_default();
            let Some(name) = src.strip_prefix("jar:") else {
                return Err(CommandError::new(format!("Cannot read: {src}")));
            };
            let sprite = SpriteImage::from_internal(name)
                .ok_or_else(|| CommandError::new(format!("No such internal sprite: {name}")))?;
            add_sprite(diagram, arg, sprite);
            Ok(())
        },
    )
}

fn sprite_named(separator: RegexTree, source: RegexTree) -> RegexTree {
    sprite_pattern(OPTIONAL_DOLLAR, NAME, vec![separator, source])
}

/// Each command spells the name, and whether its dollar is optional, its own way.
fn sprite_pattern(dollar: &'static str, name: &'static str, rest: Vec<RegexTree>) -> RegexTree {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::leaf("sprite"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf(dollar),
        RegexTree::named(1, "NAME", name),
    ];
    parts.extend(rest);
    parts.push(RegexTree::end());
    RegexTree::concat(parts)
}

fn single_line_named<D: TitledDiagram + 'static>(
    separator: RegexTree,
    source: RegexTree,
    apply: fn(&mut D, &RegexResult) -> CommandResult,
) -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        sprite_named(separator, source),
        move |diagram: &mut D, _: &LineLocation, arg: &RegexResult| apply(diagram, arg),
    )))
}

/// PlantUML fails on an unknown library, and draws nothing for an unknown sprite.
fn add_stdlib_sprite<D: TitledDiagram>(
    diagram: &mut D,
    arg: &RegexResult,
    read: fn(&Stdlib, &str) -> Option<Rc<dyn Sprite>>,
) -> CommandResult {
    let library = arg.get("STDLIB", 0).unwrap_or_default();
    let library = Stdlib::retrieve(library)
        .ok_or_else(|| CommandError::new(format!("Cannot read sprite: no library {library}")))?;
    if let Some(sprite) = read(&library, arg.get("STDLIB", 1).unwrap_or_default()) {
        add_sprite(diagram, arg, sprite);
    }
    Ok(())
}

/// `sprite $name`, an optional size and encoding, then `ending`.
fn declaration(dimension: &'static str, ending: Vec<RegexTree>) -> RegexTree {
    let mut rest = vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::named(5, "DIM", dimension)),
    ];
    rest.extend(ending);
    sprite_pattern(OPTIONAL_DOLLAR, NAME_WITH_DOTS, rest)
}

/// Without a size, a sprite has 16 gray levels and the size of its text.
fn execute_internal<D: TitledDiagram>(
    diagram: &mut D,
    arg: &RegexResult,
    strings: &[String],
) -> CommandResult {
    let sprite: Rc<dyn Sprite> = if arg.get("DIM", 0).is_none() {
        Rc::new(SpriteGrayLevel::Gray16.build_sprite(0, 0, strings))
    } else {
        let (width, height) = (declared_size(arg, 0)?, declared_size(arg, 1)?);
        if arg.get("DIM", 4).is_some() {
            Rc::new(SpriteColorBuilder4096::build_sprite(strings))
        } else {
            Rc::new(gray_level_sprite(arg, width, height, strings)?)
        }
    };
    add_sprite(diagram, arg, sprite);
    Ok(())
}

/// A width or height, which PlantUML reads as an `int`.
fn declared_size(arg: &RegexResult, index: usize) -> Result<usize, CommandError> {
    arg.get("DIM", index)
        .and_then(|text| text.parse::<i32>().ok())
        .and_then(|size| usize::try_from(size).ok())
        .ok_or_else(sprite_too_large)
}

fn gray_level_sprite(
    arg: &RegexResult,
    width: usize,
    height: usize,
    strings: &[String],
) -> Result<SpriteMonochrome, CommandError> {
    let level = arg
        .get("DIM", 2)
        .and_then(|text| text.parse().ok())
        .and_then(SpriteGrayLevel::get)
        .ok_or_else(|| CommandError::new("Only 4, 8 or 16 graylevel are allowed."))?;
    if !is_acceptable_size(width, height) {
        return Err(sprite_too_large());
    }
    if arg.get("DIM", 3).is_none() {
        return Ok(level.build_sprite(width, height, strings));
    }
    let compressed: String = strings.iter().map(|text| crate::java::trim(text)).collect();
    level
        .build_sprite_z(width, height, &compressed)
        .ok_or_else(|| CommandError::new("Cannot decode sprite."))
}

/// Where PlantUML would run out of memory, or fail to parse the size.
fn sprite_too_large() -> CommandError {
    CommandError::new("Sprite too large.")
}

fn add_sprite<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, sprite: Rc<dyn Sprite>) {
    let name = arg.get("NAME", 0).unwrap_or_default().to_owned();
    diagram.titled().skin.add_sprite(name, sprite);
}
