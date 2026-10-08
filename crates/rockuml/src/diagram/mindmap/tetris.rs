//! Packs the subtrees of an idea against each other like Tetris pieces (PlantUML's `Tetris`,
//! `StripeFrontier`, `Stripe`, `SymetricalTee` and `SymetricalTeePositioned`).

/// A subtree seen from its parent: the node (`1`) and, past it, its children (`2`), each centred on the
/// same axis.
#[derive(Clone, Copy, Debug)]
pub(super) struct SymetricalTee {
    thickness1: f64,
    elongation1: f64,
    thickness2: f64,
    elongation2: f64,
}

impl SymetricalTee {
    pub(super) fn new(
        thickness1: f64,
        elongation1: f64,
        thickness2: f64,
        elongation2: f64,
    ) -> Self {
        Self {
            thickness1,
            elongation1,
            thickness2,
            elongation2,
        }
    }
}

/// A tee with its axis at `y`.
#[derive(Clone, Copy, Debug)]
pub(super) struct SymetricalTeePositioned {
    tee: SymetricalTee,
    y: f64,
}

/// A horizontal segment: `x1..x2` at `y`.
struct Segment {
    x1: f64,
    x2: f64,
    y: f64,
}

impl SymetricalTeePositioned {
    fn new(tee: SymetricalTee) -> Self {
        Self { tee, y: 0.0 }
    }

    pub(super) fn get_y(&self) -> f64 {
        self.y
    }

    fn move_so_that_segment_a1_is_on(&mut self, new_y: f64) {
        let current = self.get_segment_a1().y;
        self.y += new_y - current;
    }

    fn move_so_that_segment_a2_is_on(&mut self, new_y: f64) {
        let current = self.get_segment_a2().y;
        self.y += new_y - current;
    }

    fn move_by(&mut self, delta: f64) {
        self.y += delta;
    }

    fn get_segment_a1(&self) -> Segment {
        Segment {
            x1: 0.0,
            x2: self.tee.elongation1,
            y: self.y - self.tee.thickness1 / 2.0,
        }
    }

    fn get_segment_b1(&self) -> Segment {
        Segment {
            x1: 0.0,
            x2: self.tee.elongation1,
            y: self.y + self.tee.thickness1 / 2.0,
        }
    }

    fn get_segment_a2(&self) -> Segment {
        Segment {
            x1: self.tee.elongation1,
            x2: self.tee.elongation1 + self.tee.elongation2,
            y: self.y - self.tee.thickness2 / 2.0,
        }
    }

    fn get_segment_b2(&self) -> Segment {
        Segment {
            x1: self.tee.elongation1,
            x2: self.tee.elongation1 + self.tee.elongation2,
            y: self.y + self.tee.thickness2 / 2.0,
        }
    }

    fn get_max_x(&self) -> f64 {
        self.tee.elongation1 + self.tee.elongation2
    }

    fn get_max_y(&self) -> f64 {
        self.y + (self.tee.thickness1 / 2.0).max(self.tee.thickness2 / 2.0)
    }

    fn get_min_y(&self) -> f64 {
        self.y - (self.tee.thickness1 / 2.0).max(self.tee.thickness2 / 2.0)
    }

    /// The lower of the two placements.
    fn get_max(self, other: Self) -> Self {
        if other.y > self.y { other } else { self }
    }
}

/// `x1..x2` is blocked down to `value`.
#[derive(Clone, Copy, Debug)]
struct Stripe {
    x1: f64,
    x2: f64,
    value: f64,
}

/// How far down each column is taken, as stripes covering every x in order.
struct StripeFrontier {
    stripes: Vec<Stripe>,
}

impl StripeFrontier {
    fn new() -> Self {
        Self {
            stripes: vec![Stripe {
                x1: -f64::MAX,
                x2: f64::MAX,
                value: -f64::MAX,
            }],
        }
    }

    fn is_empty(&self) -> bool {
        self.stripes.len() == 1
    }

    /// How far down `x1..x2` is taken.
    fn get_contact(&self, x1: f64, x2: f64) -> f64 {
        self.collisionning(x1, x2)
            .map(|index| self.stripes[index].value)
            .fold(-f64::MAX, f64::max)
    }

