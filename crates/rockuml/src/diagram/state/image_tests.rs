//! The images of state diagram leaves against PlantUML's, drawn on the debug surface. The fixture comes from
//! `tools/oracle/cuca-unit/StateImageDump.java` and holds the diagrams it draws.

use std::cell::RefCell;
use std::rc::Rc;

use super::tests::parse_source;
use crate::color::HColor;
use crate::java::double_to_string;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::ugraphic::UGraphic;
use crate::svek::{IEntityImage, create_entity_image_block};

const FIXTURE: &str = include_str!("../../../tests/data/state-images.txt");

/// The image as `StateImageDump.dump` writes it.
fn dump(image: &dyn IEntityImage) -> String {
    let dimension = image.calculate_dimension(&StringBounderDebug);
    let mut out = format!(
        "dimension: {} {}\n",
        double_to_string(dimension.width),
        double_to_string(dimension.height),
    );
    let debug = Rc::new(RefCell::new(UGraphicDebug::new("DATE".to_owned())));
    let ug = UGraphic::new(debug.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
    image.draw_u(&ug);
    let document = debug.borrow().document(&DebugHeader {
        dimension,
        scale_factor: 1.0,
        seed: 0,
        svg_link_target: None,
        hover_path_color_rgb: None,
        preserve_aspect_ratio: "none".to_owned(),
    });
    for line in document
        .split('\n')
        .skip_while(|line| !line.is_empty())
        .skip(1)
    {
        out += line;
        out.push('\n');
    }
    out
}

/// A case's name, source lines, and the dump of each leaf by `uid name`.
type FixtureCase = (
    &'static str,
    Vec<&'static str>,
    Vec<(&'static str, &'static str)>,
);

fn fixture_cases() -> Vec<FixtureCase> {
    FIXTURE
        .split("--- case ")
        .skip(1)
        .map(|case| {
            let (name, rest) = case.split_once('\n').unwrap();
            let (source, dumps) = rest.split_once("@enduml\n").unwrap();
            let mut lines: Vec<&str> = source.lines().collect();
            lines.push("@enduml");
            let dumps = dumps
                .split("=== ")
                .skip(1)
                .map(|dump| dump.split_once('\n').unwrap())
                .collect();
            (name, lines, dumps)
        })
        .collect()
}

#[test]
fn state_images_draw_like_plantuml() {
    let cases = fixture_cases();
    assert_eq!(cases.len(), 9);
    let mut count = 0;
    let mut failures = Vec::new();
    for (name, source, dumps) in cases {
        let diagram = parse_source(&source).unwrap();
        let cuca = &diagram.cuca;
        let leafs = cuca.leafs();
        assert_eq!(leafs.len(), dumps.len(), "{name}");
        for (leaf, (header, expected)) in leafs.into_iter().zip(dumps) {
            count += 1;
            let entity = cuca.entity(leaf);
            let label = format!("{name} {} {}", entity.get_uid(), entity.get_name(cuca));
            assert_eq!(label, header);
            let image = create_entity_image_block(leaf, cuca).unwrap();
            let actual = dump(image.as_ref());
            if actual != expected {
                failures.push(format!(
                    "=== {header}\n--- expected\n{expected}--- actual\n{actual}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {count} images differ; first:\n{}",
        failures.len(),
        failures[0]
    );
}
