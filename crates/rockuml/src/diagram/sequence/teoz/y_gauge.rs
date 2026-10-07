/// Where a tile sits vertically: its top and bottom, and for messages the arrow's line and the point the
/// tile was chained from, so that parallel messages can share the arrow line (PlantUML's `YGauge`).
///
/// A tile's bottom is always `add_at_least` from something it is chained to, so that the solver can
/// only push it down.
#[derive(Clone, Debug)]
pub(super) struct YGauge {
    pub min: Real,
    pub max: Real,
    pub contact: Option<Real>,
    pub origin: Option<Real>,
}

use crate::real::Real;

impl YGauge {
    pub(super) fn new(min: Real, max: Real) -> Self {
        Self {
            min,
            max,
            contact: None,
            origin: None,
        }
    }

    pub(super) fn create(min: &Real, height: f64) -> Self {
        Self::new(min.clone(), min.add_at_least(height))
    }

    /// Below `current`, with the arrow line `contact_relative` below the top.
    pub(super) fn create_with_contact(
        current: &YGauge,
        contact_relative: f64,
        height: f64,
    ) -> Self {
        let origin = current.max.clone();
        let contact = origin.add_at_least(contact_relative);
        Self {
            min: contact.add_fixed(-contact_relative),
            max: contact.add_at_least(height - contact_relative),
            contact: Some(contact),
            origin: Some(origin),
        }
    }

    /// Beside `current`, sharing its arrow line (`&` messages).
    pub(super) fn create_parallel(current: &YGauge, contact_relative: f64, height: f64) -> Self {
        let (Some(contact), Some(origin)) = (&current.contact, &current.origin) else {
            return Self::create(&current.min, height);
        };
        contact.ensure_bigger_than(&origin.add_fixed(contact_relative));
        let max = contact.add_at_least(height - contact_relative);
        max.ensure_bigger_than(&current.max);
        Self {
            min: contact.add_fixed(-contact_relative),
            max,
            contact: Some(contact.clone()),
            origin: Some(origin.clone()),
        }
    }

    /// Below `current`, passing its arrow line on to what follows.
    pub(super) fn create_propagating(current: &YGauge, height: f64) -> Self {
        let min = current.max.clone();
        Self {
            max: min.add_at_least(height),
            min,
            contact: current.contact.clone(),
            origin: current.origin.clone(),
        }
    }
}
