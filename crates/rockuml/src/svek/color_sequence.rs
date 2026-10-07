/// Numbers the nodes and clusters of one layout, from 2 (PlantUML's `ColorSequence`, named for the colours
/// Graphviz layouts tag them with). Smetana layouts name their nodes after these numbers.
pub(crate) struct ColorSequence {
    cpt: i32,
}

impl Default for ColorSequence {
    fn default() -> Self {
        Self { cpt: 1 }
    }
}

impl ColorSequence {
    pub(crate) fn get_value(&mut self) -> i32 {
        self.cpt += 1;
        self.cpt
    }
}
