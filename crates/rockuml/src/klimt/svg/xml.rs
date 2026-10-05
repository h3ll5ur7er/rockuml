//! The little XML tree PlantUML builds SVG documents with, and its compact serialisation.

#[derive(Clone, Debug, PartialEq)]
pub struct XmlNode {
    tag_name: String,
    /// In insertion order; setting an attribute again keeps its place.
    attributes: Vec<(String, String)>,
    children: Vec<XmlContent>,
}

#[derive(Clone, Debug, PartialEq)]
enum XmlContent {
    Element(XmlNode),
    Text(String),
    ProcessingInstruction { target: String, data: String },
}

impl XmlNode {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_owned(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn set_attribute(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        match self.attributes.iter_mut().find(|(known, _)| known == name) {
            Some((_, old)) => *old = value,
            None => self.attributes.push((name.to_owned(), value)),
        }
    }

    pub fn append_child(&mut self, child: XmlNode) {
        self.children.push(XmlContent::Element(child));
    }

    pub fn set_text_content(&mut self, text: &str) {
        self.children = vec![XmlContent::Text(text.to_owned())];
    }

    pub fn append_processing_instruction(&mut self, target: &str, data: &str) {
        self.children.push(XmlContent::ProcessingInstruction {
            target: target.to_owned(),
            data: data.to_owned(),
        });
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn first_element_mut(&mut self) -> Option<&mut XmlNode> {
        self.children.iter_mut().find_map(|child| match child {
            XmlContent::Element(element) => Some(element),
            _ => None,
        })
    }

    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        self.write_to(&mut out);
        out
    }

    fn write_to(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.tag_name);
        for (name, value) in &self.attributes {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            escape_into(out, value, true);
            out.push('"');
        }
        if self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        for child in &self.children {
            match child {
                XmlContent::Element(element) => element.write_to(out),
                XmlContent::Text(text) => escape_into(out, text, false),
                XmlContent::ProcessingInstruction { target, data } => {
                    out.push_str("<?");
                    out.push_str(target);
                    if !data.is_empty() {
                        out.push(' ');
                        out.push_str(data);
                    }
                    out.push_str("?>");
                }
            }
        }
        out.push_str("</");
        out.push_str(&self.tag_name);
        out.push('>');
    }
}

/// PlantUML escapes only what XML requires: `&`, `<`, and in attributes `"`. Characters XML cannot hold at all
/// (most control characters) are dropped, where PlantUML writes a document no XML reader accepts.
fn escape_into(out: &mut String, text: &str, in_attribute: bool) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' if in_attribute => out.push_str("&quot;"),
            // `]]>` must not appear in text; PlantUML writes it anyway.
            '>' if out.ends_with("]]") => out.push_str("&gt;"),
            c if is_xml_char(c) => out.push(c),
            _ => {}
        }
    }
}

fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_stays_well_formed() {
        let mut text = XmlNode::new("text");
        text.set_text_content("a\u{1}b\u{FFFE}c\td ]]> >");
        assert_eq!(text.to_xml(), "<text>abc\td ]]&gt; ></text>");
    }

    #[test]
    fn serialises_compactly_with_attributes_in_insertion_order() {
        let mut svg = XmlNode::new("svg");
        svg.set_attribute("b", "1");
        svg.set_attribute("a", "x\"<&>");
        svg.set_attribute("b", "2");
        svg.append_processing_instruction("plantuml", "1.2026.8");
        svg.append_child(XmlNode::new("defs"));
        let mut text = XmlNode::new("text");
        text.set_text_content("a < b & \"c\" > d");
        svg.append_child(text);
        assert_eq!(
            svg.to_xml(),
            r#"<svg b="2" a="x&quot;&lt;&amp;>"><?plantuml 1.2026.8?><defs/><text>a &lt; b &amp; "c" > d</text></svg>"#
        );
    }
}
