//! The drawing of a network diagram: one row per network, one column per server, the networks drawn as thin
//! tubes and the servers linked to them (PlantUML's `GridTextBlockSimple`, `GridTextBlockDecorated`,
//! `NServerDraw` and `VerticalLine`).

use std::collections::BTreeSet;

use super::NwDiagram;
use super::model::{NServer, NetworkId, NwGroup};
use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{MinMax, UTranslate, XDimension2D};
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::style::{PName, SName, StyleSignature, ValueReading};

pub(super) const NETWORK_THIN: f64 = 5.0;
const MINIMUM_WIDTH: f64 = 70.0;
/// The room above a server's box.
pub(super) const MAGIC: f64 = 15.0;
const MARGIN_AD: f64 = 10.0;
const MARGIN_BOX_W: f64 = 15.0;

/// A server in its cell: its box and the links to its networks.
pub(super) struct NServerDraw<'a> {
    diagram: &'a NwDiagram,
    server: &'a NServer,
    network: NetworkId,
    box_: Box<dyn TextBlock + 'a>,
    /// The networks the server is linked to and its address on each, in the networks' order.
    conns: Vec<(NetworkId, String)>,
    top_margin: f64,
}

impl<'a> NServerDraw<'a> {
    pub(super) fn new(
        diagram: &'a NwDiagram,
        server: &'a NServer,
        box_: Box<dyn TextBlock + 'a>,
        conns: Vec<(NetworkId, String)>,
        top_margin: f64,
    ) -> Self {
        Self {
            diagram,
            server,
            network: server.get_main_network_next(),
            box_,
            conns,
            top_margin,
        }
    }

    fn conn(&self, network: NetworkId) -> Option<&str> {
        self.conns
            .iter()
            .find(|(connected, _)| *connected == network)
            .map(|(_, address)| address.as_str())
    }

    fn is_linked_to(&self, network: NetworkId) -> bool {
        self.conn(network).is_some()
    }

    fn get_min_max(&self, string_bounder: &dyn StringBounder, width: f64, height: f64) -> MinMax {
        let x_middle = width / 2.0;
        let y_middle = height / 2.0;
        let dim_box = self.box_.calculate_dimension(string_bounder);
        let x1 = x_middle - dim_box.width / 2.0;
        let y1 = y_middle - dim_box.height / 2.0;
        let x2 = x_middle + dim_box.width / 2.0;
        let y2 = y_middle + dim_box.height / 2.0;
        MinMax::empty()
            .add_point(x1 - 5.0, y1 - 5.0)
            .add_point(x2 + 5.0, y2 + 5.0)
    }

    fn draw_me(&self, ug: &UGraphic, width: f64, height: f64) {
        draw_center(ug, Some(self.box_.as_ref()), width / 2.0, height / 2.0);
    }

    fn draw_links(&self, ug: &UGraphic, xstart: f64, width: f64, height: f64) {
        const SEVEN: f64 = 9.0;
        let ug = ug.translated(xstart, 0.0);
        let network = &self.diagram.networks[self.network];
        let ynet1 = network.y.get();
        let y_middle = height / 2.0;
        let string_bounder = ug.string_bounder();
        let dim_box = self.box_.calculate_dimension(string_bounder);
        let alpha = y_middle - dim_box.height / 2.0;
        let pos_link1 = f64::midpoint(y_middle - dim_box.height / 2.0 - self.top_margin, MAGIC);
        let x_middle = width / 2.0;
        let x_link_pos = width / 2.0;
        let skip: BTreeSet<OrderedY> = self
            .diagram
            .networks
            .iter()
            .filter(|n| xstart + x_middle > n.xmin.get() && xstart + x_middle < n.xmax.get())
            .map(|n| OrderedY(n.y.get()))
            .collect();
        if self.server.print_first_link {
            let line_ug = ug.translated(x_link_pos + network.magic_delta(), 0.0);
            if network.visible {
                vertical_line(&line_ug, ynet1 + NETWORK_THIN, ynet1 + alpha, &skip);
            } else {
                vertical_line(&line_ug, ynet1, ynet1 + alpha, &BTreeSet::new());
            }
        }
        let link = self
            .diagram
            .to_text_block(SName::Arrow, self.conn(self.network));
        draw_center(
            &ug,
            link.as_deref(),
            x_middle + network.magic_delta(),
            ynet1 + pos_link1,
        );
        let mut x = x_link_pos - (self.conns.len() as f64 - 2.0) * SEVEN / 2.0;
        let mut first = true;
        for (other, address) in &self.conns {
            if *other == self.network {
                continue;
            }
            let other_network = &self.diagram.networks[*other];
            let ynet2 = other_network.y.get();
            vertical_line(
                &ug.translated(x - other_network.magic_delta(), 0.0),
                ynet1 + y_middle + dim_box.height / 2.0,
                ynet2,
                &skip,
            );
            let block = self.diagram.to_text_block(SName::Arrow, Some(address));
            let xtext = match &block {
                Some(block) if first && self.conns.len() > 2 => {
                    x - block.calculate_dimension(string_bounder).width / 2.0
                }
                _ => x,
            };
            draw_center(
                &ug,
                block.as_deref(),
                xtext - other_network.magic_delta(),
                ynet2 - alpha / 2.0,
            );
            x += SEVEN;
            first = false;
        }
    }

