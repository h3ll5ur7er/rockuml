//! The line joining the arrows out of a conditional (PlantUML's `ConnectionHline`, in `FtileIfWithLinks`
//! and `FtileIfLongHorizontal`); drawn one swimlane at a time, it spans only the lanes those arrows end in.

use std::rc::Rc;

use super::UGraphicInterceptorOneSwimlane;
use crate::diagram::activity3::SwimlaneId;
use crate::ftile::Ftile;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::ugraphic::UGraphic;

/// From where to where the horizontal line joining the arrows out of `all_tiles` runs, `width` being the
/// width of the conditional; `None` where it draws no line (PlantUML's `ConnectionHline.getMinmax` and
/// `getMinmaxSimple`, which answer `NaN`). `left_out`, when given, is a point the line also reaches.
pub(super) fn hline_extent(
    ug: &UGraphic,
    width: f64,
    all_tiles: &[Rc<dyn Ftile>],
    left_out: Option<f64>,
    get_translate_for: impl Fn(&dyn Ftile, &dyn StringBounder) -> UTranslate,
) -> Option<(f64, f64)> {
    let string_bounder = ug.string_bounder();
    let interceptor = ug.layer::<UGraphicInterceptorOneSwimlane>();
    let (mut min_x, mut max_x) = match interceptor {
        Some(interceptor) => {
            let all = interceptor.get_ordered_list_of_all_swimlanes();
            let current = all
                .iter()
                .position(|other| *other == interceptor.get_swimlane())?;
            let has_out_in = |into: SwimlaneId| {
                all_tiles
                    .iter()
                    .any(|tile| has_point_out(tile, string_bounder) && outcome_in(tile, into))
            };
            let first = all.iter().position(|into| has_out_in(*into))?;
            let last = all.iter().rposition(|into| has_out_in(*into))?;
            if current < first || current > last {
                return None;
            }
            (
                if current == first { width } else { 0.0 },
                if current == last { 0.0 } else { width },
            )
        }
        None => (width / 2.0, width / 2.0),
    };
    if let Some(left_out) = left_out {
        min_x = min_x.min(left_out);
        max_x = max_x.max(left_out);
    }
    for tile in all_tiles {
        if !has_point_out(tile, string_bounder) {
            continue;
        }
        if let Some(interceptor) = interceptor
            && !outcome_in(tile, interceptor.get_swimlane())
        {
            continue;
        }
        let translate = get_translate_for(tile.as_ref(), string_bounder);
        let out = tile
            .calculate_dimension(string_bounder)
            .translate(translate)
            .get_left();
        min_x = min_x.min(out);
        max_x = max_x.max(out);
    }
    Some((min_x, max_x))
}

fn has_point_out(tile: &Rc<dyn Ftile>, string_bounder: &dyn StringBounder) -> bool {
    tile.calculate_dimension(string_bounder).has_point_out()
}

/// `ftileDoesOutcomeInThatSwimlane`.
fn outcome_in(tile: &Rc<dyn Ftile>, swimlane: SwimlaneId) -> bool {
    tile.get_swimlane_out() == Some(swimlane) && tile.get_swimlanes().contains(&Some(swimlane))
}
