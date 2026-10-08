//! Network diagrams, `@startnwdiag` (PlantUML's `nwdiag` package).

mod commands;
mod grid;
mod model;

use std::collections::HashMap;
use std::rc::Rc;

use grid::{Grid, MAGIC, NETWORK_THIN, NServerDraw};
use model::{
    BarId, BoxId, NBar, NBox, NPlayField, NServer, NTetris, Network, NetworkId, NwGroup, Properties,
};

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::{Colors, HColor};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, ParserPass};
use crate::creole::{CreoleMode, Display};
use crate::jaws::BLOCK_E1_NEWLINE;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, MinMax, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// What a `{ ... }` block opened.
#[derive(Clone, Copy)]
enum Stackable {
    Network(NetworkId),
    Group(usize),
}

pub(super) struct NwDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    /// In the order they were named.
    servers: Vec<NServer>,
    networks: Vec<Network>,
    groups: Vec<NwGroup>,
    /// The open blocks, the innermost first.
    stack: Vec<Stackable>,
    play_field: NPlayField,
    bars: Vec<NBar>,
    boxes: Vec<NBox>,
}

/// Reads network diagrams (PlantUML's `NwDiagramFactory`).
pub(super) struct NwDiagramFactory;

impl CommandFactory for NwDiagramFactory {
    type Diagram = NwDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::NwDiag;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> NwDiagram {
        NwDiagram {
            source: source.clone(),
            titled: Titled::new(SName::NwdiagDiagram, "NWDIAG", source),
            servers: Vec::new(),
            networks: Vec::new(),
            groups: Vec::new(),
            stack: Vec::new(),
            play_field: NPlayField::default(),
            bars: Vec::new(),
            boxes: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<NwDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend(commands::all());
        commands
    }
}

impl NwDiagram {
    fn server_named(&self, name: &str) -> Option<usize> {
        self.servers.iter().position(|server| server.name == name)
    }

    fn current_network(&self) -> Option<NetworkId> {
        self.stack.iter().find_map(|element| match element {
            Stackable::Network(network) => Some(*network),
            Stackable::Group(_) => None,
        })
    }

    fn current_group(&self) -> Option<usize> {
        match self.stack.first() {
            Some(Stackable::Group(group)) => Some(*group),
            _ => None,
        }
    }

    fn open_group(&mut self) -> CommandResult {
        if self
            .stack
            .iter()
            .any(|element| matches!(element, Stackable::Group(_)))
        {
            return Err(CommandError::new("Cannot nest group"));
        }
        self.groups.push(NwGroup::new());
        self.stack
            .insert(0, Stackable::Group(self.groups.len() - 1));
        Ok(())
    }

    fn open_network(&mut self, name: Option<&str>) -> CommandResult {
        if self.current_group().is_some() {
            return Err(CommandError::new("Cannot open network in a group"));
        }
        if self
            .stack
            .iter()
            .any(|element| matches!(element, Stackable::Network(_)))
        {
            return Err(CommandError::new("Cannot nest network"));
        }
        if self.networks.is_empty() && self.groups.is_empty() {
            self.eventually_connect_all_standalone_servers_to_hidden_network();
        }
        let network = self.create_network(name);
        self.stack.insert(0, Stackable::Network(network));
        Ok(())
    }

    fn eventually_connect_all_standalone_servers_to_hidden_network(&mut self) {
        let mut first = None;
        for server in 0..self.servers.len() {
            if self.servers[server].is_alone() {
                let network = *first.get_or_insert_with(|| self.create_hidden_network());
                self.connect_me_if_alone(server, network);
            }
        }
    }

    fn close_something(&mut self) {
        if !self.stack.is_empty() {
            self.stack.remove(0);
        }
    }

    fn create_network(&mut self, name: Option<&str>) -> NetworkId {
        let up = self.play_field.get_last();
        let nstage = self.play_field.create_new_stage();
        self.networks.push(Network::new(up, nstage, name));
        self.networks.len() - 1
    }

