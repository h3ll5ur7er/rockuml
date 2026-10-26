//! What the icon and sprite lists share with PlantUML's donors card, which is not ported itself.

use crate::klimt::TextBlock;
use crate::klimt::graphic_strings::GraphicStrings;

/// The lines cut into `nb_col` columns as even as can be, as if `reserved` more lines followed
/// (`PSystemDonors.getCols`).
pub(super) fn get_cols(
    lines: &[String],
    nb_col: usize,
    reserved: usize,
) -> Vec<Box<dyn TextBlock>> {
    let max_line = (lines.len() + (nb_col - 1) + reserved) / nb_col;
    lines
        .chunks(max_line.max(1))
        .map(|current| {
            Box::new(GraphicStrings::create_black_on_white(
                current.iter().cloned(),
            )) as Box<dyn TextBlock>
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::debug::StringBounderDebug;

    #[test]
    fn lines_fill_the_columns_in_turn() {
        let lines: Vec<String> = (0..10).map(|i| i.to_string()).collect();
        let heights: Vec<f64> = get_cols(&lines, 4, 0)
            .iter()
            .map(|column| column.calculate_dimension(&StringBounderDebug).height)
            .collect();
        assert_eq!(heights.len(), 4);
        assert!(
            heights[0] == heights[2] && heights[2] > heights[3],
            "{heights:?}"
        );
    }
}
