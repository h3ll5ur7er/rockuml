//! Packet diagrams, `@startpacketdiag` (PlantUML's `packetdiag` package): fields laid out bit by bit in
//! rows under a numbered bit scale.

mod block;
mod commands;
mod indicator;

use std::rc::Rc;

use block::PacketBlock;
use indicator::PacketIndicator;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::Command;
use crate::command::factory::AbstractDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// The bits in a row unless `colwidth` says otherwise.
const DEFAULT_COL_WIDTH: i32 = 16;

/// Which way the bits are numbered.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScaleDirection {
    Ltr,
    Rtl,
}

/// A field as declared: `width` bits up to `bit_end` (`PacketItem`).
struct PacketItem {
    width: i32,
    height: i32,
    bit_end: i32,
    desc: String,
}

impl PacketItem {
    fn of_range(start: i32, end: i32, height: i32, desc: &str) -> Self {
        Self {
            width: end.wrapping_sub(start).wrapping_add(1),
            height,
            bit_end: end,
            desc: desc.to_owned(),
        }
    }

    fn to_packet_block(&self) -> PacketBlock {
        PacketBlock::new(self.width, self.height, &self.desc)
    }

    fn to_packet_block_width(&self, new_width: i32) -> PacketBlock {
        PacketBlock::new(new_width.min(self.width), self.height, &self.desc)
    }
}

pub(super) struct PacketDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    col_width: i32,
    /// How wide a bit is drawn.
    bit_width: f64,
    /// How high a field one unit high is drawn.
    bit_height: f64,
    /// How often the scale is numbered, in bits.
    scale_interval: i32,
    use_default_scale_interval: bool,
    /// Whether the fields of a row all take the tallest one's height.
    same_height: bool,
    scale_direction: ScaleDirection,
    packet_items: Vec<PacketItem>,
    packet_grid: Vec<Vec<PacketBlock>>,
    packet_indicators: Vec<PacketIndicator>,
}

/// Reads packet diagrams (PlantUML's `PacketDiagramFactory`).
pub(super) struct PacketDiagramFactory;

impl CommandFactory for PacketDiagramFactory {
    type Diagram = PacketDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Packet;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> PacketDiagram {
        PacketDiagram {
            source: source.clone(),
            titled: Titled::new(SName::PacketdiagDiagram, "PACKET", source),
            col_width: DEFAULT_COL_WIDTH,
            bit_width: 24.0,
            bit_height: 32.0,
            scale_interval: DEFAULT_COL_WIDTH / 2,
            use_default_scale_interval: true,
            same_height: false,
            scale_direction: ScaleDirection::Ltr,
            packet_items: Vec::new(),
            packet_grid: Vec::new(),
            packet_indicators: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<PacketDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend(commands::all());
        commands
    }
}

impl PacketDiagram {
    /// Widths of 0 or less are ignored.
    fn set_col_width(&mut self, col_width: i32) {
        if col_width > 0 {
            self.col_width = col_width;
        }
    }

    /// Bits between full ticks.
    fn full_indicator_interval(&self) -> i32 {
        if self.col_width >= 4 {
            self.col_width / 4
        } else {
            self.col_width
        }
    }

    /// Intervals of 0 or less are ignored; others are kept within the row's width.
    fn update_scale_interval(&mut self, value: i32) {
        if value > 0 {
            self.scale_interval = value.min(self.col_width);
            self.use_default_scale_interval = false;
        }
    }

    fn update_node_height(&mut self, node_height: i32) {
        self.bit_height = f64::from(node_height.max(0));
    }

    fn add_packet_item_range(&mut self, start: i32, end: i32, height: i32, desc: &str) {
        self.packet_items
            .push(PacketItem::of_range(start, end, height, desc));
    }

    fn last_packet_end(&self) -> Option<i32> {
        self.packet_items.last().map(|item| item.bit_end)
    }

    fn style(&self) -> Style {
        self.style_of(&[SName::Root, SName::Element, SName::PacketdiagDiagram])
    }

    /// The fields' style (`PacketBlock.getStyle`).
    fn block_style(&self) -> Style {
        self.style_of(&[
            SName::Root,
            SName::Element,
            SName::PacketdiagDiagram,
            SName::Rectangle,
        ])
    }

    fn style_of(&self, names: &[SName]) -> Style {
        StyleSignature::of(names).get_merged_style(&self.titled.skin.current_style_builder())
    }

    fn build(&mut self) -> Result<(), NotYetPorted> {
        if self.use_default_scale_interval {
            self.update_scale_interval(self.col_width / 2);
        }
        self.adjust_layout();
        self.adjust_col_width();
        self.adjust_indicators()?;
        self.adjust_bit_width();
        Ok(())
    }