    fn create_hidden_network(&mut self) -> NetworkId {
        let network = self.create_network(Some(""));
        self.networks[network].visible = false;
        network
    }

    fn new_server(&mut self, name: &str, bar: Option<BarId>) -> usize {
        let bar = bar.unwrap_or_else(|| {
            self.bars.push(NBar::default());
            self.bars.len() - 1
        });
        self.servers.push(NServer::new(name, bar));
        self.servers.len() - 1
    }

    /// `NServer.connectTo`.
    fn connect(&mut self, server: usize, network: NetworkId, address: Option<&str>) {
        let address = address.unwrap_or_default();
        let connected = &mut self.servers[server];
        if address.is_empty() && connected.get_address(network).is_some() {
            return;
        }
        match connected
            .connections
            .iter_mut()
            .find(|(existing, _)| *existing == network)
        {
            Some(entry) => address.clone_into(&mut entry.1),
            None => connected.connections.push((network, address.to_owned())),
        }
        let bar = &mut self.bars[connected.bar];
        if bar.start.is_none() {
            bar.add_stage(self.networks[network].nstage);
        } else if connected.get_main_network_next() != network
            && let Some(up) = self.networks[network].up
        {
            bar.add_stage(up);
        }
    }

    fn connect_me_if_alone(&mut self, server: usize, network: NetworkId) {
        if self.servers[server].is_alone() {
            self.connect(server, network, Some(""));
            if !self.networks[network].visible {
                self.servers[server].print_first_link = false;
            }
        }
    }

    fn add_in_play_field(&mut self, bar: BarId) {
        match self.bars[bar].parent {
            None => {
                let single = self.boxes.len();
                self.boxes.push(NBox::default());
                self.add_in_box(single, bar);
                self.bars[bar].parent = Some(single);
                self.play_field.boxes.push(single);
            }
            Some(parent) => {
                if !self.play_field.boxes.contains(&parent) {
                    self.play_field.boxes.push(parent);
                }
            }
        }
    }

    fn add_in_box(&mut self, nbox: BoxId, bar: BarId) {
        if self.boxes[nbox].bars.contains(&bar) {
            return;
        }
        let (start, end) = self.bar_stages(bar);
        let target = &mut self.boxes[nbox];
        target.bars.push(bar);
        target.tetris.add(bar, start, end, 1);
    }

    fn bar_stages(&self, bar: BarId) -> (usize, usize) {
        let bar = &self.bars[bar];
        (
            bar.start.expect("a placed server spans a stage"),
            bar.end.expect("a placed server spans a stage"),
        )
    }

    fn link(&mut self, name1: &str, name2: &str) -> CommandResult {
        let Some(server1) = self.server_named(name1) else {
            if self.networks.is_empty() {
                self.very_first_link(name1, name2);
                return Ok(());
            }
            return Err(CommandError::new(format!("what about {name1}")));
        };
        if self.servers[server1].is_alone() {
            if self.networks.is_empty() {
                self.create_hidden_network();
            }
            self.connect_me_if_alone(server1, 0);
        }
        let tmp1 = self.servers[server1].get_main_network_next();
        let network = match self.just_after(tmp1) {
            Some(just_after) if !self.networks[just_after].visible => just_after,
            _ => self.create_network(Some("")),
        };
        self.networks[network].visible = false;
        let server2 = match self.server_named(name2) {
            None => {
                let bar = self.servers[server1].bar;
                let server2 = self.new_server(name2, Some(bar));
                self.connect(server1, network, Some(""));
                self.connect(server2, network, Some(""));
                server2
            }
            Some(server2) => {
                self.servers[server1].blank_some_address();
                let address1 = self.servers[server1].some_address();
                self.connect(server1, network, Some(&address1));
                let address2 = self.servers[server2].some_address();
                self.connect(server2, network, Some(&address2));
                server2
            }
        };
        let bar = self.servers[server2].bar;
        self.add_in_play_field(bar);
        Ok(())
    }

