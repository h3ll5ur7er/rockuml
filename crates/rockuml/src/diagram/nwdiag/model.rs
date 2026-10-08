//! What a network diagram describes, and the columns its servers get (PlantUML's `nwdiag.core` package and
//! `NBar`, `NBox`, `NPlayField`, `NTetris`, `BooleanGrid`).

use std::cell::Cell;
use std::collections::HashSet;

use crate::color::HColor;
use crate::decoration::symbol::{PackageStyle, USymbol, USymbols};
use crate::skin::actor::ActorStyle;
use crate::skin::component_style::ComponentStyle;

pub(super) type NetworkId = usize;
pub(super) type BarId = usize;
pub(super) type BoxId = usize;
/// A stage is a row of the play field: networks take one each, in order.
pub(super) type Stage = usize;

pub(super) struct Network {
    /// `None` for `network {`, which then has no name to show.
    pub(super) name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) color: Option<HColor>,
    pub(super) visible: bool,
    pub(super) own_address: Option<String>,
    pub(super) full_width: bool,
    /// The stage before the network's own, when there is one.
    pub(super) up: Option<Stage>,
    pub(super) nstage: Stage,
    /// Where the network was last drawn, which the links to it are drawn against.
    pub(super) y: Cell<f64>,
    pub(super) xmin: Cell<f64>,
    pub(super) xmax: Cell<f64>,
}

impl Network {
    pub(super) fn new(up: Option<Stage>, nstage: Stage, name: Option<&str>) -> Self {
        Self {
            name: name.map(str::to_owned),
            description: None,
            color: None,
            visible: true,
            own_address: None,
            full_width: false,
            up,
            nstage,
            y: Cell::new(0.0),
            xmin: Cell::new(0.0),
            xmax: Cell::new(0.0),
        }
    }

    pub(super) fn display_name(&self) -> Option<&str> {
        self.description.as_deref().or(self.name.as_deref())
    }

    /// Links to the network are shifted alternately left and right of their column's middle.
    pub(super) fn magic_delta(&self) -> f64 {
        if !self.visible {
            0.0
        } else if self.nstage.is_multiple_of(2) {
            2.0
        } else {
            -2.0
        }
    }
}

pub(super) struct NServer {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) backcolor: Option<String>,
    pub(super) shape: USymbol,
    /// In the order the server joined the networks; the first is its main network.
    pub(super) connections: Vec<(NetworkId, String)>,
    pub(super) bar: BarId,
    pub(super) declared_address: Option<String>,
    pub(super) print_first_link: bool,
}

impl NServer {
    pub(super) fn new(name: &str, bar: BarId) -> Self {
        Self {
            name: name.to_owned(),
            description: name.to_owned(),
            backcolor: None,
            shape: USymbols::RECTANGLE,
            connections: Vec::new(),
            bar,
            declared_address: None,
            print_first_link: true,
        }
    }

    pub(super) fn is_alone(&self) -> bool {
        self.connections.is_empty()
    }

    pub(super) fn some_address(&self) -> String {
        match self.connections.first() {
            Some((_, address)) if !address.is_empty() => address.clone(),
            _ => self.declared_address.clone().unwrap_or_default(),
        }
    }

    pub(super) fn blank_some_address(&mut self) {
        if let Some((_, address)) = self.connections.first_mut() {
            address.clear();
        }
    }

    /// The first connection without an address gets this one.
    pub(super) fn learn_this_address(&mut self, address: Option<&str>) {
        if let Some((_, existing)) = self
            .connections
            .iter_mut()
            .find(|(_, existing)| existing.is_empty())
        {
            address.unwrap_or_default().clone_into(existing);
        }
    }

    pub(super) fn get_main_network_next(&self) -> NetworkId {
        self.connections
            .first()
            .expect("a drawn server is connected")
            .0
    }

    pub(super) fn get_address(&self, network: NetworkId) -> Option<&str> {
        self.connections
            .iter()
            .find(|(connected, _)| *connected == network)
            .map(|(_, address)| address.as_str())
    }

    pub(super) fn update_properties(&mut self, props: &Properties) {
        if let Some(description) = props.get("description") {
            description.clone_into(&mut self.description);
        }
        if let Some(color) = props.get("color") {
            self.backcolor = Some(color.to_owned());
        }
        if let Some(address) = props.get("address") {
            self.declared_address = Some(address.to_owned());
        }
        if let Some(shape) = props.get("shape")
            && let Some(symbol) = USymbols::from_string(
                shape,
                ActorStyle::Stickman,
                ComponentStyle::Rectangle,
                PackageStyle::Rectangle,
            )
        {
            self.shape = symbol;
        }
    }
}