    /// No wider than the first row.
    fn adjust_col_width(&mut self) {
        if let Some(first) = self.packet_grid.first() {
            let max_width = first.iter().map(|block| block.width).sum();
            self.col_width = self.col_width.min(max_width);
        }
    }

    /// PlantUML fails on a scale of no bits, numbering it by a remainder by zero, and on one of fewer, of
    /// which it cannot take that many indicators; only fields ending before they start make those.
    fn adjust_indicators(&mut self) -> Result<(), NotYetPorted> {
        if self.col_width == 0 || self.col_width < -1 {
            return Err(NotYetPorted(
                "a first row of no bits or fewer, where PlantUML crashes",
            ));
        }
        let full_length_interval = self.full_indicator_interval();
        let indicator = |i| PacketIndicator {
            full: i % full_length_interval == 0,
            numbered: i % self.scale_interval == 0,
            bit_number: i,
        };
        self.packet_indicators = match self.scale_direction {
            ScaleDirection::Ltr => (0..=self.col_width).map(indicator).collect(),
            ScaleDirection::Rtl => (0..=self.col_width).rev().map(indicator).collect(),
        };
        Ok(())
    }

    /// Lays the fields out in rows, splitting those that overflow their row (`adjustLayout`). A field
    /// continuing on a row after full rows keeps the width of a full row there, as in PlantUML.
    fn adjust_layout(&mut self) {
        let col_width = self.col_width;
        let mut grid: Vec<Vec<PacketBlock>> = Vec::new();
        let mut curr_row = Vec::new();
        let mut remain_row_width = col_width;
        for packet in &self.packet_items {
            if let Some(rest) = fit_packet_in_row(packet, remain_row_width, &mut curr_row) {
                remain_row_width = col_width;
                grid.push(std::mem::take(&mut curr_row));
                let row_cnt = rest.width / remain_row_width;
                let remain = rest.width % remain_row_width;
                for _ in 0..row_cnt {
                    grid.push(vec![rest.to_packet_block_width(remain_row_width)]);
                }
                if remain > 0 {
                    curr_row.push(rest.to_packet_block_width(remain_row_width));
                    remain_row_width -= remain;
                }
            } else {
                remain_row_width -= packet.width;
                if remain_row_width == 0 {
                    remain_row_width = col_width;
                    grid.push(std::mem::take(&mut curr_row));
                }
            }
        }
        if !curr_row.is_empty() {
            grid.push(curr_row);
        }
        for blocks in &mut grid {
            if self.same_height {
                let max_h = blocks.iter().map(|block| block.height).max().unwrap_or(1);
                for block in blocks.iter_mut() {
                    block.height = max_h;
                }
            }
            if self.scale_direction == ScaleDirection::Rtl {
                blocks.reverse();
            }
        }
        self.packet_grid = grid;
    }

    /// A bit is three times as wide as the font is high.
    fn adjust_bit_width(&mut self) {
        const BIT_SCALE: f64 = 3.0;
        self.bit_width = self.style().value(PName::FontSize).as_double() * BIT_SCALE;
    }
}

/// Puts the field in the row, or as much of it as fits; returns what is left over.
fn fit_packet_in_row(
    packet: &PacketItem,
    remain_row_width: i32,
    row: &mut Vec<PacketBlock>,
) -> Option<PacketItem> {
    let overflow = packet.width - remain_row_width;
    if overflow > 0 {
        row.push(packet.to_packet_block_width(remain_row_width));
        Some(PacketItem {
            width: overflow,
            height: packet.height,
            bit_end: packet.bit_end,
            desc: packet.desc.clone(),
        })
    } else {
        row.push(packet.to_packet_block());
        None
    }
}

/// The scale, then the rows (`getTextBlock`).
struct Drawing<'a> {
    diagram: &'a PacketDiagram,
    style: Style,
    block_style: Style,
}

