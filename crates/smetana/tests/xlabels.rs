//! Replays Smetana's external label placer on the cases in data/xlabels.txt (made by
//! tools/oracle/smetana-unit/xlabels.sh): Hilbert keys, R-tree searches and whole `placeLabels` runs, requiring
//! bit-identical results.

use smetana::h::{boxf, point, pointf};
use smetana::label::{
    Child, RTreeInsert, RTreeOpen, RTreeSearch, Rect_t, hd_hil_s_from_xy, label_params_t, object_t,
    placeLabels, xlabel_t,
};

const FIXTURE: &str = include_str!("data/xlabels.txt");

fn double(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).expect("hex double"))
}

fn int(s: &str) -> i32 {
    s.parse().expect("int")
}

fn rect(fields: &[&str]) -> Rect_t {
    Rect_t {
        boundary: [
            int(fields[0]),
            int(fields[1]),
            int(fields[2]),
            int(fields[3]),
        ],
    }
}

/// The fixture's lines, split into words, without the comment header.
fn lines() -> impl Iterator<Item = Vec<&'static str>> {
    FIXTURE
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split(' ').collect())
}

#[test]
fn hilbert_keys_match_java() {
    let mut cases = 0;
    for f in lines().filter(|f| f[0] == "hilbert") {
        let p = point {
            x: int(f[1]),
            y: int(f[2]),
        };
        assert_eq!(hd_hil_s_from_xy(p, int(f[3])), int(f[4]), "{f:?}");
        cases += 1;
    }
    assert_eq!(cases, 400);
}

#[test]
fn rtree_searches_match_java() {
    let mut lines = lines().peekable();
    let mut cases = 0;
    while let Some(f) = lines.next() {
        if f[0] != "rtree" {
            continue;
        }
        let mut tree = RTreeOpen();
        let mut data = 0;
        while lines.peek().is_some_and(|f| f[0] == "insert") {
            let f = lines.next().unwrap();
            let mut root = tree.root;
            RTreeInsert(&mut tree, &rect(&f[1..]), data, &mut root, 0);
            tree.root = root;
            data += 1;
        }
        assert_eq!(data.to_string(), f[1], "insert count");
        while lines.peek().is_some_and(|f| f[0] == "search") {
            let f = lines.next().unwrap();
            let found: Vec<String> = RTreeSearch(&tree, tree.root, &rect(&f[1..5]))
                .iter()
                .map(|b| match b.child {
                    Some(Child::Data(d)) => d.to_string(),
                    other => panic!("leaf child {other:?}"),
                })
                .collect();
            assert_eq!(found, f[6..], "case {cases}: {f:?}");
        }
        cases += 1;
    }
    assert_eq!(cases, 44);
}

#[test]
fn placed_labels_match_java() {
    let mut lines = lines().peekable();
    let mut cases = 0;
    while let Some(f) = lines.next() {
        if f[0] != "place" {
            continue;
        }
        let (n_objs, n_lbls) = (int(f[1]), int(f[2]));
        let params = label_params_t {
            force: f[4] == "1",
            bb: boxf {
                LL: pointf {
                    x: double(f[6]),
                    y: double(f[7]),
                },
                UR: pointf {
                    x: double(f[8]),
                    y: double(f[9]),
                },
            },
        };
        let objs: Vec<object_t> = (0..n_objs)
            .map(|_| {
                let o = lines.next().unwrap();
                object_t {
                    pos: pointf {
                        x: double(o[1]),
                        y: double(o[2]),
                    },
                    sz: pointf {
                        x: double(o[3]),
                        y: double(o[4]),
                    },
                    lbl: (o[5] != "-").then(|| o[5].parse().unwrap()),
                }
            })
            .collect();
        let mut lbls: Vec<xlabel_t> = (0..n_lbls)
            .map(|_| {
                let l = lines.next().unwrap();
                xlabel_t {
                    sz: pointf {
                        x: double(l[1]),
                        y: double(l[2]),
                    },
                    ..xlabel_t::default()
                }
            })
            .collect();
        let r = placeLabels(&objs, n_objs, &mut lbls, &params);
        assert_eq!(
            lines.next().unwrap(),
            ["result", &r.to_string()],
            "case {cases}"
        );
        for (i, l) in lbls.iter().enumerate() {
            let p = lines.next().unwrap();
            let expected = (double(p[1]).to_bits(), double(p[2]).to_bits(), int(p[3]));
            assert_eq!(
                (l.pos.x.to_bits(), l.pos.y.to_bits(), l.set),
                expected,
                "case {cases} label {i}"
            );
        }
        cases += 1;
    }
    assert_eq!(cases, 150);
}