    fn add_segment(&mut self, x1: f64, x2: f64, value: f64) {
        assert!(x1 < x2, "a segment is not empty");
        let collisions: Vec<usize> = self.collisionning(x1, x2).collect();
        if collisions.len() > 1 {
            let starts: Vec<f64> = collisions[1..]
                .iter()
                .map(|&index| self.stripes[index].x1)
                .collect();
            let mut x = x1;
            for start in starts {
                self.add_segment(x, start, value);
                x = start;
            }
            self.add_segment(x, x2, value);
        } else {
            self.add_single_internal(x1, x2, value, collisions[0]);
        }
    }

    fn add_single_internal(&mut self, x1: f64, x2: f64, value: f64, touch: usize) {
        let touched = self.stripes[touch];
        if value <= touched.value {
            return;
        }
        let mut replacement = Vec::with_capacity(3);
        if touched.x1 != x1 {
            replacement.push(Stripe {
                x1: touched.x1,
                x2: x1,
                value: touched.value,
            });
        }
        replacement.push(Stripe { x1, x2, value });
        if x2 != touched.x2 {
            replacement.push(Stripe {
                x1: x2,
                x2: touched.x2,
                value: touched.value,
            });
        }
        self.stripes.splice(touch..=touch, replacement);
    }

    /// The stripes `x1..x2` overlaps, in order.
    fn collisionning(&self, x1: f64, x2: f64) -> impl Iterator<Item = usize> + '_ {
        let first = self
            .stripes
            .iter()
            .position(|stripe| x1 < stripe.x2)
            .expect("the stripes cover every x");
        let last = first
            + self.stripes[first..]
                .iter()
                .position(|stripe| x2 <= stripe.x2)
                .expect("the stripes cover every x");
        first..=last
    }
}

pub(super) struct Tetris {
    frontier: StripeFrontier,
    elements: Vec<SymetricalTeePositioned>,
    min_y: f64,
    max_y: f64,
}

impl Tetris {
    pub(super) fn new() -> Self {
        Self {
            frontier: StripeFrontier::new(),
            elements: Vec::new(),
            min_y: f64::MAX,
            max_y: -f64::MAX,
        }
    }

    /// Centres the pieces on 0.
    pub(super) fn balance(&mut self) {
        if self.elements.is_empty() {
            return;
        }
        for element in &self.elements {
            self.min_y = self.min_y.min(element.get_min_y());
            self.max_y = self.max_y.max(element.get_max_y());
        }
        let mean = f64::midpoint(self.min_y, self.max_y);
        for element in &mut self.elements {
            element.move_by(-mean);
        }
    }

    pub(super) fn get_height(&self) -> f64 {
        if self.elements.is_empty() {
            return 0.0;
        }
        self.max_y - self.min_y
    }

    pub(super) fn get_width(&self) -> f64 {
        self.elements
            .iter()
            .fold(0.0, |result, tee| f64::max(result, tee.get_max_x()))
    }

    /// Drops the piece in as high as it fits below the pieces already in.
    pub(super) fn add(&mut self, tee: SymetricalTee) {
        if self.frontier.is_empty() {
            self.add_internal(SymetricalTeePositioned::new(tee));
            return;
        }
        let c1 = self.frontier.get_contact(0.0, tee.elongation1);
        let c2 = self
            .frontier
            .get_contact(tee.elongation1, tee.elongation1 + tee.elongation2);
        let mut p1 = SymetricalTeePositioned::new(tee);
        p1.move_so_that_segment_a1_is_on(c1);
        let mut p2 = SymetricalTeePositioned::new(tee);
        p2.move_so_that_segment_a2_is_on(c2);
        self.add_internal(p1.get_max(p2));
    }

    fn add_internal(&mut self, result: SymetricalTeePositioned) {
        self.elements.push(result);
        let b1 = result.get_segment_b1();
        self.frontier.add_segment(b1.x1, b1.x2, b1.y);
        let b2 = result.get_segment_b2();
        if b2.x1 != b2.x2 {
            self.frontier.add_segment(b2.x1, b2.x2, b2.y);
        }
    }

    pub(super) fn get_elements(&self) -> &[SymetricalTeePositioned] {
        &self.elements
    }
}
