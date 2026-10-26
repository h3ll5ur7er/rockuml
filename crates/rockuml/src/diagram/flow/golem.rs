//! The grid flow diagrams lay their boxes out on (PlantUML's `golem` package): every tile takes two by two
//! cells, a new tile goes two cells away from the last one, and the tiles in its way move east.

use crate::java::RuntimeException;

/// A side of a tile (PlantUML's `TileGeometry`, without the `CENTER` nothing uses).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TileGeometry {
    North,
    South,
    East,
    West,
}

impl TileGeometry {
    pub(super) fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }

    /// The side a command's `n`, `s`, `e` or `w` names, in either case (`fromString`).
    pub(super) fn from_letter(letter: &str) -> Self {
        match letter.to_ascii_uppercase().as_str() {
            "N" => Self::North,
            "S" => Self::South,
            "E" => Self::East,
            "W" => Self::West,
            _ => unreachable!("the commands accept n, s, e and w only"),
        }
    }
}

/// The cells a tile covers, both bounds included (`Position`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Position {
    pub xmin: i32,
    pub ymin: i32,
    pub xmax: i32,
    pub ymax: i32,
}

impl Position {
    fn moved(self, geometry: TileGeometry, size_move: i32) -> Self {
        let (dx, dy) = match geometry {
            TileGeometry::North => (0, -size_move),
            TileGeometry::South => (0, size_move),
            TileGeometry::West => (-size_move, 0),
            TileGeometry::East => (size_move, 0),
        };
        Self {
            xmin: self.xmin + dx,
            ymin: self.ymin + dy,
            xmax: self.xmax + dx,
            ymax: self.ymax + dy,
        }
    }

    pub(super) fn center_x(self) -> i32 {
        (self.xmin + self.xmax + 1) / 2
    }

    pub(super) fn center_y(self) -> i32 {
        (self.ymin + self.ymax + 1) / 2
    }
}

/// A tile, by the number PlantUML gives it in order of creation (`Tile`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Tile(pub usize);

/// A side of a tile (`TileArea`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TileArea {
    pub tile: Tile,
    pub geometry: TileGeometry,
}

/// A link from a side of a tile to a side of another (`Path`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Path {
    pub start: TileArea,
    pub dest: TileArea,
}

/// The tiles and the links between them (`TilesField`).
///
/// PlantUML keeps the positions in a `HashMap` keyed by tiles, which hash by identity, so their order changes
/// from one Java machine to another, and with it the place of tiles moved onto one another to make room; the
/// positions here come in the order of the tiles.
pub(super) struct TilesField {
    positions: Vec<Position>,
    paths: Vec<Path>,
}

impl TilesField {
    pub(super) const ROOT: Tile = Tile(0);

    pub(super) fn new() -> Self {
        Self {
            positions: vec![Position {
                xmin: 0,
                ymin: 0,
                xmax: 1,
                ymax: 1,
            }],
            paths: Vec::new(),
        }
    }

    /// A tile on the `geometry` side of `start`, linked to it. PlantUML throws an `IllegalArgumentException`
    /// where making room moved `start` away from the new tile.
    pub(super) fn create_tile(
        &mut self,
        start: Tile,
        geometry: TileGeometry,
    ) -> Result<Tile, RuntimeException> {
        let result = Tile(self.positions.len());
        let position = self.get_free_position(start, geometry);
        self.positions.push(position);
        let path = self.build_path(
            Self::area(start, geometry),
            Self::area(result, geometry.opposite()),
        )?;
        self.paths.push(path);
        Ok(result)
    }

    /// A link from the `start_direction` side of `start` to the opposite side of `dest`. PlantUML throws an
    /// `IllegalArgumentException` unless the two sides face each other.
    pub(super) fn add_path(
        &mut self,
        start: Tile,
        dest: Tile,
        start_direction: TileGeometry,
    ) -> Result<(), RuntimeException> {
        let path = self.build_path(
            Self::area(start, start_direction),
            Self::area(dest, start_direction.opposite()),
        )?;
        self.paths.push(path);
        Ok(())
    }

    fn area(tile: Tile, geometry: TileGeometry) -> TileArea {
        TileArea { tile, geometry }
    }