    fn just_after(&self, network: NetworkId) -> Option<NetworkId> {
        (network + 1 < self.networks.len()).then_some(network + 1)
    }

    fn very_first_link(&mut self, name1: &str, name2: &str) {
        let network = self.create_network(Some(name1));
        let server2 = self.new_server(name2, None);
        self.connect(server2, network, Some(""));
        let bar = self.servers[server2].bar;
        self.add_in_play_field(bar);
    }

    fn add_element(&mut self, name: &str, definition: Option<&str>) -> CommandResult {
        let props = Properties::parse(definition);
        let server = match self.server_named(name) {
            Some(server) => server,
            None => self.new_server(name, None),
        };
        if let Some(group) = self.current_group() {
            if self.groups.iter().any(|group| group.names.contains(name)) {
                return Err(CommandError::new("Element already in another group."));
            }
            self.groups[group].names.insert(name.to_owned());
            if self.current_network().is_none() {
                self.servers[server].update_properties(&props);
                return Ok(());
            }
        }
        let Some(network) = self.current_network().filter(|_| !self.networks.is_empty()) else {
            self.servers[server].update_properties(&props);
            self.servers[server].learn_this_address(props.get("address"));
            return Ok(());
        };
        self.connect(server, network, props.get("address"));
        let bar = self.servers[server].bar;
        self.add_in_play_field(bar);
        self.servers[server].update_properties(&props);
        Ok(())
    }

    fn set_property(&mut self, property: &str, value: &str) {
        if property.eq_ignore_ascii_case("address")
            && let Some(network) = self.current_network()
        {
            self.networks[network].own_address = Some(value.to_owned());
        }
        if property.eq_ignore_ascii_case("width")
            && let Some(network) = self.current_network()
        {
            self.networks[network].full_width = value.eq_ignore_ascii_case("full");
        }
        if property.eq_ignore_ascii_case("color") {
            let color = HColor::parse_or_white(value);
            if let Some(group) = self.current_group() {
                self.groups[group].color = Some(color);
            } else if let Some(network) = self.current_network() {
                self.networks[network].color = Some(color);
            }
        }
        // PlantUML fails on a description outside any group or network.
        if property.eq_ignore_ascii_case("description") {
            if let Some(group) = self.current_group() {
                self.groups[group].description = Some(value.to_owned());
            } else if let Some(network) = self.current_network() {
                self.networks[network].description = Some(value.to_owned());
            }
        }
    }

    /// Puts the servers of each group side by side (`NPlayField.fixGroups`).
    fn fix_groups(&mut self) {
        for group in 0..self.groups.len() {
            for server in 0..self.servers.len() {
                if self.groups[group].contains(&self.servers[server]) {
                    self.fix_server_in_group(server, group);
                }
            }
        }
    }

    fn fix_server_in_group(&mut self, server: usize, group: usize) {
        let group_box = *self.groups[group].nbox.get_or_insert_with(|| {
            self.boxes.push(NBox::default());
            self.boxes.len() - 1
        });
        let bar = self.servers[server].bar;
        let parent = self.bars[bar].parent;
        if parent == Some(group_box) {
            return;
        }
        if let Some(position) = self
            .play_field
            .boxes
            .iter()
            .position(|&b| Some(b) == parent)
        {
            self.play_field.boxes.remove(position);
        }
        if !self.play_field.boxes.contains(&group_box) {
            self.play_field.boxes.push(group_box);
        }
        self.bars[bar].parent = Some(group_box);
        self.add_in_box(group_box, bar);
    }

