//! Positions solved by constraints (PlantUML's `real` package): each `Real` is a coordinate that can only be
//! pushed forward by "at least this far from that" forces, until no force is violated any more.
//!
//! The solver relaxes the forces in the order they were added and stops at the first fixed point, so the
//! order constraints are added in is part of the result, as in PlantUML.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// A coordinate on one line of constraints.
#[derive(Clone)]
pub(crate) struct Real(Rc<RealNode>);

struct RealNode {
    line: Rc<RealLine>,
    kind: RealKind,
}

enum RealKind {
    /// A free point that forces move (`RealImpl`).
    Free(Cell<f64>),
    /// Another point at a fixed distance; moving it moves the other point (`RealDelta`).
    Delta { delegated: Real, diff: f64 },
    /// Like `Delta`, with a distance that is read when the value is (`RealDeltaLive`): margins can still grow
    /// after the constraint is registered.
    LiveDelta {
        delegated: Real,
        offset: Rc<dyn Fn() -> f64>,
    },
    /// The largest of several points, remembered from the first time it is read (`RealMax`).
    Max {
        all: Vec<Real>,
        cache: Cell<Option<f64>>,
    },
    /// The smallest of several points, remembered from the first time it is read (`RealMin`).
    Min {
        all: Vec<Real>,
        cache: Cell<Option<f64>>,
    },
    /// Halfway between two points; moving it moves both (`RealMiddle2`).
    Middle(Real, Real),
}

#[derive(Default)]
struct RealLine {
    forces: RefCell<Vec<PositiveForce>>,
}

/// Keeps `moving` at least `minimum_distance` after `fixed`.
struct PositiveForce {
    fixed: Real,
    moving: Real,
    minimum_distance: f64,
}

impl PositiveForce {
    fn apply(&self) -> bool {
        let moving = self.moving.current_value();
        let fixed = self.fixed.current_value();
        let diff = moving - fixed - self.minimum_distance;
        // A chained sum can be a few ULPs off, so that moving by `-diff` changes nothing and the force
        // would fire forever; the slack grows with the magnitude for that reason.
        let epsilon = 1e-6_f64.max(1000.0 * ulp(moving.abs().max(fixed.abs())));
        if diff >= -epsilon {
            return false;
        }
        self.moving.move_by(-diff);
        true
    }
}

/// Java's `Math.ulp` for finite numbers: the distance to the next number of larger magnitude.
fn ulp(value: f64) -> f64 {
    let magnitude = value.abs();
    f64::from_bits(magnitude.to_bits() + 1) - magnitude
}

impl RealLine {
    /// Applies the forces in order, over and over, until none moves a point.
    fn compile(&self) {
        const ITERATION_LIMIT: u32 = 99_999;
        let forces = self.forces.borrow();
        for _ in 0..=ITERATION_LIMIT {
            let mut moved = false;
            for force in forces.iter() {
                moved |= force.apply();
            }
            if !moved {
                return;
            }
        }
        panic!("Infinite Loop?");
    }
}

impl Real {
    /// The origin of a new line of constraints, at 0.
    pub(crate) fn origin() -> Self {
        Self::on(Rc::new(RealLine::default()), RealKind::Free(Cell::new(0.0)))
    }

    fn on(line: Rc<RealLine>, kind: RealKind) -> Self {
        Self(Rc::new(RealNode { line, kind }))
    }

    fn line(&self) -> Rc<RealLine> {
        self.0.line.clone()
    }

    pub(crate) fn current_value(&self) -> f64 {
        match &self.0.kind {
            RealKind::Free(value) => value.get(),
            RealKind::Delta { delegated, diff } => delegated.current_value() + diff,
            RealKind::LiveDelta { delegated, offset } => delegated.current_value() + offset(),
            RealKind::Max { all, cache } => Self::cached(cache, all, |value, best| value > best),
            RealKind::Min { all, cache } => Self::cached(cache, all, |value, best| value < best),
            RealKind::Middle(p1, p2) => (p1.current_value() + p2.current_value()) / 2.0,
        }
    }

    /// PlantUML keeps the first value it computes, even when the points move later.
    fn cached(cache: &Cell<Option<f64>>, all: &[Real], is_better: fn(f64, f64) -> bool) -> f64 {
        if let Some(value) = cache.get() {
            return value;
        }
        let value = all
            .iter()
            .map(Real::current_value)
            .reduce(|best, value| if is_better(value, best) { value } else { best })
            .expect("max and min have points");
        cache.set(Some(value));
        value
    }

    /// The point always `delta` after this one.
    pub(crate) fn add_fixed(&self, delta: f64) -> Real {
        Self::on(
            self.line(),
            RealKind::Delta {
                delegated: self.clone(),
                diff: delta,
            },
        )
    }

    /// A new point at least `delta` after this one.
    pub(crate) fn add_at_least(&self, delta: f64) -> Real {
        match &self.0.kind {
            RealKind::Free(value) => {
                let result = Self::on(self.line(), RealKind::Free(Cell::new(value.get() + delta)));
                self.add_force(self.clone(), result.clone(), delta);
                result
            }
            RealKind::Delta { delegated, diff } => delegated.add_at_least(delta).add_fixed(*diff),
            RealKind::LiveDelta { delegated, offset } => delegated
                .add_at_least(delta)
                .with_live_offset(offset.clone()),
            RealKind::Max { .. } | RealKind::Min { .. } | RealKind::Middle(..) => {
                unreachable!("PlantUML has no point at least after a max, min or middle")
            }
        }
    }