/// `[name = value, ...]` after an element (`NwDiagram.toSet`).
pub(super) struct Properties(Vec<(String, String)>);

impl Properties {
    pub(super) fn parse(definition: Option<&str>) -> Self {
        static PATTERN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r#"\s*([a-zA-Z0-9_]+)\s*=\s*("([^"]*)"|[^\s,]+)"#).unwrap()
        });
        let mut result: Vec<(String, String)> = Vec::new();
        for captures in PATTERN.captures_iter(definition.unwrap_or_default()) {
            let value = captures.get(3).unwrap_or_else(|| captures.get(2).unwrap());
            let name = captures[1].to_owned();
            match result.iter_mut().find(|(existing, _)| *existing == name) {
                Some(entry) => value.as_str().clone_into(&mut entry.1),
                None => result.push((name, value.as_str().to_owned())),
            }
        }
        Self(result)
    }

    pub(super) fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, value)| value.as_str())
    }
}

pub(super) struct NwGroup {
    pub(super) names: HashSet<String>,
    pub(super) color: Option<HColor>,
    pub(super) description: Option<String>,
    pub(super) nbox: Option<BoxId>,
}

impl NwGroup {
    pub(super) fn new() -> Self {
        Self {
            names: HashSet::new(),
            color: None,
            description: None,
            nbox: None,
        }
    }

    pub(super) fn contains(&self, server: &NServer) -> bool {
        self.names.contains(&server.name)
    }
}

/// The stages one server spans, between its networks.
#[derive(Default)]
pub(super) struct NBar {
    pub(super) parent: Option<BoxId>,
    pub(super) start: Option<Stage>,
    pub(super) end: Option<Stage>,
}

impl NBar {
    pub(super) fn add_stage(&mut self, stage: Stage) {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            self.start = Some(start.min(stage));
            self.end = Some(end.max(stage));
        } else {
            self.start = Some(stage);
            self.end = Some(stage);
        }
    }
}

/// Bars kept side by side, as a group's servers are.
#[derive(Default)]
pub(super) struct NBox {
    pub(super) bars: Vec<BarId>,
    pub(super) tetris: NTetris,
}

/// Places elements spanning stages in the leftmost columns where they fit (`NTetris`).
#[derive(Default)]
pub(super) struct NTetris {
    /// Each element and its column, in the order added.
    pub(super) all: Vec<(usize, i32)>,
    grid: BooleanGrid,
}

impl NTetris {
    /// Adds the element spanning `start..=end` and `width` columns.
    pub(super) fn add(&mut self, element: usize, start: Stage, end: Stage, width: i32) {
        let (start, end) = (to_i32(start), to_i32(end));
        for x in 0..=100 {
            if !self.grid.is_burn_rect(x, start, x + width - 1, end) {
                self.all.push((element, x));
                self.grid.burn_rect(x, start, x + width - 1, end);
                return;
            }
        }
        panic!("no column left for a network element");
    }

    pub(super) fn get_n_width(&self, width_of: impl Fn(usize) -> i32) -> i32 {
        self.all
            .iter()
            .map(|&(element, x)| x + width_of(element))
            .max()
            .unwrap_or(0)
            .max(0)
    }
}

fn to_i32(stage: Stage) -> i32 {
    i32::try_from(stage).expect("a few stages")
}

/// The cells taken (`BooleanGrid`).
#[derive(Default)]
struct BooleanGrid {
    burned: HashSet<(i32, i32)>,
}

impl BooleanGrid {
    fn burn_rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        check(x1, y1, x2, y2);
        for x in x1..=x2 {
            for y in y1..=y2 {
                assert!(self.burned.insert((x, y)), "a cell is taken once");
            }
        }
    }

    fn is_burn_rect(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
        check(x1, y1, x2, y2);
        (x1..=x2).any(|x| (y1..=y2).any(|y| self.burned.contains(&(x, y))))
    }
}

fn check(x1: i32, y1: i32, x2: i32, y2: i32) {
    assert!(
        x1 >= 0 && y1 >= 0 && x2 >= x1 && y2 >= y1,
        "a rectangle of the grid"
    );
}

/// The stages and the boxes laid out in columns (`NPlayField`).
#[derive(Default)]
pub(super) struct NPlayField {
    pub(super) stages: usize,
    pub(super) boxes: Vec<BoxId>,
}

impl NPlayField {
    pub(super) fn get_last(&self) -> Option<Stage> {
        self.stages.checked_sub(1)
    }

    pub(super) fn create_new_stage(&mut self) -> Stage {
        self.stages += 1;
        self.stages - 1
    }
}
