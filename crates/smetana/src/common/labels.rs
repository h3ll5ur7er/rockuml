//! `common/labels.c`: text labels. Smetana does not measure text: PlantUML passes each label's size in its text
//! (`_dim_W_H_`, see [`hackInitDimensionFromLabel`]), and any other line is 0 wide.

use crate::cgraph::id::agnameof;
use crate::cgraph::obj::{agraphof, agroot};
use crate::cgraph::{Agobj, aghead};
use crate::core::Globals;
use crate::core::consts::{LT_HTML, LT_RECD};
use crate::core::ids::TextlabelId;
use crate::core::jmath::max;
use crate::h::{pointf, textlabel_t};

/// `Macro.hackInitDimensionFromLabel`: the size PlantUML encodes in a label `_dim_W_H_`, if the label is one.
fn hackInitDimensionFromLabel(size: &mut pointf, label: &str) {
    let number = |s: &str| !s.is_empty() && s.chars().all(|c| c == '.' || c.is_ascii_digit());
    let dims = label
        .strip_prefix("_dim_")
        .and_then(|rest| rest.strip_suffix('_'))
        .and_then(|rest| rest.split_once('_'))
        .filter(|(w, h)| number(w) && number(h));
    if let Some((w, h)) = dims {
        size.x = parse_double(w);
        size.y = parse_double(h);
    }
}

/// `Double.parseDouble` of a run of digits and dots, which throws on more than one dot.
fn parse_double(s: &str) -> f64 {
    s.parse()
        .unwrap_or_else(|_| panic!("NumberFormatException: {s:?}"))
}

/// `storeline`: adds a line to a label and grows the label by its size.
fn storeline(zz: &mut Globals, lp: TextlabelId, line: String) {
    let label = &zz.textlabels[lp];
    let oldsz = label.nspans + 1;
    let (old, nspans, fontsize) = (label.span, label.nspans, label.fontsize);
    let span = zz.textspans.REALLOC(oldsz + 1, old);
    let mut size = pointf {
        x: 0.0,
        y: f64::from((fontsize * 1.20) as i32),
    };
    hackInitDimensionFromLabel(&mut size, &line);
    let s = &mut zz.textspans[span.at(nspans)];
    s.str = line;
    s.size.y = f64::from(size.y as i32);
    let label = &mut zz.textlabels[lp];
    label.span = Some(span);
    label.nspans += 1;
    // The width is the widest line's, the height the sum of the lines'.
    label.dimen.x = max(label.dimen.x, size.x);
    label.dimen.y += size.y;
}

/// `make_simple_label`: splits a label into lines at `\n`, `\l`, `\r` and newlines, and sizes it.
fn make_simple_label(zz: &mut Globals, lp: TextlabelId) {
    let label = &mut zz.textlabels[lp];
    label.dimen.x = 0.0;
    label.dimen.y = 0.0;
    if label.text.is_empty() {
        return;
    }
    let str: Vec<char> = label.text.chars().collect();
    let mut line = String::new();
    // Whether the buffer advanced since the line started: a trailing backslash advances it by a NUL, which
    // makes an empty last line.
    let mut pending = false;
    let mut p = 0;
    while p < str.len() {
        let c = str[p];
        p += 1;
        if c == '\\' {
            match str.get(p) {
                Some('n' | 'l' | 'r') => {
                    storeline(zz, lp, std::mem::take(&mut line));
                    pending = false;
                }
                Some(&t) => {
                    line.push(t);
                    pending = true;
                }
                None => pending = true,
            }
            if p < str.len() {
                p += 1;
            }
        } else if c == '\n' {
            // tcldot can enter real line ends.
            storeline(zz, lp, std::mem::take(&mut line));
            pending = false;
        } else {
            line.push(c);
            pending = true;
        }
    }
    if pending {
        storeline(zz, lp, line);
    }
    let label = &mut zz.textlabels[lp];
    label.space = label.dimen;
}

