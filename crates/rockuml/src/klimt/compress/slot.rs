use std::cmp::Ordering;

/// Where a drawing has something along one axis, or where it has nothing (`Slot`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Slot {
    start: f64,
    end: f64,
}

impl Slot {
    /// PlantUML refuses a slot without extent; [`SlotSet::add_slot`] never makes one.
    fn new(start: f64, end: f64) -> Self {
        debug_assert!(start < end, "start={start} end={end}");
        Self { start, end }
    }

    pub(crate) fn get_start(self) -> f64 {
        self.start
    }

    pub(crate) fn get_end(self) -> f64 {
        self.end
    }

    pub(crate) fn size(self) -> f64 {
        self.end - self.start
    }

    /// Both ends belong to the slot, so slots that touch merge.
    fn contains(self, v: f64) -> bool {
        v >= self.start && v <= self.end
    }

    fn intersect(self, other: Slot) -> bool {
        self.contains(other.start)
            || self.contains(other.end)
            || other.contains(self.start)
            || other.contains(self.end)
    }

    fn merge(self, other: Slot) -> Slot {
        Slot::new(self.start.min(other.start), self.end.max(other.end))
    }

    fn compare_to(self, other: Slot) -> Ordering {
        if self.start < other.start {
            Ordering::Less
        } else if self.start > other.start {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

/// Slots that never touch, in the order they were made (`SlotSet`).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SlotSet {
    all: Vec<Slot>,
}

impl SlotSet {
    /// Adds `start..end`, merged with every slot it touches.
    ///
    /// PlantUML fails the whole diagram on a shape without extent along the axis, such as a text in a font
    /// of size 0 (`:<size:0>x</size>;`); here such a shape takes no room.
    pub(crate) fn add_slot(&mut self, start: f64, end: f64) {
        if start >= end {
            return;
        }
        let mut new_slot = Slot::new(start, end);
        let mut collisions = Vec::new();
        self.all.retain(|&slot| {
            let collides = slot.intersect(new_slot);
            if collides {
                collisions.push(slot);
            }
            !collides
        });
        for slot in collisions {
            new_slot = new_slot.merge(slot);
        }
        self.all.push(new_slot);
    }

    /// The slots shrunk by `margin` at both ends; those not wider than both margins go.
    pub(crate) fn smaller(&self, margin: f64) -> SlotSet {
        let mut result = SlotSet::default();
        for slot in &self.all {
            if slot.size() <= 2.0 * margin {
                continue;
            }
            result.add_slot(slot.get_start() + margin, slot.get_end() - margin);
        }
        result
    }

    /// The gaps between the slots. PlantUML sorts the set itself; nothing reads it afterwards.
    pub(crate) fn reverse(&self) -> SlotSet {
        let mut sorted = self.all.clone();
        sorted.sort_by(|a, b| a.compare_to(*b));
        let mut result = SlotSet::default();
        for pair in sorted.windows(2) {
            result.add_slot(pair[0].get_end(), pair[1].get_start());
        }
        result
    }

    pub(crate) fn get_slots(&self) -> &[Slot] {
        &self.all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots(set: &SlotSet) -> Vec<(f64, f64)> {
        set.get_slots()
            .iter()
            .map(|slot| (slot.get_start(), slot.get_end()))
            .collect()
    }

    #[test]
    fn touching_slots_merge_and_the_merged_slot_goes_last() {
        let mut set = SlotSet::default();
        set.add_slot(0.0, 10.0);
        set.add_slot(30.0, 40.0);
        set.add_slot(10.0, 20.0);
        assert_eq!(slots(&set), [(30.0, 40.0), (0.0, 20.0)]);
    }

    #[test]
    fn the_gaps_come_sorted_and_shrink_by_the_margin() {
        let mut set = SlotSet::default();
        set.add_slot(50.0, 60.0);
        set.add_slot(0.0, 10.0);
        set.add_slot(25.0, 30.0);
        assert_eq!(slots(&set.reverse()), [(10.0, 25.0), (30.0, 50.0)]);
        assert_eq!(
            slots(&set.reverse().smaller(5.0)),
            [(15.0, 20.0), (35.0, 45.0)]
        );
    }

    #[test]
    fn shapes_without_extent_take_no_room() {
        let mut set = SlotSet::default();
        set.add_slot(10.0, 10.0);
        set.add_slot(f64::MAX, -f64::MAX);
        assert_eq!(slots(&set), []);
    }
}
