//! The path data of an `OpenIconic` icon, an emoji or an SVG sprite, turned into PlantUML's path segments.

use crate::klimt::affine::XAffineTransform;
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::USegment;

pub(crate) struct SvgPath {
    movements: Vec<Movement>,
    translate: UTranslate,
}

impl SvgPath {
    pub(crate) fn new(path: &str, translate: UTranslate) -> Self {
        let mut movements = Vec::new();
        let mut last = SvgPosition::ORIGIN;
        let mut last_move = SvgPosition::ORIGIN;
        let mut mirror_control_point = None;
        for movement in parse_movements(&decipher(path)) {
            let mut movement = movement.into_absolute_upper_case(last);
            if movement.letter == 'Z' {
                last = last_move;
            }
            if movement.letter == 'S' {
                movement = movement.muto_to_c(mirror_control_point);
            }
            if let Some(position) = movement.last_position() {
                if movement.letter == 'M' {
                    last_move = position;
                }
                last = position;
            }
            mirror_control_point = movement.mirror_control_point();
            movements.push(movement);
        }
        Self {
            movements,
            translate,
        }
    }

    /// The path scaled by `factor`.
    pub(crate) fn to_upath(&self, factor: f64) -> Vec<USegment> {
        self.build_upath(
            |position| (position.x * factor, position.y * factor),
            (factor, factor),
        )
    }

    /// The path transformed by `at`. Arcs keep their rotation and only stretch along the axes.
    pub(crate) fn to_upath_affine(&self, at: &XAffineTransform) -> Vec<USegment> {
        self.build_upath(
            |position| at.transform((position.x, position.y)),
            (at.scale_x(), at.scale_y()),
        )
    }

    /// Closing a subpath adds nothing, as in PlantUML. Quadratic curves become cubic ones with both control
    /// points on the quadratic one.
    fn build_upath(
        &self,
        point: impl Fn(SvgPosition) -> (f64, f64),
        (scale_x, scale_y): (f64, f64),
    ) -> Vec<USegment> {
        let mut path = Vec::new();
        let mut previous: Option<&Movement> = None;
        for movement in &self.movements {
            let arguments = &movement.arguments;
            let quadratic_to = |control, end| USegment::CubicTo {
                ctrl1: point(control),
                ctrl2: point(control),
                end,
            };
            match (movement.letter, movement.last_position().map(&point)) {
                ('Z', None) => {}
                ('M', Some((x, y))) => move_to(&mut path, x, y),
                ('L', Some((x, y))) => path.push(USegment::LineTo(x, y)),
                ('C', Some(end)) => path.push(USegment::CubicTo {
                    ctrl1: point(movement.position(0)),
                    ctrl2: point(movement.position(2)),
                    end,
                }),
                ('Q', Some(end)) => path.push(quadratic_to(movement.position(0), end)),
                ('T', Some(end)) => {
                    path.push(quadratic_to(
                        smooth_quadratic_control(previous, movement),
                        end,
                    ));
                }
                ('A', Some(end)) => path.push(USegment::ArcTo {
                    radius: (arguments[0] * scale_x, arguments[1] * scale_y),
                    x_axis_rotation: arguments[2],
                    large_arc: arguments[3] != 0.0,
                    sweep: arguments[4] != 0.0,
                    end,
                }),
                (letter, _) => unreachable!("absolute paths have no {letter}"),
            }
            previous = Some(movement);
        }
        let (dx, dy) = (self.translate.dx * scale_x, self.translate.dy * scale_y);
        // Translating by zero would turn -0 into 0, which PlantUML prints differently.
        if dx != 0.0 || dy != 0.0 {
            for segment in &mut path {
                *segment = segment.translate(dx, dy);
            }
        }
        path
    }
}

