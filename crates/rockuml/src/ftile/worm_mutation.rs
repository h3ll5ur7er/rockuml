//! How the lines of a multi-colour arrow are offset from each other (PlantUML's `WormMutation`): one
//! translation per point of the worm, following its bends.

use super::Worm;
use crate::klimt::geom::UTranslate;

pub(crate) struct WormMutation {
    translations: Vec<UTranslate>,
}

impl WormMutation {
    /// The offsets for `worm`, `delta` apart.
    pub(crate) fn create(worm: &Worm, delta: f64) -> Self {
        let signature = worm.get_directions_code();
        match get_definition(&signature) {
            Some(definition) => Self::from_definition(definition, delta),
            None => Self::create_from_long_signature(&signature, delta),
        }
    }

    /// Chains the offsets of each bend, turning a bend's offsets round when they do not continue the
    /// previous one. PlantUML fails on a worm going straight on between two bends; here such a bend is
    /// skipped, and points without an offset stay put.
    fn create_from_long_signature(signature: &str, delta: f64) -> Self {
        let mut result = Self {
            translations: Vec::new(),
        };
        let bends = signature.len().saturating_sub(1);
        for i in 0..bends {
            let Some(definition) = get_definition(&signature[i..i + 2]) else {
                continue;
            };
            let mut tmp = Self::from_definition(definition, delta);
            if i == 0 {
                result.translations.push(tmp.translations[0]);
            } else if !result.get_last().is_almost_same(tmp.translations[0]) {
                tmp = tmp.reverse();
            }
            result.translations.push(tmp.translations[1]);
            if i == bends - 1 {
                result.translations.push(tmp.translations[2]);
            }
        }
        result
    }

    fn from_definition(definition: &str, delta: f64) -> Self {
        Self {
            translations: definition
                .bytes()
                .map(|digit| translation(digit - b'0', delta))
                .collect(),
        }
    }

    fn reverse(&self) -> Self {
        Self {
            translations: self.translations.iter().map(|tr| tr.reverse()).collect(),
        }
    }

    pub(crate) fn get_last(&self) -> UTranslate {
        self.translations.last().copied().unwrap_or_default()
    }

    pub(crate) fn get_first(&self) -> UTranslate {
        self.translations.first().copied().unwrap_or_default()
    }

    pub(crate) fn size(&self) -> usize {
        self.translations.len()
    }

    /// Where the labels of an arrow of `size` colours go: beyond the outermost line.
    pub(crate) fn get_text_translate(&self, size: usize) -> UTranslate {
        // PlantUML starts the maximum at `Double.MIN_VALUE`, the smallest positive double.
        let (mut min, mut max) = (f64::MAX, 4.9e-324_f64);
        for tr in &self.translations {
            if tr.dx > max {
                max = tr.dx;
            }
            if tr.dx < min {
                min = tr.dx;
            }
        }
        let extreme = if max.abs() > min.abs() { max } else { min };
        UTranslate::new(extreme * (size as f64 - 1.0), 0.0)
    }

    pub(crate) fn is_dx_negative(&self) -> bool {
        self.get_first().dx < 0.0
    }

    /// `original` with each point moved by its offset (`mute`).
    pub(crate) fn mute(&self, original: &Worm) -> Worm {
        let mut result = original.clone_empty();
        for i in 0..original.size() {
            let translation = self.translations.get(i).copied().unwrap_or_default();
            result.add_point_at(translation.get_translated(original.get_point(i)));
        }
        result
    }
}

/// The offsets of a straight segment or a single bend, as digits naming a direction clockwise from up
/// (`getDefinition`).
fn get_definition(signature: &str) -> Option<&'static str> {
    Some(match signature {
        "D" | "U" => "33",
        "L" | "R" => "55",
        "RD" => "123",
        "RU" => "543",
        "LD" => "187",
        "DL" => "345",
        "DR" => "765",
        "UL" => "321",
        "UR" => "781",
        _ => return None,
    })
}

fn translation(kind: u8, delta: f64) -> UTranslate {
    match kind {
        1 => UTranslate::new(0.0, -delta),
        2 => UTranslate::new(delta, -delta),
        3 => UTranslate::new(delta, 0.0),
        4 => UTranslate::new(delta, delta),
        5 => UTranslate::new(0.0, delta),
        6 => UTranslate::new(-delta, delta),
        7 => UTranslate::new(-delta, 0.0),
        _ => UTranslate::new(-delta, -delta),
    }
}