    /// Side by side tiles, or a link westwards along a row over the tiles between.
    fn build_path(
        &self,
        tile_area1: TileArea,
        tile_area2: TileArea,
    ) -> Result<Path, RuntimeException> {
        let path = Path {
            start: tile_area1,
            dest: tile_area2,
        };
        if self.is_adjoining(tile_area1, tile_area2) {
            return Ok(path);
        }
        let pos1 = self.get_position(tile_area1.tile);
        let pos2 = self.get_position(tile_area2.tile);
        if pos1.ymin == pos2.ymin
            && pos1.ymax == pos2.ymax
            && tile_area1.geometry == TileGeometry::West
            && tile_area2.geometry == TileGeometry::East
        {
            return Ok(path);
        }
        Err(RuntimeException)
    }

    fn is_adjoining(&self, tile_area1: TileArea, tile_area2: TileArea) -> bool {
        let pos1 = self.get_position(tile_area1.tile);
        let pos2 = self.get_position(tile_area2.tile);
        if pos1 == pos2 {
            return true;
        }
        let (geom1, geom2) = (tile_area1.geometry, tile_area2.geometry);
        if geom1 != geom2.opposite() {
            return false;
        }
        let same_row = pos1.ymin == pos2.ymin && pos1.ymax == pos2.ymax;
        let same_column = pos1.xmin == pos2.xmin && pos1.xmax == pos2.xmax;
        match geom1 {
            TileGeometry::East => same_row && pos1.xmax + 1 == pos2.xmin,
            TileGeometry::West => same_row && pos1.xmin == pos2.xmax + 1,
            TileGeometry::South => same_column && pos1.ymax + 1 == pos2.ymin,
            TileGeometry::North => same_column && pos1.ymin == pos2.ymax + 1,
        }
    }

    fn get_tile_at(&self, position: Position) -> Option<usize> {
        self.positions.iter().position(|&p| p == position)
    }

    fn get_free_position(&mut self, start: Tile, geometry: TileGeometry) -> Position {
        let position = self.get_position(start).moved(geometry, 2);
        while self.is_occuped(position) {
            self.move_all_to_east(position);
        }
        position
    }

    /// Moves every tile east of or below `starting_position`'s top left corner two cells east, one at a time,
    /// each found again by its position, as PlantUML does.
    fn move_all_to_east(&mut self, starting_position: Position) {
        let to_move: Vec<Position> = self
            .positions
            .iter()
            .copied()
            .filter(|p| p.xmax >= starting_position.xmin && p.ymax >= starting_position.ymin)
            .collect();
        for p in to_move {
            let tile = self
                .get_tile_at(p)
                .expect("a position to move is held by a tile");
            self.positions[tile] = p.moved(TileGeometry::East, 2);
        }
    }

    fn is_occuped(&self, test: Position) -> bool {
        self.positions.contains(&test)
    }

    pub(super) fn get_position(&self, tile: Tile) -> Position {
        self.positions[tile.0]
    }

    pub(super) fn get_paths(&self) -> &[Path] {
        &self.paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xmins(field: &TilesField, tiles: usize) -> Vec<(i32, i32)> {
        (0..tiles)
            .map(|tile| {
                let position = field.get_position(Tile(tile));
                (position.xmin, position.ymin)
            })
            .collect()
    }

    #[test]
    fn a_new_tile_pushes_the_tiles_in_its_way_east() {
        let mut field = TilesField::new();
        let below = field
            .create_tile(TilesField::ROOT, TileGeometry::South)
            .unwrap();
        field.create_tile(below, TileGeometry::East).unwrap();
        let blocked = field.create_tile(below, TileGeometry::East).unwrap();
        assert_eq!(blocked, Tile(3));
        assert_eq!(xmins(&field, 4), [(0, 0), (0, 2), (4, 2), (2, 2)]);
    }

    #[test]
    fn a_tile_moved_away_from_its_new_neighbour_cannot_be_linked_to_it() {
        let mut field = TilesField::new();
        let below = field
            .create_tile(TilesField::ROOT, TileGeometry::South)
            .unwrap();
        assert_eq!(
            field.create_tile(below, TileGeometry::North),
            Err(RuntimeException)
        );
    }

    #[test]
    fn links_go_to_facing_sides_or_west_along_a_row() {
        let mut field = TilesField::new();
        let east = field
            .create_tile(TilesField::ROOT, TileGeometry::East)
            .unwrap();
        let far = field.create_tile(east, TileGeometry::East).unwrap();
        assert_eq!(
            field.add_path(far, TilesField::ROOT, TileGeometry::West),
            Ok(())
        );
        assert_eq!(field.add_path(far, far, TileGeometry::North), Ok(()));
        assert_eq!(
            field.add_path(TilesField::ROOT, far, TileGeometry::East),
            Err(RuntimeException)
        );
        assert_eq!(field.get_paths().len(), 4);
    }
}