/// `make_label`: a new text label of `obj`. Record labels are kept verbatim for the record parser.
pub(crate) fn make_label(
    zz: &mut Globals,
    obj: Agobj,
    str: &str,
    kind: i32,
    fontsize: f64,
    fontname: &str,
    fontcolor: &str,
) -> TextlabelId {
    let g = match obj {
        Agobj::Graph(sg) => zz.graphs[sg].root,
        Agobj::Node(n) => agroot(zz, agraphof(zz, n)),
        Agobj::Edge(e) => agroot(zz, agraphof(zz, aghead(zz, e))),
    };
    let mut rv = textlabel_t {
        fontname: fontname.to_owned(),
        fontcolor: fontcolor.to_owned(),
        fontsize,
        charset: zz.gd(g).charset,
        ..textlabel_t::default()
    };
    if kind & LT_RECD != 0 {
        str.clone_into(&mut rv.text);
        if kind & LT_HTML != 0 {
            rv.html = true;
        }
        zz.textlabels.push(rv)
    } else if kind == LT_HTML {
        unimplemented!("HTML labels")
    } else {
        if rv.charset == 1 {
            unimplemented!("latin1ToUTF8");
        }
        // htmlEntityUTF8 only copies the text in Smetana.
        rv.text = strdup_and_subst_obj0(zz, str, obj);
        let lp = zz.textlabels.push(rv);
        make_simple_label(zz, lp);
        lp
    }
}

/// `strdup_and_subst_obj0` without backslash escaping: replaces `\N` with a node's name. Smetana supports no
/// other substitution; other escapes are kept for [`make_simple_label`].
fn strdup_and_subst_obj0(zz: &Globals, str: &str, obj: Agobj) -> String {
    let n_str = match obj {
        Agobj::Node(n) => agnameof(zz, n).expect("node name"),
        _ => "\\N".to_owned(),
    };
    let mut newstr = String::with_capacity(str.len());
    let mut s = str.chars();
    while let Some(c) = s.next() {
        if c != '\\' {
            newstr.push(c);
            continue;
        }
        match s.next() {
            Some('N') => newstr.push_str(&n_str),
            Some(e @ ('G' | 'E' | 'T' | 'H' | 'L' | '\\')) => {
                unimplemented!("label escape \\{e}")
            }
            Some(c) => {
                newstr.push('\\');
                newstr.push(c);
            }
            None => panic!("StringIndexOutOfBoundsException: label ends with a backslash"),
        }
    }
    newstr
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cgraph::graph::agopen;
    use crate::cgraph::node::agnode;
    use crate::core::consts::LT_NONE;
    use crate::h::cgraph::Agdirected;

    fn label(text: &str) -> textlabel_t {
        let mut zz = Globals::open();
        let g = agopen(&mut zz, Some("g"), Agdirected);
        crate::cgraph::rec::agbindrec(&mut zz, g, crate::cgraph::rec::Rec::Info);
        let n = agnode(&mut zz, g, Some("node1"), true).unwrap();
        let lp = make_label(
            &mut zz,
            n.into(),
            text,
            LT_NONE,
            14.0,
            "Times-Roman",
            "black",
        );
        zz.textlabels[lp].clone()
    }

    #[test]
    fn hack_labels_carry_their_size() {
        let l = label("_dim_120_34_");
        assert_eq!(l.dimen, pointf { x: 120.0, y: 34.0 });
        assert_eq!(l.space, l.dimen);
        assert_eq!(l.nspans, 1);
    }

    #[test]
    fn other_lines_are_empty_and_one_font_size_high() {
        let l = label("\\N");
        assert_eq!(l.text, "node1");
        assert_eq!(l.dimen, pointf { x: 0.0, y: 16.0 });
        let l = label("a\\lb\nc");
        assert_eq!(l.nspans, 3);
        assert_eq!(l.dimen.y, 48.0);
    }

    #[test]
    fn the_hack_needs_digits() {
        let l = label("_dim_-1_5_");
        assert_eq!(l.dimen, pointf { x: 0.0, y: 16.0 });
    }
}
