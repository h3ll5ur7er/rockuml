use super::slot::{Slot, SlotSet};

/// Moves coordinates back by the free space before them, squeezing that space out (`CompressionTransform`).
pub(crate) struct CompressionTransform {
    all: Vec<Slot>,
}

impl CompressionTransform {
    /// Squeezes out `slot_set`, the free space.
    pub(crate) fn new(slot_set: &SlotSet) -> Self {
        Self {
            all: slot_set.get_slots().to_vec(),
        }
    }

    pub(crate) fn transform(&self, v: f64) -> f64 {
        v - self.get_compress_delta(v)
    }

    /// Summed in the order of the slots, as PlantUML sums them.
    fn get_compress_delta(&self, v: f64) -> f64 {
        let mut result = 0.0;
        for slot in &self.all {
            if slot.get_start() > v {
                continue;
            }
            if v > slot.get_end() {
                result += slot.size();
            } else {
                result += v - slot.get_start();
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_move_back_by_the_free_space_before_them() {
        let mut free = SlotSet::default();
        free.add_slot(10.0, 20.0);
        free.add_slot(30.0, 35.0);
        let transform = CompressionTransform::new(&free);
        assert_eq!(transform.transform(5.0), 5.0);
        assert_eq!(transform.transform(15.0), 10.0);
        assert_eq!(transform.transform(25.0), 15.0);
        assert_eq!(transform.transform(40.0), 25.0);
    }
}