    /// The column of each server's bar (`NPlayField.doLayout`).
    fn do_layout(&self) -> HashMap<BarId, usize> {
        let mut tetris = NTetris::default();
        for &nbox in &self.play_field.boxes {
            let bars = &self.boxes[nbox].bars;
            let start = bars
                .iter()
                .map(|&bar| self.bar_stages(bar).0)
                .min()
                .expect("a box holds bars");
            let end = bars
                .iter()
                .map(|&bar| self.bar_stages(bar).1)
                .max()
                .expect("a box holds bars");
            let width = self.boxes[nbox].tetris.get_n_width(|_| 1);
            tetris.add(nbox, start, end, width);
        }
        let mut result = HashMap::new();
        for &(nbox, box_position) in &tetris.all {
            for &(bar, bar_position) in &self.boxes[nbox].tetris.all {
                let column = usize::try_from(box_position + bar_position).expect("a column");
                result.insert(bar, column);
            }
        }
        result
    }

    fn style_of(&self, name: SName) -> Style {
        StyleSignature::of(&[SName::Root, SName::Element, SName::NwdiagDiagram, name])
            .get_merged_style(&self.titled.skin.current_style_builder())
    }

    /// A server's text with `, ` breaking lines; nothing for no text (`NServer.toTextBlock`).
    fn to_text_block(&self, name: SName, text: Option<&str>) -> Option<Rc<dyn TextBlock>> {
        let text = text?;
        if text.is_empty() {
            return Some(Rc::new(TextBlockEmpty::default()));
        }
        let text = text.replace(", ", &BLOCK_E1_NEWLINE.to_string());
        Some(Rc::new(Display::with_newlines(&text).create0(
            &self.style_of(name).font_configuration(),
            HorizontalAlignment::Left,
            &self.titled.skin,
            0.0,
            CreoleMode::Full,
        )))
    }

    fn server_draw<'a>(&'a self, server: &'a NServer, top_margin: f64) -> NServerDraw<'a> {
        let mut fashion = self
            .style_of(SName::Server)
            .symbol_context(&Colors::default());
        if let Some(back) = server
            .backcolor
            .as_deref()
            .and_then(|name| HColor::parse(name).ok().flatten())
        {
            fashion.back_color = back;
        }
        let desc = self
            .to_text_block(SName::Server, Some(&server.description))
            .unwrap_or_else(|| Rc::new(TextBlockEmpty::default()));
        let box_ = server.shape.as_small(
            Rc::new(TextBlockEmpty::default()),
            desc,
            Rc::new(TextBlockEmpty::default()),
            fashion,
            HorizontalAlignment::Center,
        );
        let conns = self
            .networks
            .iter()
            .enumerate()
            .filter_map(|(network, _)| {
                server
                    .get_address(network)
                    .map(|address| (network, address.to_owned()))
            })
            .collect();
        NServerDraw::new(self, server, box_, conns, top_margin)
    }

    fn build_grid(&self, string_bounder: &dyn StringBounder) -> Grid<'_> {
        let mut grid = Grid::new(self, self.networks.len(), self.servers.len());
        let layout = self.do_layout();
        for i in 0..self.networks.len() {
            for server in &self.servers {
                if server.get_main_network_next() != i {
                    continue;
                }
                let Some(&column) = layout.get(&server.bar) else {
                    continue;
                };
                let mut top_margin = MAGIC;
                if let Some(group) = self.groups.iter().find(|group| group.contains(server)) {
                    top_margin += self.group_top_header_height(group, string_bounder);
                }
                grid.add(i, column, self.server_draw(server, top_margin));
            }
        }
        grid
    }

