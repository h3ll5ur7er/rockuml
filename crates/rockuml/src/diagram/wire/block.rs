//! The nested blocks of a wire diagram, the texts printed between them and the spots on them.

use std::cell::Cell;

use super::link::WLinkType;
use crate::color::HColor;
use crate::command::{CommandError, CommandResult};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;

const STARTING_Y: f64 = 10.0;

/// The `NullPointerException` PlantUML throws for a line indented deeper than the blocks go: `method` is called
/// on a last child that does not exist, which the JVM names as `receiver`.
fn no_last_child(method: &str, receiver: &str) -> CommandError {
    CommandError::thrown(&format!(
        "java.lang.NullPointerException: Cannot invoke \"net.sourceforge.plantuml.wire.WBlock.{method}\" \
         because {receiver} is null"
    ))
}

const LAST_CHILD: &str =
    "the return value of \"net.sourceforge.plantuml.wire.WBlock.getLastChild()\"";

/// A block placed below the previous one in its parent, or where its parent's cursor was sent; it is as large
/// as asked, or as its children need, or 100 square (PlantUML's `WBlock`).
pub(super) struct WBlock {
    name: String,
    forced_width: f64,
    forced_height: f64,
    color: Option<HColor>,
    children: Vec<WBlock>,
    position: UTranslate,
    cursor: UTranslate,
    /// The child whose height the cursor still has to pass.
    added_to_cursor: Option<usize>,
    future_out_horizontal: Cell<Option<UTranslate>>,
    future_out_vertical: Cell<Option<UTranslate>>,
    prints: Vec<WPrint>,
}

/// A block found by its dotted name, and where its parent is.
pub(super) struct Located<'a> {
    block: &'a WBlock,
    origin: UTranslate,
}

impl Located<'_> {
    /// Where `x` and `y` are from the block's corner, as pixels, percents of its size, or both
    /// (`50%+10`), on the whole drawing (`getAbsolutePosition`).
    pub(super) fn get_absolute_position(&self, x: &str, y: &str) -> UTranslate {
        let block = self.block;
        let dimension = || block.get_max_dimension();
        UTranslate::new(
            block.position.dx + self.origin.dx + parse_length(x, || dimension().width),
            block.position.dy + self.origin.dy + parse_length(y, || dimension().height),
        )
    }

    /// Where the next link across leaves: at the given point, or on the right side below the previous one
    /// (`getNextOutHorizontal`).
    pub(super) fn get_next_out_horizontal(
        &self,
        at: Option<(&str, &str)>,
        link_type: WLinkType,
    ) -> UTranslate {
        let future = &self.block.future_out_horizontal;
        let result = self.next_out(at, future, ("100%", "5"));
        future.set(Some(
            result.compose(UTranslate::new(0.0, link_type.space_for_next())),
        ));
        result
    }

    /// Where the next link down leaves: at the given point, or on the bottom side right of the previous one
    /// (`getNextOutVertical`).
    pub(super) fn get_next_out_vertical(
        &self,
        at: Option<(&str, &str)>,
        link_type: WLinkType,
    ) -> UTranslate {
        let future = &self.block.future_out_vertical;
        let result = self.next_out(at, future, ("5", "100%"));
        future.set(Some(
            result.compose(UTranslate::new(link_type.space_for_next(), 0.0)),
        ));
        result
    }

    fn next_out(
        &self,
        at: Option<(&str, &str)>,
        future: &Cell<Option<UTranslate>>,
        first: (&str, &str),
    ) -> UTranslate {
        match (at, future.get()) {
            (Some((x, y)), _) => self.get_absolute_position(x, y),
            (None, None) => self.get_absolute_position(first.0, first.1),
            (None, Some(future)) => future,
        }
    }
}

/// `value` in pixels, as a percent of `full` (`50%`), or both (`50%-10`) (`parseWidth`, `parseHeight`).
fn parse_length(value: &str, full: impl Fn() -> f64) -> f64 {
    let number = |text: &str| text.parse::<f64>().expect("the pattern reads numbers");
    if let Some(percent) = value.strip_suffix('%') {
        return full() * (number(percent) / 100.0);
    }
    if let Some((percent, pixels)) = value.split_once('%') {
        return full() * (number(percent) / 100.0) + number(pixels);
    }
    number(value)
}

impl WBlock {
    /// The diagram itself, which draws no frame.
    pub(super) fn root() -> Self {
        Self::new(String::new(), UTranslate::default(), 0.0, 0.0, None)
    }

    fn new(
        name: String,
        position: UTranslate,
        width: f64,
        height: f64,
        color: Option<HColor>,
    ) -> Self {
        Self {
            name,
            forced_width: width,
            forced_height: height,
            color,
            children: Vec::new(),
            position,
            cursor: UTranslate::new(10.0, STARTING_Y),
            added_to_cursor: None,
            future_out_horizontal: Cell::new(None),
            future_out_vertical: Cell::new(None),
            prints: Vec::new(),
        }
    }

    fn get_child_by_name(&self, name: &str) -> Option<&WBlock> {
        self.children.iter().find(|child| child.name == name)
    }