    /// The address on the network after the server's main one.
    fn link2(&self) -> Option<std::rc::Rc<dyn TextBlock>> {
        let networks = &self.diagram.networks;
        if self.network + 1 >= networks.len() {
            return None;
        }
        self.diagram
            .to_text_block(SName::Arrow, self.conn(self.network + 1))
    }

    fn natural_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dimension = |block: Option<std::rc::Rc<dyn TextBlock>>| {
            block.map_or_else(XDimension2D::default, |block| {
                block.calculate_dimension(string_bounder)
            })
        };
        let dim_link1 = dimension(
            self.diagram
                .to_text_block(SName::Arrow, self.conn(self.network)),
        );
        let dim_box = self.box_.calculate_dimension(string_bounder);
        let dim_link2 = dimension(self.link2());
        let width = (dim_link1.width + 2.0 * MARGIN_AD)
            .max(dim_box.width + 2.0 * MARGIN_BOX_W)
            .max(dim_link2.width + 2.0 * MARGIN_AD);
        let height = dim_link1.height
            + 2.0 * MARGIN_AD
            + 2.0 * self.top_margin
            + dim_box.height
            + dim_link2.height
            + 2.0 * MARGIN_AD;
        XDimension2D::new(width, height)
    }
}

/// A y that orders like Java's `TreeSet<Double>`.
#[derive(Clone, Copy, PartialEq)]
struct OrderedY(f64);

impl Eq for OrderedY {}

impl PartialOrd for OrderedY {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedY {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

fn draw_center(ug: &UGraphic, block: Option<&dyn TextBlock>, x: f64, y: f64) {
    let Some(block) = block else {
        return;
    };
    let dim = block.calculate_dimension(ug.string_bounder());
    block.draw_u(&ug.translated(x - dim.width / 2.0, y - dim.height / 2.0));
}

/// A link from `y1` to `y2`, hopping over the networks it crosses at `skip` (`VerticalLine`).
fn vertical_line(ug: &UGraphic, y1: f64, y2: f64, skip: &BTreeSet<OrderedY>) {
    let (y1, y2) = (y1.min(y2), y1.max(y2));
    let ug = ug.with_backcolor(HColor::NONE);
    let mut drawn = false;
    let mut segments = vec![USegment::MoveTo(0.0, y1)];
    for &OrderedY(step) in skip {
        if step < y1 {
            continue;
        }
        drawn = true;
        if step == y2 {
            segments.push(USegment::LineTo(0.0, y2));
        } else {
            segments.push(USegment::LineTo(0.0, y2.min(step - 3.0)));
            if y2 > step {
                segments.push(USegment::ArcTo {
                    radius: (4.0, 4.0),
                    x_axis_rotation: 0.0,
                    large_arc: false,
                    sweep: true,
                    end: (0.0, step + 9.0),
                });
                continue;
            }
        }
        ug.draw(&UShape::path(std::mem::take(&mut segments)));
        let current = step + 9.0;
        segments.push(USegment::MoveTo(0.0, current));
        if current >= y2 {
            break;
        }
    }
    if !drawn {
        segments.push(USegment::LineTo(0.0, y2));
        ug.draw(&UShape::path(segments));
    }
}

/// The servers in their cells: row `i` holds the servers whose main network is network `i`.
pub(super) struct Grid<'a> {
    diagram: &'a NwDiagram,
    groups: &'a [NwGroup],
    data: Vec<Vec<Option<NServerDraw<'a>>>>,
}

impl<'a> Grid<'a> {
    pub(super) fn new(diagram: &'a NwDiagram, lines: usize, cols: usize) -> Self {
        Self {
            diagram,
            groups: &diagram.groups,
            data: (0..lines)
                .map(|_| (0..cols).map(|_| None).collect())
                .collect(),
        }
    }

    pub(super) fn add(&mut self, i: usize, j: usize, value: NServerDraw<'a>) {
        self.data[i][j] = Some(value);
    }

    fn nb_cols(&self) -> usize {
        self.data.first().map_or(0, Vec::len)
    }