/// The previous control point mirrored through the previous end. PlantUML takes the first point of the previous
/// movement as its control point, which for a smooth quadratic curve is its end. Without a previous quadratic
/// curve PlantUML fails; the control point is then the curve's own end.
fn smooth_quadratic_control(previous: Option<&Movement>, movement: &Movement) -> SvgPosition {
    match previous {
        Some(previous) if matches!(previous.letter, 'Q' | 'T') => previous
            .last_position()
            .expect("curves have an end")
            .mirror(previous.position(0)),
        _ => movement.position(0),
    }
}

/// PlantUML's `UPath.moveTo` skips a move to the first point of the previous segment, which for a curve is its
/// first control point.
fn move_to(path: &mut Vec<USegment>, x: f64, y: f64) {
    let first_point = |segment: &USegment| match *segment {
        USegment::MoveTo(x, y) | USegment::LineTo(x, y) => (x, y),
        USegment::CubicTo { ctrl1, .. } => ctrl1,
        USegment::ArcTo { radius, .. } => radius,
    };
    if path.last().map(first_point) != Some((x, y)) {
        path.push(USegment::MoveTo(x, y));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SvgPosition {
    x: f64,
    y: f64,
}

impl SvgPosition {
    const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    /// The point opposite `to_be_mirrored` with this one in the middle.
    fn mirror(self, to_be_mirrored: Self) -> Self {
        Self {
            x: 2.0 * self.x - to_be_mirrored.x,
            y: 2.0 * self.y - to_be_mirrored.y,
        }
    }
}

/// One command of the path with its arguments.
#[derive(Debug, PartialEq)]
struct Movement {
    letter: char,
    arguments: Vec<f64>,
}

impl Movement {
    fn through(letter: char, positions: &[SvgPosition]) -> Self {
        Self {
            letter,
            arguments: positions
                .iter()
                .flat_map(|position| [position.x, position.y])
                .collect(),
        }
    }

    fn position(&self, index: usize) -> SvgPosition {
        SvgPosition {
            x: self.arguments[index],
            y: self.arguments[index + 1],
        }
    }

    fn last_position(&self) -> Option<SvgPosition> {
        (!self.arguments.is_empty()).then(|| self.position(self.arguments.len() - 2))
    }

    /// The same movement in absolute coordinates, with horizontal and vertical lines as plain lines.
    fn into_absolute_upper_case(self, delta: SvgPosition) -> Self {
        let relative = |index| delta.add(self.position(index));
        let first = self.arguments.first().copied().unwrap_or_default();
        match self.letter {
            'H' => Self::through(
                'L',
                &[SvgPosition {
                    x: first,
                    y: delta.y,
                }],
            ),
            'V' => Self::through(
                'L',
                &[SvgPosition {
                    x: delta.x,
                    y: first,
                }],
            ),
            'h' => Self::through(
                'L',
                &[SvgPosition {
                    x: delta.x + first,
                    y: delta.y,
                }],
            ),
            'v' => Self::through(
                'L',
                &[SvgPosition {
                    x: delta.x,
                    y: delta.y + first,
                }],
            ),
            letter if letter.is_ascii_uppercase() => self,
            'm' => Self::through('M', &[relative(0)]),
            'l' => Self::through('L', &[relative(0)]),
            't' => Self::through('T', &[relative(0)]),
            'z' => Self::through('Z', &[]),
            'c' => Self::through('C', &[relative(0), relative(2), relative(4)]),
            'q' => Self::through('Q', &[relative(0), relative(2)]),
            's' => Self::through('S', &[relative(0), relative(2)]),
            'a' => {
                let end = relative(5);
                let mut arguments = self.arguments[..5].to_vec();
                arguments.extend([end.x, end.y]);
                Self {
                    letter: 'A',
                    arguments,
                }
            }
            letter => unreachable!("parse_movements keeps no {letter}"),
        }
    }

    /// PlantUML's reading of a smooth curve: without a previous curve, its first control point is its
    /// second one rather than the current point.
    fn muto_to_c(self, mirror_control_point: Option<SvgPosition>) -> Self {
        let control = self.position(0);
        let end = self.position(2);
        Self::through(
            'C',
            &[mirror_control_point.unwrap_or(control), control, end],
        )
    }

    /// Where a following smooth curve takes its first control point from.
    fn mirror_control_point(&self) -> Option<SvgPosition> {
        let end = self.last_position()?;
        (self.letter == 'C').then(|| end.mirror(self.position(2)))
    }
}

fn argument_number(letter: char) -> Option<usize> {
    match letter.to_ascii_lowercase() {
        'm' | 'l' | 't' => Some(2),
        'h' | 'v' => Some(1),
        'z' => Some(0),
        'c' => Some(6),
        'q' | 's' => Some(4),
        'a' => Some(7),
        _ => None,
    }
}

/// Numbers after a movement's own arguments repeat it, except that a move goes on as a line.
fn implicit(letter: char) -> char {
    match letter {
        'm' => 'l',
        'M' => 'L',
        other => other,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SvgCommand {
    Letter(char),
    Number(f64),
}

/// PlantUML fails on a malformed path; the movements before the fault are kept.
fn parse_movements(commands: &[SvgCommand]) -> Vec<Movement> {
    let mut movements = Vec::new();
    let mut last_letter = None;
    let mut rest = commands;
    while let Some((first, tail)) = rest.split_first() {
        let (letter, arguments) = match *first {
            SvgCommand::Letter(letter) => {
                last_letter = Some(implicit(letter));
                (letter, tail)
            }
            SvgCommand::Number(_) => match last_letter {
                Some(letter) => (letter, rest),
                None => break,
            },
        };
        let Some(count) = argument_number(letter).filter(|&count| count <= arguments.len()) else {
            break;
        };
        let (arguments, tail) = arguments.split_at(count);
        let Some(arguments) = arguments
            .iter()
            .map(|command| match *command {
                SvgCommand::Number(number) => Some(number),
                SvgCommand::Letter(_) => None,
            })
            .collect()
        else {
            break;
        };
        movements.push(Movement { letter, arguments });
        rest = tail;
    }
    movements
}

/// PlantUML's `StringDecipher`: the path's letters and numbers, without the separators between them.
fn decipher(path: &str) -> Vec<SvgCommand> {
    let mut commands = Vec::new();
    let mut input = path;
    loop {
        input = input.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        let Some(first) = input.chars().next() else {
            break;
        };
        if first.is_alphabetic() {
            commands.push(SvgCommand::Letter(first));
            input = &input[first.len_utf8()..];
            continue;
        }
        let sign = usize::from(first == '+' || first == '-');
        let mut seen_dot = false;
        let length = sign
            + input[sign..]
                .chars()
                .take_while(|&c| {
                    let accepted = c.is_ascii_digit() || (c == '.' && !seen_dot);
                    seen_dot |= c == '.';
                    accepted
                })
                .count();
        if !input[..length].chars().any(|c| c.is_ascii_digit()) {
            break;
        }
        let length = length + exponent_length(&input[length..]);
        commands.push(SvgCommand::Number(
            input[..length]
                .parse()
                .expect("digits with at most one dot and an exponent"),
        ));
        input = &input[length..];
    }
    commands
}

/// The length of the scientific notation exponent `input` starts with, like `e-3`; zero when it has no digits.
fn exponent_length(input: &str) -> usize {
    let Some(rest) = input.strip_prefix(['e', 'E']) else {
        return 0;
    };
    let sign = usize::from(rest.starts_with(['+', '-']));
    let digits = rest[sign..]
        .chars()
        .take_while(char::is_ascii_digit)
        .count();
    if digits == 0 { 0 } else { 1 + sign + digits }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_run_together_where_a_sign_or_second_dot_starts_the_next() {
        assert_eq!(
            decipher("M2 0c-.55 0-1.04.23z"),
            [
                SvgCommand::Letter('M'),
                SvgCommand::Number(2.0),
                SvgCommand::Number(0.0),
                SvgCommand::Letter('c'),
                SvgCommand::Number(-0.55),
                SvgCommand::Number(0.0),
                SvgCommand::Number(-1.04),
                SvgCommand::Number(0.23),
                SvgCommand::Letter('z'),
            ]
        );
    }

    #[test]
    fn numbers_take_an_exponent_only_with_digits() {
        assert_eq!(
            decipher("6e-3 1E2e"),
            [
                SvgCommand::Number(0.006),
                SvgCommand::Number(100.0),
                SvgCommand::Letter('e'),
            ]
        );
    }

    #[test]
    fn repeated_arguments_repeat_the_command_and_a_move_goes_on_as_a_line() {
        let path = SvgPath::new("m1 1 2 0h1v1z", UTranslate::default());
        assert_eq!(
            path.to_upath(2.0),
            [
                USegment::MoveTo(2.0, 2.0),
                USegment::LineTo(6.0, 2.0),
                USegment::LineTo(8.0, 2.0),
                USegment::LineTo(8.0, 4.0),
            ]
        );
    }

    #[test]
    fn a_smooth_curve_mirrors_the_previous_control_point() {
        let path = SvgPath::new("M0 0c0 1 1 2 2 2s2-1 2-2", UTranslate::default());
        assert_eq!(
            path.to_upath(1.0)[2],
            USegment::CubicTo {
                ctrl1: (3.0, 2.0),
                ctrl2: (4.0, 1.0),
                end: (4.0, 0.0),
            }
        );
    }

    #[test]
    fn quadratic_curves_are_cubic_with_the_control_point_twice() {
        let path = SvgPath::new("M0 0q1 2 3 0t2 0T7 1", UTranslate::default());
        let quadratic = |control, end| USegment::CubicTo {
            ctrl1: control,
            ctrl2: control,
            end,
        };
        assert_eq!(
            path.to_upath(1.0)[1..],
            [
                quadratic((1.0, 2.0), (3.0, 0.0)),
                quadratic((5.0, -2.0), (5.0, 0.0)),
                quadratic((5.0, 0.0), (7.0, 1.0)),
            ],
            "a smooth curve after another one mirrors that one's end through itself"
        );
    }

    #[test]
    fn a_malformed_path_keeps_the_movements_before_the_fault() {
        let path = SvgPath::new("M0 0L1 1B2 2", UTranslate::default());
        assert_eq!(
            path.to_upath(1.0),
            [USegment::MoveTo(0.0, 0.0), USegment::LineTo(1.0, 1.0)]
        );
        assert_eq!(
            SvgPath::new("1 2L3", UTranslate::default()).to_upath(1.0),
            []
        );
    }

    #[test]
    fn arcs_keep_their_rotation_and_flags_and_move_only_their_end() {
        let path = SvgPath::new("M1 1a1 2 30 0 1 2 0z", UTranslate::new(1.0, 0.0));
        assert_eq!(
            path.to_upath(2.0)[1],
            USegment::ArcTo {
                radius: (2.0, 4.0),
                x_axis_rotation: 30.0,
                large_arc: false,
                sweep: true,
                end: (8.0, 2.0),
            }
        );
    }

    #[test]
    fn an_affine_transform_moves_every_point() {
        let path = SvgPath::new("M1 1L2 1", UTranslate::default());
        let at = XAffineTransform::new(0.0, 1.0, -1.0, 0.0, 10.0, 20.0);
        assert_eq!(
            path.to_upath_affine(&at),
            [USegment::MoveTo(9.0, 21.0), USegment::LineTo(9.0, 22.0)]
        );
    }

    #[test]
    fn a_move_to_the_first_point_of_the_previous_segment_is_skipped() {
        let path = SvgPath::new("M0 0C1 1 2 2 3 3M1 1L5 5M3 3", UTranslate::default());
        assert_eq!(
            path.to_upath(1.0),
            [
                USegment::MoveTo(0.0, 0.0),
                USegment::CubicTo {
                    ctrl1: (1.0, 1.0),
                    ctrl2: (2.0, 2.0),
                    end: (3.0, 3.0),
                },
                USegment::LineTo(5.0, 5.0),
                USegment::MoveTo(3.0, 3.0),
            ]
        );
    }
}