    /// Pushes this point to `other` or beyond.
    pub(crate) fn ensure_bigger_than(&self, other: &Real) {
        match &self.0.kind {
            RealKind::Free(_) | RealKind::Middle(..) => {
                self.add_force(other.clone(), self.clone(), 0.0);
            }
            RealKind::Delta { delegated, diff } => {
                delegated.ensure_bigger_than(&other.add_fixed(-diff))
            }
            RealKind::LiveDelta { delegated, offset } => {
                let offset = offset.clone();
                delegated.ensure_bigger_than(&other.with_live_offset(Rc::new(move || -offset())));
            }
            RealKind::Min { all, .. } => {
                for real in all {
                    real.ensure_bigger_than(other);
                }
            }
            RealKind::Max { .. } => unreachable!("PlantUML never pushes a max"),
        }
    }

    /// This point plus an offset read whenever the value is.
    pub(crate) fn with_live_offset(&self, offset: Rc<dyn Fn() -> f64>) -> Real {
        Self::on(
            self.line(),
            RealKind::LiveDelta {
                delegated: self.clone(),
                offset,
            },
        )
    }

    fn move_by(&self, delta: f64) {
        match &self.0.kind {
            RealKind::Free(value) => value.set(value.get() + delta),
            RealKind::Delta { delegated, .. } | RealKind::LiveDelta { delegated, .. } => {
                delegated.move_by(delta);
            }
            RealKind::Middle(p1, p2) => {
                p1.move_by(delta / 2.0);
                p2.move_by(delta / 2.0);
            }
            RealKind::Max { .. } | RealKind::Min { .. } => {
                unreachable!("PlantUML never moves a max or min")
            }
        }
    }

    fn add_force(&self, fixed: Real, moving: Real, minimum_distance: f64) {
        self.0.line.forces.borrow_mut().push(PositiveForce {
            fixed,
            moving,
            minimum_distance,
        });
    }

    /// Solves every constraint of this point's line.
    pub(crate) fn compile_now(&self) {
        self.0.line.compile();
    }

    pub(crate) fn max(reals: Vec<Real>) -> Real {
        let line = reals.first().expect("max of at least one point").line();
        Self::on(
            line,
            RealKind::Max {
                all: reals,
                cache: Cell::new(None),
            },
        )
    }

    pub(crate) fn min(reals: Vec<Real>) -> Real {
        let line = reals.first().expect("min of at least one point").line();
        Self::on(
            line,
            RealKind::Min {
                all: reals,
                cache: Cell::new(None),
            },
        )
    }

    pub(crate) fn middle(p1: &Real, p2: &Real) -> Real {
        Self::on(p1.line(), RealKind::Middle(p1.clone(), p2.clone()))
    }
}

impl fmt::Debug for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Real({})", self.current_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_pushed_just_far_enough() {
        let origin = Real::origin();
        let a = origin.add_at_least(10.0);
        let b = a.add_at_least(5.0);
        b.ensure_bigger_than(&origin.add_fixed(40.0));
        origin.compile_now();
        assert_eq!((a.current_value(), b.current_value()), (10.0, 40.0));
        a.ensure_bigger_than(&origin.add_fixed(50.0));
        origin.compile_now();
        assert_eq!((a.current_value(), b.current_value()), (50.0, 55.0));
    }

    #[test]
    fn fixed_offsets_move_the_point_they_follow() {
        let origin = Real::origin();
        let a = origin.add_at_least(0.0);
        let right_of_a = a.add_fixed(20.0);
        right_of_a.ensure_bigger_than(&origin.add_fixed(50.0));
        origin.compile_now();
        assert_eq!(a.current_value(), 30.0);
    }

    /// Moving a middle moves each end by half the distance, so the middle itself only creeps towards
    /// its target, as in PlantUML.
    #[test]
    fn a_middle_moves_both_ends_by_half() {
        let origin = Real::origin();
        let a = origin.add_at_least(0.0);
        let b = origin.add_at_least(10.0);
        let middle = Real::middle(&a, &b);
        middle.ensure_bigger_than(&origin.add_fixed(25.0));
        origin.compile_now();
        assert!((middle.current_value() - 25.0).abs() < 1e-5);
        assert!((b.current_value() - a.current_value() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn live_offsets_are_read_when_solving() {
        let origin = Real::origin();
        let margin = Rc::new(Cell::new(0.0));
        let a = origin.add_at_least(0.0);
        let read = margin.clone();
        let before_a = a.with_live_offset(Rc::new(move || -read.get()));
        before_a.ensure_bigger_than(&origin.add_fixed(10.0));
        margin.set(15.0);
        origin.compile_now();
        assert_eq!(a.current_value(), 25.0);
    }

    #[test]
    fn max_and_min_keep_their_first_value_like_plantuml() {
        let origin = Real::origin();
        let a = origin.add_at_least(5.0);
        let b = origin.add_at_least(7.0);
        let max = Real::max(vec![a.clone(), b.clone()]);
        let min = Real::min(vec![a.clone(), b]);
        assert_eq!((max.current_value(), min.current_value()), (7.0, 5.0));
        a.ensure_bigger_than(&origin.add_fixed(100.0));
        origin.compile_now();
        assert_eq!((max.current_value(), min.current_value()), (7.0, 5.0));
    }

    #[test]
    fn pushing_a_min_pushes_all_its_points() {
        let origin = Real::origin();
        let a = origin.add_at_least(5.0);
        let b = origin.add_at_least(7.0);
        Real::min(vec![a.clone(), b.clone()]).ensure_bigger_than(&origin.add_fixed(10.0));
        origin.compile_now();
        assert_eq!((a.current_value(), b.current_value()), (10.0, 10.0));
    }
}