    fn col_width(&self, string_bounder: &dyn StringBounder, j: usize) -> f64 {
        self.data
            .iter()
            .filter_map(|line| line[j].as_ref())
            .map(|cell| cell.natural_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    pub(super) fn line_height(&self, string_bounder: &dyn StringBounder, i: usize) -> f64 {
        self.data[i]
            .iter()
            .flatten()
            .map(|cell| cell.natural_dimension(string_bounder).height)
            .fold(50.0, f64::max)
    }

    fn style_of(&self, name: SName) -> crate::style::Style {
        StyleSignature::of(&[SName::Root, SName::Element, SName::NwdiagDiagram, name])
            .get_merged_style(&self.diagram.titled.skin.current_style_builder())
    }

    /// The groups, the network tubes and the links (`drawGrid`).
    fn draw_grid(&self, ug: &UGraphic) {
        for group in self.groups {
            self.draw_group(ug, group);
        }
        self.draw_network_tube(ug);
        self.draw_links(ug);
    }

    fn draw_links(&self, ug: &UGraphic) {
        let line_color = self
            .style_of(SName::Arrow)
            .value(PName::LineColor)
            .as_color();
        let ug = ug.apply(line_color);
        let string_bounder = ug.string_bounder();
        for (i, line) in self.data.iter().enumerate() {
            let line_height = self.line_height(string_bounder, i);
            let mut x = 0.0;
            for (j, cell) in line.iter().enumerate() {
                let col_width = self.col_width(string_bounder, j);
                if let Some(cell) = cell {
                    cell.draw_links(&ug, x, col_width, line_height);
                }
                x += col_width;
            }
        }
    }

    fn draw_group(&self, ug: &UGraphic, group: &NwGroup) {
        let string_bounder = ug.string_bounder();
        let mut size: Option<MinMax> = None;
        let mut y = 0.0;
        for (i, line) in self.data.iter().enumerate() {
            let line_height = self.line_height(string_bounder, i);
            let mut x = 0.0;
            for (j, cell) in line.iter().enumerate() {
                let col_width = self.col_width(string_bounder, j);
                if let Some(cell) = cell
                    && group.contains(cell.server)
                {
                    let min_max = cell
                        .get_min_max(string_bounder, col_width, line_height)
                        .translate(UTranslate::new(x, y));
                    size = Some(size.map_or(min_max, |size| size.add_min_max(min_max)));
                }
                x += col_width;
            }
            y += line_height;
        }
        if let Some(size) = size {
            self.diagram.draw_group(ug, group, size);
        }
    }

    fn draw_network_tube(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let style = self.style_of(SName::Network);
        let mut y = 0.0;
        for (i, line) in self.data.iter().enumerate() {
            let network = &self.diagram.networks[i];
            self.compute_min_max(line, string_bounder, i);
            let width = MINIMUM_WIDTH.max(network.xmax.get() - network.xmin.get());
            let back_color = network
                .color
                .clone()
                .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
            network.y.set(y);
            if network.visible {
                ug.translated(network.xmin.get(), y)
                    .apply(style.value(PName::LineColor).as_color())
                    .with_backcolor(back_color)
                    .draw(&UShape::Rectangle(URectangle::new(width, NETWORK_THIN)));
            }
            y += self.line_height(string_bounder, i);
        }
    }

    fn compute_min_max(
        &self,
        line: &[Option<NServerDraw<'a>>],
        string_bounder: &dyn StringBounder,
        network_id: NetworkId,
    ) {
        let network = &self.diagram.networks[network_id];
        let mut x = 0.0;
        let mut xmin: f64 = if network.full_width { 0.0 } else { -1.0 };
        let mut xmax = 0.0;
        for j in 0..line.len() {
            let hline = self.is_there_a_link(j, network_id);
            if hline && xmin < 0.0 {
                xmin = x;
            }
            x += self.col_width(string_bounder, j);
            if hline || network.full_width {
                xmax = x;
            }
        }
        network.xmin.set(xmin);
        network.xmax.set(xmax);
    }

    fn is_there_a_link(&self, j: usize, network: NetworkId) -> bool {
        self.data.iter().any(|line| {
            line[j]
                .as_ref()
                .is_some_and(|cell| cell.is_linked_to(network))
        })
    }
}

impl TextBlock for Grid<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        if self.data.is_empty() {
            return XDimension2D::default();
        }
        let height: f64 = (0..self.data.len())
            .map(|i| self.line_height(string_bounder, i))
            .sum();
        let width: f64 = (0..self.nb_cols())
            .map(|j| self.col_width(string_bounder, j))
            .sum();
        XDimension2D::new(MINIMUM_WIDTH.max(width), height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.draw_grid(ug);
        let string_bounder = ug.string_bounder();
        let mut y = 0.0;
        for (i, line) in self.data.iter().enumerate() {
            let line_height = self.line_height(string_bounder, i);
            let mut x = 0.0;
            for (j, cell) in line.iter().enumerate() {
                let col_width = self.col_width(string_bounder, j);
                if let Some(cell) = cell {
                    cell.draw_me(&ug.translated(x, y), col_width, line_height);
                }
                x += col_width;
            }
            y += line_height;
        }
    }
}