    /// The block named by the path of names `a.b.c` from this one (`getBlock`).
    pub(super) fn get_block(&self, name: &str) -> Option<Located<'_>> {
        self.get_block_from(UTranslate::default(), name)
    }

    fn get_block_from(&self, origin: UTranslate, name: &str) -> Option<Located<'_>> {
        match name.split_once('.') {
            None => self
                .get_child_by_name(name)
                .map(|block| Located { block, origin }),
            Some((first, rest)) => {
                let child = self.get_child_by_name(first)?;
                child.get_block_from(child.position.compose(origin), rest)
            }
        }
    }

    fn get_last_child(
        &mut self,
        method: &str,
        receiver: &str,
    ) -> Result<&mut WBlock, CommandError> {
        self.children
            .last_mut()
            .ok_or_else(|| no_last_child(method, receiver))
    }

    /// Sends the cursor right of what the block holds, to the top (`newColumn`).
    pub(super) fn new_column(&mut self, level: usize) -> CommandResult {
        if level == 0 {
            let max = self.get_natural_dimension();
            self.cursor = UTranslate::new(max.width + 10.0, STARTING_Y);
            self.added_to_cursor = None;
            return Ok(());
        }
        self.get_last_child("newColumn(int)", LAST_CHILD)?
            .new_column(level - 1)
    }

    pub(super) fn wgoto(&mut self, level: usize, x: f64, y: f64) -> CommandResult {
        if level == 0 {
            self.cursor = UTranslate::new(x, y);
            self.added_to_cursor = None;
            return Ok(());
        }
        self.get_last_child("wgoto(int, double, double)", LAST_CHILD)?
            .wgoto(level - 1, x, y)
    }

    pub(super) fn wmove(&mut self, level: usize, x: f64, y: f64) -> CommandResult {
        if level == 0 {
            self.cursor = self.cursor.compose(UTranslate::new(x, y));
            return Ok(());
        }
        self.get_last_child("wmove(int, double, double)", LAST_CHILD)?
            .wmove(level - 1, x, y)
    }

    /// Prints the text at the cursor, measured as PlantUML measures it for PNG whatever the output, and moves
    /// the cursor below it.
    pub(super) fn print(
        &mut self,
        string_bounder: &dyn StringBounder,
        skin: &SkinParam,
        level: usize,
        text: Display,
    ) -> CommandResult {
        if level == 0 {
            let print = WPrint {
                position: self.get_next_position(),
                label: text,
            };
            self.cursor = self
                .cursor
                .compose(UTranslate::new(0.0, print.get_height(string_bounder, skin)));
            self.prints.push(print);
            return Ok(());
        }
        self.get_last_child(
            "print(net.sourceforge.plantuml.klimt.font.StringBounder, \
             net.sourceforge.plantuml.style.ISkinParam, int, String)",
            LAST_CHILD,
        )?
        .print(string_bounder, skin, level - 1, text)
    }

    /// Adds a block below the cursor. Its name must be new at every level down to its parent (`addBlock`).
    pub(super) fn add_block(
        &mut self,
        level: usize,
        name: &str,
        width: f64,
        height: f64,
        color: Option<HColor>,
    ) -> CommandResult {
        if self.get_child_by_name(name).is_some() {
            return Err(CommandError::new("Component exists already"));
        }
        if level == 0 {
            self.cursor = self.cursor.compose(UTranslate::new(0.0, 10.0));
            let new_block = WBlock::new(
                name.to_owned(),
                self.get_next_position(),
                width,
                height,
                color,
            );
            self.cursor = self.cursor.compose(UTranslate::new(0.0, 10.0));
            self.added_to_cursor = Some(self.children.len());
            self.children.push(new_block);
            return Ok(());
        }
        self.get_last_child(
            "addBlock(int, String, double, double, net.sourceforge.plantuml.klimt.color.HColor)",
            "\"<local8>\"",
        )?
        .add_block(level - 1, name, width, height, color)
    }

    fn get_next_position(&mut self) -> UTranslate {
        if let Some(added) = self.added_to_cursor.take() {
            let dim = self.children[added].get_max_dimension();
            self.cursor = self.cursor.compose(UTranslate::new(0.0, dim.height));
        }
        self.cursor
    }

    pub(super) fn draw_me(&self, ug: &UGraphic, skin: &SkinParam) {
        self.draw_box(ug, skin);
        let font = FontConfiguration::new(
            UFont::new("SansSerif", UFontFace::NORMAL, 12),
            HColor::BLACK,
            8,
        );
        Display::create([self.name.replace('_', " ")])
            .create0(
                &font,
                HorizontalAlignment::Left,
                &SpriteContainerEmpty,
                0.0,
                CreoleMode::Full,
            )
            .draw_u(&ug.translated(5.0, 0.0));
    }

    fn draw_box(&self, ug: &UGraphic, skin: &SkinParam) {
        let ug = ug.with_color(HColor::BLACK);
        if !self.name.is_empty() {
            let dim = self.get_max_dimension();
            let rect = UShape::Rectangle(URectangle::new(dim.width, dim.height));
            match &self.color {
                Some(color) => ug.with_backcolor(color.clone()).draw(&rect),
                None => ug.draw(&rect),
            }
        }
        for child in &self.children {
            child.draw_me(&ug.apply(child.position), skin);
        }
        for print in &self.prints {
            print.draw_me(&ug.apply(print.position), skin);
        }
    }

    pub(super) fn get_max_dimension(&self) -> XDimension2D {
        if !self.children.is_empty() {
            if self.forced_width != 0.0 {
                return XDimension2D::new(self.forced_width, self.forced_height);
            }
            return self.get_natural_dimension();
        }
        let x = if self.forced_width == 0.0 {
            100.0
        } else {
            self.forced_width
        };
        let y = if self.forced_height == 0.0 {
            100.0
        } else {
            self.forced_height
        };
        XDimension2D::new(x, y)
    }

    fn get_natural_dimension(&self) -> XDimension2D {
        let mut x: f64 = 0.0;
        let mut y: f64 = 0.0;
        for child in &self.children {
            let dim = child.get_max_dimension();
            x = x.max(child.position.dx + dim.width + 10.0);
            y = y.max(child.position.dy + dim.height + 10.0);
        }
        XDimension2D::new(x, y)
    }
}

