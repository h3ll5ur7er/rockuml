//! Where a tile deep inside a tree is drawn, relative to the tree's root (PlantUML's `Genealogy`): loops
//! find their `break`s with it.

use std::collections::HashMap;
use std::rc::Rc;

use super::{Ftile, same};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;

pub(crate) struct Genealogy {
    /// Keyed by the address of the child, as PlantUML's identity map is keyed by the tile.
    my_father_is: HashMap<*const (), Rc<dyn Ftile>>,
    root: Rc<dyn Ftile>,
}

impl Genealogy {
    pub(crate) fn new(root: &Rc<dyn Ftile>) -> Self {
        let mut genealogy = Self {
            my_father_is: HashMap::new(),
            root: root.clone(),
        };
        genealogy.process(root);
        genealogy
    }

    fn process(&mut self, current: &Rc<dyn Ftile>) {
        for child in current.get_my_children() {
            self.set_my_father(&child, current);
            self.process(&child);
        }
    }

    /// PlantUML refuses a tile with two fathers; a tree has none, so the first is kept.
    fn set_my_father(&mut self, child: &Rc<dyn Ftile>, father: &Rc<dyn Ftile>) {
        self.my_father_is
            .entry(key(child.as_ref()))
            .or_insert_with(|| father.clone());
    }

    fn get_my_father(&self, me: &dyn Ftile) -> Option<&Rc<dyn Ftile>> {
        self.my_father_is.get(&key(me))
    }

    /// Where `child` is drawn, composed from its father up to the root. A tile outside the tree stops the
    /// climb where PlantUML would fail.
    pub(crate) fn get_translate(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let mut current = child;
        let mut result = UTranslate::default();
        while !same(current, self.root.as_ref()) {
            let Some(father) = self.get_my_father(current) else {
                break;
            };
            let tr = father.get_translate_for(current, string_bounder);
            result = tr.compose(result);
            current = father.as_ref();
        }
        result
    }
}

fn key(tile: &dyn Ftile) -> *const () {
    std::ptr::from_ref(tile).cast()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
    use crate::ftile::{AbstractFtile, FtileGeometry, Swimable};
    use crate::klimt::debug::StringBounderDebug;
    use crate::klimt::ugraphic::UGraphic;
    use crate::skin::SkinParam;

    /// A tile drawing each child `step` further right and down than the one before.
    struct Nest {
        base: AbstractFtile,
        step: f64,
        children: Vec<Rc<dyn Ftile>>,
    }

    fn nest(step: f64, children: Vec<Rc<dyn Ftile>>) -> Rc<dyn Ftile> {
        Rc::new(Nest {
            base: AbstractFtile::new(Rc::new(SkinParam::default())),
            step,
            children,
        })
    }

    impl Swimable for Nest {
        fn get_swimlanes(&self) -> SwimlaneSet {
            SwimlaneSet::new()
        }

        fn get_swimlane_in(&self) -> Option<SwimlaneId> {
            None
        }

        fn get_swimlane_out(&self) -> Option<SwimlaneId> {
            None
        }
    }

    impl Ftile for Nest {
        fn skin_param(&self) -> &SkinParam {
            self.base.skin_param()
        }

        fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
            FtileGeometry::new(0.0, 0.0, 0.0, 0.0)
        }

        fn get_translate_for(
            &self,
            child: &dyn Ftile,
            _string_bounder: &dyn StringBounder,
        ) -> UTranslate {
            let index = self
                .children
                .iter()
                .position(|tile| same(tile.as_ref(), child))
                .expect("a child");
            let offset = self.step * (index + 1) as f64;
            UTranslate::new(offset, offset)
        }

        fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
            self.children.clone()
        }

        fn draw_u(&self, _ug: &UGraphic) {}
    }

    #[test]
    fn translations_compose_from_the_child_up_to_the_root() {
        let deep = nest(0.0, vec![]);
        let middle = nest(10.0, vec![nest(0.0, vec![]), deep.clone()]);
        let root = nest(100.0, vec![middle.clone()]);
        let genealogy = Genealogy::new(&root);
        let string_bounder = StringBounderDebug;
        assert_eq!(
            genealogy.get_translate(deep.as_ref(), &string_bounder),
            UTranslate::new(120.0, 120.0)
        );
        assert_eq!(
            genealogy.get_translate(middle.as_ref(), &string_bounder),
            UTranslate::new(100.0, 100.0)
        );
        assert_eq!(
            genealogy.get_translate(root.as_ref(), &string_bounder),
            UTranslate::default()
        );
    }
}
