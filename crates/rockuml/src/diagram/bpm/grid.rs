//! The grid the elements are placed on, the connectors routed between them and the cleaners that merge its
//! lines (PlantUML's `Grid`, `ChainImpl`, `Navigators`, `Cell`, `Coord`, `ConnectorPuzzleEmpty`,
//! `CleanerEmptyLine` and `CleanerInterleavingLines`).
//!
//! PlantUML keeps the cells in a `HashMap` keyed by the identity of its lines and columns. Its order shows
//! nowhere: every walk over it either sets flags or finds the one cell holding something, except
//! `getById` when two merges share a name, where PlantUML picks either. rockuml keeps the order the cells
//! were made in and picks the first.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::element::BpmElement;
use crate::diagram::NotYetPorted;

/// Where PlantUML throws while it lays the grid out, and draws a crash report instead.
pub(super) const CRASH: NotYetPorted =
    NotYetPorted("the crash report of a BPM diagram PlantUML cannot lay out");

/// A side of a cell a connector leaves by (`ConnectorPuzzle.Where`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Where {
    North,
    East,
    South,
    West,
}

impl Where {
    pub(super) const VALUES: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    fn coding(self) -> u8 {
        match self {
            Self::North => 1,
            Self::East => 2,
            Self::South => 4,
            Self::West => 8,
        }
    }

    fn to_short_string(self) -> char {
        match self {
            Self::North => 'N',
            Self::East => 'E',
            Self::South => 'S',
            Self::West => 'W',
        }
    }
}

/// The sides a cell is connected by (`AbstractConnectorPuzzle`'s `EnumSet`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Connections(u8);

impl Connections {
    /// The sides whose initials `directions` holds (`ConnectorPuzzleEmpty.get`).
    fn get(directions: &str) -> Self {
        let mut result = Self::default();
        for w in Where::VALUES {
            if directions.contains(w.to_short_string()) {
                result.append(w);
            }
        }
        result
    }

    pub(super) fn have(self, w: Where) -> bool {
        self.0 & w.coding() != 0
    }

    pub(super) fn append(&mut self, w: Where) {
        self.0 |= w.coding();
    }

    fn append_all(&mut self, other: Self) {
        self.0 |= other.0;
    }

    fn remove(&mut self, w: Where) -> Result<(), NotYetPorted> {
        if !self.have(w) {
            return Err(CRASH);
        }
        self.0 &= !w.coding();
        Ok(())
    }

    /// `checkDirections`: connected by exactly these sides.
    fn check_directions(self, directions: &str) -> bool {
        self == Self::get(directions)
    }
}

/// What a cell holds: an element or a piece of connector (`Placeable`, `ConnectorPuzzleEmpty`).
pub(super) enum Placeable {
    Element(BpmElement),
    Puzzle(Connections),
}

impl Placeable {
    fn connections_mut(&mut self) -> &mut Connections {
        match self {
            Self::Element(element) => &mut element.connections,
            Self::Puzzle(connections) => connections,
        }
    }
}