/// A text printed in a block (PlantUML's `WPrint`).
struct WPrint {
    position: UTranslate,
    label: Display,
}

impl WPrint {
    fn get_text_block(&self, skin: &SkinParam) -> SheetBlock2 {
        let font =
            FontConfiguration::black_blue_true(UFont::new("SansSerif", UFontFace::NORMAL, 10));
        self.label.create0(
            &font,
            HorizontalAlignment::Left,
            skin,
            0.0,
            CreoleMode::Full,
        )
    }

    fn draw_me(&self, ug: &UGraphic, skin: &SkinParam) {
        self.get_text_block(skin).draw_u(ug);
    }

    fn get_height(&self, string_bounder: &dyn StringBounder, skin: &SkinParam) -> f64 {
        self.get_text_block(skin)
            .calculate_dimension(string_bounder)
            .height
    }
}

/// A dot on a block, red unless coloured, at its corner or at a point given as for links (PlantUML's
/// `Spot`).
pub(super) struct Spot {
    pub(super) block: String,
    pub(super) color: HColor,
    pub(super) x: String,
    pub(super) y: String,
}

impl Spot {
    pub(super) fn draw_me(&self, ug: &UGraphic, root: &WBlock) {
        let block = root
            .get_block(&self.block)
            .expect("a spot is only made on a block found");
        let tr = block
            .get_absolute_position(&self.x, &self.y)
            .compose(UTranslate::new(-2.0, -2.0));
        ug.with_color(self.color.clone())
            .with_backcolor(self.color.clone())
            .apply(tr)
            .draw(&UShape::Ellipse(UEllipse::new(5.0, 5.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_pixels_percents_or_both() {
        let unused = || unreachable!("no percent");
        assert_eq!(parse_length("12", unused), 12.0);
        assert_eq!(parse_length("-5", unused), -5.0);
        assert_eq!(parse_length("50%", || 200.0), 100.0);
        assert_eq!(parse_length("100%-10", || 80.0), 70.0);
        assert_eq!(parse_length("25%+3", || 40.0), 13.0);
    }

    #[test]
    fn blocks_stack_below_each_other_and_columns_start_right_of_them() {
        let mut root = WBlock::root();
        root.add_block(0, "a", 0.0, 0.0, None).unwrap();
        root.add_block(0, "b", 40.0, 30.0, None).unwrap();
        root.new_column(0).unwrap();
        root.add_block(0, "c", 0.0, 0.0, None).unwrap();
        let position = |name| {
            root.get_block(name)
                .unwrap()
                .get_absolute_position("0", "0")
        };
        assert_eq!(position("a"), UTranslate::new(10.0, 20.0));
        assert_eq!(position("b"), UTranslate::new(10.0, 140.0));
        assert_eq!(position("c"), UTranslate::new(130.0, 20.0));
        assert_eq!(root.get_max_dimension(), XDimension2D::new(240.0, 180.0));
    }

    #[test]
    fn nested_blocks_are_found_by_path_and_names_are_new_down_the_levels() {
        let mut root = WBlock::root();
        root.add_block(0, "outer", 0.0, 0.0, None).unwrap();
        root.add_block(1, "inner", 0.0, 0.0, None).unwrap();
        let inner = root.get_block("outer.inner").unwrap();
        assert_eq!(
            inner.get_absolute_position("100%", "5"),
            UTranslate::new(120.0, 45.0)
        );
        assert_eq!(
            root.add_block(1, "outer", 0.0, 0.0, None)
                .unwrap_err()
                .message,
            "Component exists already"
        );
        let too_deep = root.add_block(3, "deep", 0.0, 0.0, None).unwrap_err();
        assert!(too_deep.message.contains("WBlock.addBlock"));
    }
}