    fn group_header(&self, group: &NwGroup) -> Option<Box<dyn TextBlock + '_>> {
        let description = group.description.as_deref()?;
        Some(Box::new(Display::with_newlines(description).create0(
            &self.style_of(SName::Group).font_configuration(),
            HorizontalAlignment::Left,
            &self.titled.skin,
            0.0,
            CreoleMode::Full,
        )))
    }

    fn group_top_header_height(&self, group: &NwGroup, string_bounder: &dyn StringBounder) -> f64 {
        self.group_header(group).map_or(0.0, |block| {
            block.calculate_dimension(string_bounder).height
        })
    }

    /// A group's box around its servers, its description above them (`NwGroup.drawGroup`).
    fn draw_group(&self, ug: &UGraphic, group: &NwGroup, mut size: MinMax) {
        let style = self.style_of(SName::Group);
        let block = self.group_header(group);
        if let Some(block) = &block {
            let block_dim = block.calculate_dimension(ug.string_bounder());
            size = size.add_point(size.min_x(), size.min_y() - block_dim.height);
        }
        let background_color = group
            .color
            .clone()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        let dimension = size.dimension();
        ug.apply(style.value(PName::LineColor).as_color())
            .with_backcolor(background_color)
            .translated(size.min_x(), size.min_y())
            .draw(&UShape::Rectangle(URectangle::new(
                dimension.width,
                dimension.height,
            )));
        if let Some(block) = block {
            block.draw_u(&ug.translated(size.min_x() + 5.0, size.min_y()));
        }
    }

    /// The network's name and address; nothing for an unnamed network without an address. PlantUML would
    /// write `null` above the address of an unnamed one.
    fn network_name(&self, network: &Network) -> Box<dyn TextBlock + '_> {
        let name = match (network.display_name(), &network.own_address) {
            (Some(name), Some(address)) => format!("{name}{BLOCK_E1_NEWLINE}{address}"),
            (Some(name), None) => name.to_owned(),
            (None, Some(address)) => address.clone(),
            (None, None) => return Box::new(TextBlockEmpty::default()),
        };
        Box::new(Display::with_newlines(&name).create0(
            &self.style_of(SName::Network).font_configuration(),
            HorizontalAlignment::Right,
            &SpriteContainerEmpty,
            0.0,
            CreoleMode::Full,
        ))
    }

    /// The network names on the left, then the grid (`drawMe`).
    fn draw_me(&self, ug: &UGraphic) {
        const MARGIN: f64 = 5.0;
        let ug = ug.translated(MARGIN, MARGIN);
        let string_bounder = ug.string_bounder();
        let grid = self.build_grid(string_bounder);
        let mut delta_x: f64 = 0.0;
        let mut delta_y = 0.0;
        let names: Vec<_> = self
            .networks
            .iter()
            .map(|network| self.network_name(network))
            .collect();
        for (i, desc) in names.iter().enumerate() {
            let dim = desc.calculate_dimension(string_bounder);
            if i == 0 {
                delta_y = (dim.height - NETWORK_THIN) / 2.0;
            }
            delta_x = delta_x.max(dim.width);
        }
        let mut y = 0.0;
        for (i, desc) in names.iter().enumerate() {
            let dim = desc.calculate_dimension(string_bounder);
            desc.draw_u(&ug.translated(delta_x - dim.width, y));
            y += grid.line_height(string_bounder, i);
        }
        delta_x += 5.0;
        grid.draw_u(&ug.translated(delta_x, delta_y));
        let dim_grid = grid.calculate_dimension(string_bounder);
        ug.translated(
            dim_grid.width + delta_x + MARGIN,
            dim_grid.height + delta_y + MARGIN,
        )
        .draw(&UShape::Empty(XDimension2D::new(1.0, 1.0)));
    }
}

/// The whole drawing, measured by drawing it.
struct Drawing<'a>(&'a NwDiagram);

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        LimitFinder::min_max_from_origin_of(self, string_bounder.shared()).dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.0.draw_me(ug);
    }
}

impl AbstractDiagram for NwDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}

    /// Servers outside any network join the first, and every server gets a column.
    fn make_diagram_ready(&mut self) {
        if self.networks.is_empty() {
            self.create_hidden_network();
        }
        for server in 0..self.servers.len() {
            self.connect_me_if_alone(server, 0);
            let bar = self.servers[server].bar;
            self.add_in_play_field(bar);
        }
        self.fix_groups();
    }
}

impl TitledDiagram for NwDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for NwDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Ok(self
            .titled
            .add_chrome(Box::new(Drawing(self)), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::none())
    }
}