/// Which placeable of the grid, by identity.
type PlaceableId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Line(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Col(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Coord {
    pub(super) line: Line,
    pub(super) col: Col,
}

impl Coord {
    pub(super) fn new(line: Line, col: Col) -> Self {
        Self { line, col }
    }
}

/// The lines or columns in their order (`ChainImpl`). PlantUML's chain can also grow before its root, which
/// BPM diagrams never ask for.
struct Chain<O> {
    list: Vec<O>,
}

impl<O: Copy + PartialEq> Chain<O> {
    fn new(root: O) -> Self {
        Self { list: vec![root] }
    }

    fn position(&self, data: O) -> usize {
        self.list
            .iter()
            .position(|&cur| cur == data)
            .expect("the grid's lines and columns are in its chains")
    }

    fn insert_after(&mut self, at: O, data: O) {
        let position = self.position(at);
        self.list.insert(position + 1, data);
    }

    fn compare(&self, a: O, b: O) -> Ordering {
        self.position(a).cmp(&self.position(b))
    }

    fn remove(&mut self, data: O) {
        let position = self.position(data);
        self.list.remove(position);
    }

    /// What a walk from `from` to `to` passes, both left out (`Navigators.iterate`).
    fn between(&self, from: O, to: O) -> Vec<O> {
        let (from, to) = (self.position(from), self.position(to));
        match from.cmp(&to) {
            Ordering::Equal => Vec::new(),
            Ordering::Less => self.list[from + 1..to].to_vec(),
            Ordering::Greater => self.list[to + 1..from].iter().rev().copied().collect(),
        }
    }
}

/// A place of the grid and the elements its element connects to, the first one leaving sideways (`Cell`).
#[derive(Default)]
struct Cell {
    data: Option<PlaceableId>,
    destinations: Vec<PlaceableId>,
}

pub(super) struct Grid {
    lines: Chain<Line>,
    cols: Chain<Col>,
    root: Coord,
    cells: HashMap<Coord, Cell>,
    /// The places of the cells in the order they were made.
    made: Vec<Coord>,
    placeables: Vec<Option<Placeable>>,
    /// Numbers the lines and columns made, which PlantUML tells apart by identity.
    created: usize,
}

impl Grid {
    pub(super) fn new() -> Self {
        let root = Coord::new(Line(0), Col(1));
        Self {
            lines: Chain::new(root.line),
            cols: Chain::new(root.col),
            root,
            cells: HashMap::from([(root, Cell::default())]),
            made: vec![root],
            placeables: Vec::new(),
            created: 2,
        }
    }

    pub(super) fn get_root(&self) -> Coord {
        self.root
    }

    /// A new line after `line`'s (`linesOf(coord).insertAfter`).
    pub(super) fn insert_line_after(&mut self, line: Line) -> Line {
        let new_line = Line(self.created);
        self.created += 1;
        self.lines.insert_after(line, new_line);
        new_line
    }

    /// A new column after `col` (`colsOf(coord).insertAfter`).
    pub(super) fn insert_col_after(&mut self, col: Col) -> Col {
        let new_col = Col(self.created);
        self.created += 1;
        self.cols.insert_after(col, new_col);
        new_col
    }

    fn get_cell(&mut self, coord: Coord) -> &mut Cell {
        self.cells.entry(coord).or_insert_with(|| {
            self.made.push(coord);
            Cell::default()
        })
    }

    fn data(&self, coord: Coord) -> Option<PlaceableId> {
        self.cells.get(&coord).and_then(|cell| cell.data)
    }

    /// The cells in the order they were made.
    fn cells_in_order(&self) -> impl Iterator<Item = (Coord, &Cell)> {
        self.made.iter().map(|coord| (*coord, &self.cells[coord]))
    }

    fn placeable_mut(&mut self, id: PlaceableId) -> &mut Placeable {
        self.placeables[id]
            .as_mut()
            .expect("placeables stay in the grid until it becomes an array")
    }

    fn element_mut(&mut self, coord: Coord) -> &mut BpmElement {
        let id = self.data(coord).expect("connections join elements");
        match self.placeable_mut(id) {
            Placeable::Element(element) => element,
            Placeable::Puzzle(_) => unreachable!("connections join elements"),
        }
    }

    /// Places `element` at `coord`.
    pub(super) fn set_element(&mut self, coord: Coord, element: BpmElement) {
        self.placeables.push(Some(Placeable::Element(element)));
        let id = self.placeables.len() - 1;
        self.get_cell(coord).data = Some(id);
    }

    /// Connects the element at `src` to the one at `dest` (`addConnectionTo2`).
    pub(super) fn add_connection(&mut self, src: Coord, dest: Coord) {
        let dest = self.data(dest).expect("connections join elements");
        self.get_cell(src).destinations.push(dest);
    }

    /// Where the element named `id` is.
    pub(super) fn get_by_id(&self, id: &str) -> Option<Coord> {
        self.cells_in_order().find_map(|(coord, cell)| {
            let placeable = self.placeables[cell.data?].as_ref()?;
            match placeable {
                Placeable::Element(element) if element.id.as_deref() == Some(id) => Some(coord),
                _ => None,
            }
        })
    }

    fn get_coord(&self, placeable: PlaceableId) -> Coord {
        self.cells
            .iter()
            .find(|(_, cell)| cell.data == Some(placeable))
            .map(|(coord, _)| *coord)
            .expect("every placeable is in a cell")
    }

    /// Routes every connection: the first one from an element leaves sideways, the others up or down.
    pub(super) fn add_connections(&mut self) -> Result<(), NotYetPorted> {
        let connections: Vec<(Coord, Vec<PlaceableId>)> = self
            .cells_in_order()
            .map(|(coord, cell)| (coord, cell.destinations.clone()))
            .collect();
        for (src, destinations) in connections {
            for (i, &destination) in destinations.iter().enumerate() {
                let dest = self.get_coord(destination);
                if i == 0 {
                    self.draw_start_horizontal(src, dest)?;
                } else {
                    self.draw_start_vertical(src, dest)?;
                }
            }
        }
        Ok(())
    }

    fn draw_start_vertical(&mut self, src: Coord, dest: Coord) -> Result<(), NotYetPorted> {
        let compare = self.lines.compare(src.line, dest.line);
        if src == dest || compare == Ordering::Equal {
            return Err(CRASH);
        }
        let start = if compare == Ordering::Less {
            Where::South
        } else {
            Where::North
        };
        self.element_mut(src).connections.append(start);
        for cur in self.lines.between(src.line, dest.line) {
            self.add_puzzle(cur, src.col, "NS")?;
        }
        for cur in self.cols.between(src.col, dest.col) {
            self.add_puzzle(dest.line, cur, "EW")?;
        }
        // The lines differ, so the end is only joined from the side, and only from another column.
        if src.col != dest.col {
            let vertical = if self.lines.compare(dest.line, src.line) == Ordering::Greater {
                "N"
            } else {
                "S"
            };
            self.add_puzzle(dest.line, src.col, vertical)?;
            let horizontal = if self.cols.compare(dest.col, src.col) == Ordering::Greater {
                "E"
            } else {
                "W"
            };
            self.add_puzzle(dest.line, src.col, horizontal)?;
            let end = if self.cols.compare(src.col, dest.col) == Ordering::Greater {
                Where::East
            } else {
                Where::West
            };
            self.element_mut(dest).connections.append(end);
        }
        Ok(())
    }

    fn draw_start_horizontal(&mut self, src: Coord, dest: Coord) -> Result<(), NotYetPorted> {
        let compare = self.cols.compare(src.col, dest.col);
        if src == dest || compare == Ordering::Equal {
            return Err(CRASH);
        }
        let start = if compare == Ordering::Less {
            Where::East
        } else {
            Where::West
        };
        self.element_mut(src).connections.append(start);
        for cur in self.cols.between(src.col, dest.col) {
            self.add_puzzle(src.line, cur, "EW")?;
        }
        for cur in self.lines.between(src.line, dest.line) {
            self.add_puzzle(cur, dest.col, "NS")?;
        }
        if src.line == dest.line {
            let end = if compare == Ordering::Less {
                Where::West
            } else {
                Where::East
            };
            self.element_mut(dest).connections.append(end);
        } else {
            let horizontal = if self.cols.compare(dest.col, src.col) == Ordering::Greater {
                "W"
            } else {
                "E"
            };
            self.add_puzzle(src.line, dest.col, horizontal)?;
            let vertical = if self.lines.compare(dest.line, src.line) == Ordering::Greater {
                "S"
            } else {
                "N"
            };
            self.add_puzzle(src.line, dest.col, vertical)?;
            let end = if self.lines.compare(src.line, dest.line) == Ordering::Greater {
                Where::South
            } else {
                Where::North
            };
            self.element_mut(dest).connections.append(end);
        }
        Ok(())
    }

    /// Adds sides to the connector at a place, where PlantUML fails on an element.
    fn add_puzzle(&mut self, line: Line, col: Col, direction: &str) -> Result<(), NotYetPorted> {
        let coord = Coord::new(line, col);
        let id = if let Some(id) = self.data(coord) {
            id
        } else {
            self.placeables
                .push(Some(Placeable::Puzzle(Connections::default())));
            let id = self.placeables.len() - 1;
            self.get_cell(coord).data = Some(id);
            id
        };
        match self.placeable_mut(id) {
            Placeable::Puzzle(connections) => {
                connections.append_all(Connections::get(direction));
                Ok(())
            }
            Placeable::Element(_) => Err(CRASH),
        }
    }

    fn is_line_empty(&self, line: Line) -> bool {
        !self
            .cells
            .iter()
            .any(|(coord, cell)| coord.line == line && cell.data.is_some())
    }

    fn remove_line(&mut self, line: Line) {
        self.cells.retain(|coord, _| coord.line != line);
        self.made.retain(|coord| coord.line != line);
        self.lines.remove(line);
    }

    /// Merges and removes lines until no cleaner finds more to do (`BpmDiagram.cleanGrid`). PlantUML's third
    /// cleaner, `CleanerMoveBlock`, finds blocks to move but moves none.
    pub(super) fn clean(&mut self) -> Result<(), NotYetPorted> {
        loop {
            let v1 = self.clean_empty_lines();
            let v2 = self.clean_interleaving_lines()?;
            if !v1 && !v2 {
                return Ok(());
            }
        }
    }

    /// `CleanerEmptyLine`.
    fn clean_empty_lines(&mut self) -> bool {
        let mut result = false;
        for line in self.lines.list.clone() {
            if self.is_line_empty(line) {
                self.remove_line(line);
                result = true;
            }
        }
        result
    }

    /// `CleanerInterleavingLines`: merges the first two neighbouring lines whose cells fit into one another.
    fn clean_interleaving_lines(&mut self) -> Result<bool, NotYetPorted> {
        for pair in self.lines.list.clone().windows(2) {
            if self.mergeable_lines(pair[0], pair[1]) {
                self.merge_lines(pair[0], pair[1])?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn merge_lines(&mut self, line1: Line, line2: Line) -> Result<(), NotYetPorted> {
        for col in self.cols.list.clone() {
            let data1 = self.data(Coord::new(line1, col));
            let data2 = self.data(Coord::new(line2, col));
            let merged = self.merge(data1, data2)?;
            self.get_cell(Coord::new(line1, col)).data = merged;
            self.get_cell(Coord::new(line2, col)).data = None;
        }
        self.remove_line(line2);
        Ok(())
    }

    fn mergeable_lines(&self, line1: Line, line2: Line) -> bool {
        self.cols.list.iter().all(|&col| {
            self.mergeable(
                self.data(Coord::new(line1, col)),
                self.data(Coord::new(line2, col)),
            )
        })
    }

    fn placeable(&self, id: PlaceableId) -> &Placeable {
        self.placeables[id]
            .as_ref()
            .expect("placeables stay in the grid until it becomes an array")
    }

    fn merge(
        &mut self,
        data1: Option<PlaceableId>,
        data2: Option<PlaceableId>,
    ) -> Result<Option<PlaceableId>, NotYetPorted> {
        let (Some(id1), Some(id2)) = (data1, data2) else {
            return Ok(data1.or(data2));
        };
        match (self.placeable(id1), self.placeable(id2)) {
            (Placeable::Element(_), _) => Ok(data1),
            (Placeable::Puzzle(puz1), Placeable::Element(_)) => {
                if puz1.check_directions("SW") {
                    let connections = self.placeable_mut(id2).connections_mut();
                    connections.remove(Where::North)?;
                    connections.append(Where::West);
                }
                Ok(data2)
            }
            (Placeable::Puzzle(_), Placeable::Puzzle(_)) => Ok(data2),
        }
    }

    fn mergeable(&self, data1: Option<PlaceableId>, data2: Option<PlaceableId>) -> bool {
        let (Some(id1), Some(id2)) = (data1, data2) else {
            return true;
        };
        match (self.placeable(id1), self.placeable(id2)) {
            (Placeable::Puzzle(puz1), Placeable::Puzzle(puz2)) => {
                puz1.check_directions("NS")
                    && (puz2.check_directions("NS")
                        || puz2.check_directions("NE")
                        || puz2.check_directions("NW"))
            }
            (Placeable::Puzzle(puz1), Placeable::Element(_)) => {
                puz1.check_directions("NS") || puz1.check_directions("SW")
            }
            (Placeable::Element(_), Placeable::Puzzle(puz2)) => puz2.check_directions("NS"),
            (Placeable::Element(_), Placeable::Element(_)) => false,
        }
    }

    /// The placeables by line and column (`toArray`).
    pub(super) fn into_array(mut self) -> Vec<Vec<Option<Placeable>>> {
        let mut result: Vec<Vec<Option<Placeable>>> = self
            .lines
            .list
            .iter()
            .map(|_| self.cols.list.iter().map(|_| None).collect())
            .collect();
        for (coord, cell) in &self.cells {
            if let Some(id) = cell.data {
                let l = self.lines.position(coord.line);
                let c = self.cols.position(coord.col);
                result[l][c] = self.placeables[id].take();
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::bpm::element::BpmElementType;

    fn element(kind: BpmElementType) -> BpmElement {
        BpmElement::new(None, kind, None)
    }

    #[test]
    fn a_walk_passes_what_lies_between_in_its_direction() {
        let mut chain = Chain::new(0);
        chain.insert_after(0, 1);
        chain.insert_after(1, 3);
        chain.insert_after(1, 2);
        assert_eq!(chain.between(0, 3), [1, 2]);
        assert_eq!(chain.between(3, 0), [2, 1]);
        assert_eq!(chain.between(1, 2), []);
        assert_eq!(chain.compare(3, 1), Ordering::Greater);
    }

    #[test]
    fn directions_compare_as_sets() {
        let mut connections = Connections::get("SN");
        assert!(connections.check_directions("NS"));
        connections.append(Where::West);
        assert!(!connections.check_directions("NS"));
        assert!(connections.remove(Where::East).is_err());
    }

    #[test]
    fn a_connector_through_an_element_crashes_as_in_plantuml() {
        let mut grid = Grid::new();
        let root = grid.get_root();
        grid.set_element(root, element(BpmElementType::Start));
        let middle = Coord::new(root.line, grid.insert_col_after(root.col));
        grid.set_element(middle, element(BpmElementType::Merge));
        let end = Coord::new(root.line, grid.insert_col_after(middle.col));
        grid.set_element(end, element(BpmElementType::Merge));
        grid.add_connection(end, root);
        assert_eq!(grid.add_connections(), Err(CRASH));
    }
}
