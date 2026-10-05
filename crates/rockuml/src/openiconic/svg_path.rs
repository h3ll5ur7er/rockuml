//! The path data of an `OpenIconic` icon, turned into PlantUML's path segments.

use crate::klimt::geom::UTranslate;
use crate::klimt::shape::USegment;

pub struct SvgPath {
    movements: Vec<Movement>,
    translate: UTranslate,
}

impl SvgPath {
    pub fn new(path: &str, translate: UTranslate) -> Self {
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

    /// The path scaled by `factor`; closing a subpath adds nothing, as in PlantUML.
    pub fn to_upath(&self, factor: f64) -> Vec<USegment> {
        let scaled = |position: SvgPosition| (position.x * factor, position.y * factor);
        let path = self.movements.iter().filter_map(|movement| {
            let arguments = &movement.arguments;
            let end = movement.last_position().map(scaled);
            match (movement.letter, end) {
                ('Z', None) => None,
                ('M', Some((x, y))) => Some(USegment::MoveTo(x, y)),
                ('L', Some((x, y))) => Some(USegment::LineTo(x, y)),
                ('C', Some(end)) => Some(USegment::CubicTo {
                    ctrl1: scaled(movement.position(0)),
                    ctrl2: scaled(movement.position(2)),
                    end,
                }),
                ('A', Some(end)) => Some(USegment::ArcTo {
                    radius: (arguments[0] * factor, arguments[1] * factor),
                    x_axis_rotation: arguments[2],
                    large_arc: arguments[3] != 0.0,
                    sweep: arguments[4] != 0.0,
                    end,
                }),
                (letter, _) => unreachable!("absolute paths have no {letter}"),
            }
        });
        let (dx, dy) = (self.translate.dx * factor, self.translate.dy * factor);
        // Translating by zero would turn -0 into 0, which PlantUML prints differently.
        if dx == 0.0 && dy == 0.0 {
            path.collect()
        } else {
            path.map(|segment| segment.translate(dx, dy)).collect()
        }
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
            'z' => Self::through('Z', &[]),
            'c' => Self::through('C', &[relative(0), relative(2), relative(4)]),
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
            letter => unreachable!("no OpenIconic icon draws with {letter}"),
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

fn argument_number(letter: char) -> usize {
    match letter.to_ascii_lowercase() {
        'm' | 'l' => 2,
        'h' | 'v' => 1,
        'z' => 0,
        'c' => 6,
        's' => 4,
        'a' => 7,
        _ => unreachable!("no OpenIconic icon draws with {letter}"),
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
            SvgCommand::Number(_) => (
                last_letter.expect("a path starts with a command letter"),
                rest,
            ),
        };
        let (arguments, tail) = arguments.split_at(argument_number(letter));
        movements.push(Movement {
            letter,
            arguments: arguments
                .iter()
                .map(|command| match command {
                    SvgCommand::Number(number) => *number,
                    SvgCommand::Letter(letter) => panic!("{letter} where a number belongs"),
                })
                .collect(),
        });
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
        let number = &input[..length];
        if !number.chars().any(|c| c.is_ascii_digit()) {
            break;
        }
        commands.push(SvgCommand::Number(
            number.parse().expect("digits with at most one dot"),
        ));
        input = &input[length..];
    }
    commands
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
}