impl Drawing<'_> {
    fn shapes<'b>(
        &'b self,
        blocks: &'b [PacketBlock],
        string_bounder: &'b dyn StringBounder,
    ) -> impl Iterator<Item = block::PacketShape> + 'b {
        let diagram = self.diagram;
        blocks.iter().map(move |block| {
            block.shape_text_block(
                &self.block_style,
                &diagram.titled.skin,
                string_bounder,
                diagram.bit_width,
                diagram.bit_height,
            )
        })
    }
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let diagram = self.diagram;
        let mut indicator_total_width = 0.0;
        let mut indicator_max_height: f64 = 0.0;
        for indicator in &diagram.packet_indicators {
            let dim =
                indicator.calculate_dimension(&self.style, &diagram.titled.skin, string_bounder);
            indicator_total_width += diagram.bit_width;
            indicator_max_height = dim.height.max(indicator_max_height);
        }
        let mut grid_max_width: f64 = 0.0;
        let mut grid_total_height = 0.0;
        for blocks in &diagram.packet_grid {
            let mut row_width = 0.0;
            let mut row_height = 0.0;
            for shape in self.shapes(blocks, string_bounder) {
                let dim = shape.calculate_dimension(string_bounder);
                row_width += dim.width;
                row_height = dim.height;
            }
            grid_max_width = grid_max_width.max(row_width);
            grid_total_height += row_height;
        }
        XDimension2D::new(
            f64::max(indicator_total_width, grid_max_width),
            indicator_max_height + grid_total_height,
        )
    }

    fn draw_u(&self, origin: &UGraphic) {
        let diagram = self.diagram;
        let string_bounder = origin.string_bounder();
        let mut ug = origin.clone();
        let mut max_width = 0.0;
        let mut max_height: f64 = 0.0;
        for indicator in &diagram.packet_indicators {
            indicator.draw_u(&ug, &self.style, &diagram.titled.skin);
            ug = ug.translated(diagram.bit_width, 0.0);
            let dim =
                indicator.calculate_dimension(&self.style, &diagram.titled.skin, string_bounder);
            max_width += diagram.bit_width;
            max_height = dim.height.max(max_height);
        }
        ug = ug.translated(-max_width, max_height);
        for blocks in &diagram.packet_grid {
            let mut total_width = 0.0;
            let mut draw_height = 0.0;
            for shape in self.shapes(blocks, string_bounder) {
                let dim = shape.calculate_dimension(string_bounder);
                total_width += dim.width;
                draw_height = dim.height;
                shape.draw_u(&ug);
                ug = ug.translated(dim.width, 0.0);
            }
            ug = ug.translated(-total_width, draw_height);
        }
    }
}

impl AbstractDiagram for PacketDiagram {
    fn check_final_error(&mut self) -> Option<String> {
        if let Err(crash) = self.build() {
            self.titled.not_ported(crash);
        }
        None
    }
}

impl TitledDiagram for PacketDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for PacketDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        // PlantUML measures a field ending before it starts as negatively wide, which `XDimension2D` rejects.
        let mut blocks = self.packet_grid.iter().flatten();
        if blocks.any(|block| block.draw_width(self.bit_width) < 0.0) {
            return Err(NotYetPorted(
                "a field ending before it starts, where PlantUML crashes",
            ));
        }
        let drawing = Drawing {
            diagram: self,
            style: self.style(),
            block_style: self.block_style(),
        };
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::test_support::not_ported_of;

    fn diagram(col_width: i32, ranges: &[(i32, i32)]) -> PacketDiagram {
        let source = Rc::new(UmlSource::new(Vec::new(), Vec::new()));
        let mut diagram = PacketDiagramFactory::create_empty_diagram(&source);
        diagram.set_col_width(col_width);
        for &(start, end) in ranges {
            diagram.add_packet_item_range(start, end, 1, "");
        }
        diagram.build().unwrap();
        diagram
    }

    fn row_widths(diagram: &PacketDiagram) -> Vec<Vec<i32>> {
        let rows = diagram.packet_grid.iter();
        rows.map(|row| row.iter().map(|block| block.width).collect())
            .collect()
    }

    #[test]
    fn fields_overflowing_their_row_continue_on_the_next_rows() {
        let diagram = diagram(8, &[(0, 5), (6, 21), (22, 40), (41, 47), (48, 60)]);
        assert_eq!(
            row_widths(&diagram),
            [
                vec![6, 2],
                vec![8],
                vec![8, 2],
                vec![8],
                vec![8],
                vec![8, 7],
                vec![8],
                vec![5]
            ]
        );
    }

    #[test]
    fn the_scale_is_no_wider_than_the_first_row() {
        let diagram = diagram(16, &[(0, 3)]);
        assert_eq!(diagram.col_width, 4);
        assert_eq!(diagram.packet_indicators.len(), 5);
        assert_eq!(diagram.scale_interval, 8);
    }

    /// Whether PlantUML crashes on the packet diagram of these fields.
    fn crashes(fields: &str) -> bool {
        not_ported_of(&format!("@startpacketdiag\n{fields}\n@endpacketdiag")).is_some()
    }

    #[test]
    fn fields_ending_before_they_start_are_plantuml_crashes() {
        assert!(crashes("5-3: neg\n0-7: x"));
        assert!(crashes("0-7: x\n9-3: neg"));
        assert!(crashes("999999-0: back"));
        assert!(!crashes("0-7: x"));
    }

    #[test]
    fn a_first_row_without_width_is_a_plantuml_crash() {
        assert!(crashes("5-4: zero"));
        assert!(!crashes("5-4: zero\n0-7: x"));
    }
}
