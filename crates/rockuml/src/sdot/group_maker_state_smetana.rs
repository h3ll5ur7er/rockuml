//! The image of a composite state laid out on its own, with its concurrent regions (PlantUML's
//! `GroupMakerStateSmetana`).

use super::CucaDiagramFileMakerSmetana;
use super::padded_entity_image::PaddedEntityImage;
use crate::abel::{EntityId, GroupType, LeafType};
use crate::diagram::NotYetPorted;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::svek::image::EntityImageState;
use crate::svek::{ConcurrentStates, IEntityImage, InnerStateAutonom};

/// Room around each concurrent region, which `ConcurrentStates` stacks edge to edge.
const REGION_PADDING: f64 = 6.0;

/// How far the separators between regions overshoot them, which the border around them makes room for.
const SEPARATOR_OVERSHOOT: f64 = 8.0;

pub(crate) struct GroupMakerStateSmetana<'a> {
    diagram: &'a CucaDiagram,
    group: EntityId,
    string_bounder: &'a dyn StringBounder,
}

impl<'a> GroupMakerStateSmetana<'a> {
    /// # Panics
    ///
    /// If `group` is a leaf.
    pub(crate) fn new(
        diagram: &'a CucaDiagram,
        group: EntityId,
        string_bounder: &'a dyn StringBounder,
    ) -> Self {
        assert!(diagram.entity(group).is_group(), "only groups are laid out");
        Self {
            diagram,
            group,
            string_bounder,
        }
    }

    /// # Panics
    ///
    /// For groups other than states and their concurrent regions.
    pub(crate) fn get_image(&self) -> Result<Box<dyn IEntityImage>, NotYetPorted> {
        let diagram = self.diagram;
        let group = diagram.entity(self.group);
        if group.count_children(diagram) == 0 && group.groups(diagram).is_empty() {
            return Ok(Box::new(EntityImageState::new(group, diagram)));
        }
        match group.get_group_type() {
            GroupType::ConcurrentState => {
                return Ok(Box::new(PaddedEntityImage::uniform(
                    self.sub_layout()?,
                    REGION_PADDING,
                )));
            }
            GroupType::State => {}
            other => panic!("{other:?} groups are not laid out on their own"),
        }
        let regions: Vec<EntityId> = group
            .leafs(diagram)
            .into_iter()
            .filter(|leaf| diagram.entity(*leaf).get_leaf_type() == Some(LeafType::StateConcurrent))
            .collect();
        let image: Box<dyn IEntityImage> = if regions.is_empty() {
            self.sub_layout()?
        } else {
            // The group's own states are the first region; the others were laid out already, deepest first.
            let mut inners: Vec<Box<dyn IEntityImage>> = vec![Box::new(PaddedEntityImage::uniform(
                self.sub_layout()?,
                REGION_PADDING,
            ))];
            for region in regions {
                let image = diagram
                    .get_svek_image(region)
                    .expect("concurrent regions are laid out before their state");
                inners.push(Box::new(image));
            }
            let separator = group
                .concurrent_separator
                .expect("a state with regions knows how they are separated");
            let stacked = ConcurrentStates::new(inners, separator, diagram);
            Box::new(with_separator_overshoot(Box::new(stacked), separator))
        };
        Ok(Box::new(InnerStateAutonom::new(image, group, diagram)))
    }

    fn sub_layout(&self) -> Result<Box<dyn IEntityImage>, NotYetPorted> {
        CucaDiagramFileMakerSmetana::with_root(self.diagram.clone(), self.group)
            .get_image(self.string_bounder)
    }
}

/// Side by side regions (`|`) have separators overshooting at the bottom, stacked ones (`-`) at the right.
fn with_separator_overshoot(
    stacked: Box<dyn IEntityImage>,
    concurrent_separator: char,
) -> PaddedEntityImage {
    let bottom = if concurrent_separator == '|' {
        SEPARATOR_OVERSHOOT
    } else {
        0.0
    };
    let right = if concurrent_separator == '-' {
        SEPARATOR_OVERSHOOT
    } else {
        0.0
    };
    PaddedEntityImage::new(stacked, 0.0, 0.0, right, bottom)
}
