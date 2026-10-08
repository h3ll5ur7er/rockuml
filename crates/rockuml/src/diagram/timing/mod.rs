//! Timing diagrams, `@startuml` with `robust`, `concise`, `clock`… players (PlantUML's `timingdiagram`
//! package).

mod commands;
mod panels;
mod player;
mod ruler;
mod time;

use std::rc::Rc;

use panels::{IntricatedPoint, Panels, panels};
use player::{Player, PlayerKind};
use ruler::{TimeAxisStategy, TimingRuler};
use time::{TimeTick, TimingFormat};

use super::builder::CommandFactory;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::{ColorType, Colors, HColor};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, ParserPass};
use crate::creole::{CreoleMode, Display};
use crate::decoration::{LinkDecor, LinkType, WithLinkType};
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::font_param::FontParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// Room left of the players' labels.
const MARGIN_X1: f64 = 5.0;
const MARGIN_X2: f64 = 5.0;

/// The style of `timingDiagram` or one of its elements.
fn timing_style(skin: &SkinParam, names: &[SName]) -> Style {
    let mut signature = vec![SName::Root, SName::Element, SName::TimingDiagram];
    signature.extend_from_slice(names);
    StyleSignature::of(&signature).get_merged_style(&skin.current_style_builder())
}

/// An arrow from a time of a player to a time of another (`TimeMessage`).
struct TimeMessage {
    player1: usize,
    tick1: Option<TimeTick>,
    player2: usize,
    tick2: Option<TimeTick>,
    label: Display,
    /// The style's colour unless the arrow sets its own.
    color: HColor,
}

/// A stretch of time painted behind the players (`Highlight`).
struct Highlight {
    tick_from: TimeTick,
    tick_to: TimeTick,
    caption: Display,
    colors: Colors,
}

pub(super) struct TimingDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    /// The times named with `@... as :code`.
    codes: Vec<(String, TimeTick)>,
    /// By code, in the order they were declared.
    players: Vec<(String, Player)>,
    messages: Vec<TimeMessage>,
    highlights: Vec<Highlight>,
    ruler: TimingRuler,
    now: Option<TimeTick>,
    last_player: Option<usize>,
    time_axis_stategy: TimeAxisStategy,
    compact_by_default: bool,
    /// The `use date format` pattern.
    date_format: Option<String>,
}

/// Reads timing diagrams (PlantUML's `TimingDiagramFactory`).
pub(super) struct TimingDiagramFactory;

impl CommandFactory for TimingDiagramFactory {
    type Diagram = TimingDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Timing;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> TimingDiagram {
        TimingDiagram {
            source: source.clone(),
            titled: Titled::new(SName::TimingDiagram, "TIMING", source),
            codes: Vec::new(),
            players: Vec::new(),
            messages: Vec::new(),
            highlights: Vec::new(),
            ruler: TimingRuler::default(),
            now: None,
            last_player: None,
            time_axis_stategy: TimeAxisStategy::Automatic,
            compact_by_default: false,
            date_format: None,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<TimingDiagram>>> {
        commands::all()
    }
}

impl TimingDiagram {
    fn skin(&self) -> &SkinParam {
        &self.titled.skin
    }

    fn player_index(&self, code: &str) -> Option<usize> {
        self.players
            .iter()
            .position(|(existing, _)| existing == code)
    }

    fn player_mut(&mut self, code: &str) -> Option<&mut Player> {
        self.players
            .iter_mut()
            .find(|(existing, _)| existing == code)
            .map(|(_, player)| player)
    }

    /// Declares a player; one declared again keeps its place among the others.
    fn add_player(&mut self, code: &str, player: Player) -> usize {
        if let Some(index) = self.player_index(code) {
            self.players[index].1 = player;
            return index;
        }
        self.players.push((code.to_owned(), player));
        self.players.len() - 1
    }

    fn create_player_state(
        &mut self,
        code: &str,
        title: &str,
        compact: bool,
        stereotype: Option<crate::stereo::Stereotype>,
        back_color: Option<HColor>,
        style: player::TimingStyle,
    ) {
        let player = Player::new(
            title,
            self.compact_by_default || compact,
            stereotype,
            back_color,
            PlayerKind::State(player::AbstractStatePlayer::new(style)),
        );
        self.last_player = Some(self.add_player(code, player));
    }

    fn create_player_clock(
        &mut self,
        code: &str,
        title: &str,
        clock: player::PlayerClock,
        stereotype: Option<crate::stereo::Stereotype>,
    ) {
        let period = clock.period;
        let player = Player::new(
            title,
            self.compact_by_default,
            stereotype,
            None,
            PlayerKind::Clock(clock),
        );
        self.add_player(code, player);
        self.ruler
            .add_time(TimeTick::new(period, TimingFormat::Decimal));
    }

    /// A binary or analog player, which `mode compact` alone makes compact.
    fn create_player_signal(
        &mut self,
        code: &str,
        title: &str,
        stereotype: Option<crate::stereo::Stereotype>,
        kind: PlayerKind,
    ) {
        let player = Player::new(title, self.compact_by_default, stereotype, None, kind);
        self.add_player(code, player);
    }

    fn add_time(&mut self, time: TimeTick, code: Option<&str>) {
        self.now = Some(time.clone());
        self.ruler.add_time(time.clone());
        if let Some(code) = code {
            match self.codes.iter_mut().find(|(existing, _)| existing == code) {
                Some(entry) => entry.1 = time,
                None => self.codes.push((code.to_owned(), time)),
            }
        }
    }

    fn get_code_value(&self, code: &str) -> Option<&TimeTick> {
        self.codes
            .iter()
            .find(|(existing, _)| existing == code)
            .map(|(_, tick)| tick)
    }

    fn get_clock_value(&self, clock_name: &str, nb: i64) -> Option<TimeTick> {
        let index = self.player_index(clock_name)?;
        match &self.players[index].1.kind {
            PlayerKind::Clock(clock) => Some(TimeTick::new(
                clock.period.multiply(nb),
                TimingFormat::Decimal,
            )),
            _ => None,
        }
    }

    fn get_timing_format_date(&self) -> TimingFormat {
        self.date_format
            .clone()
            .map_or(TimingFormat::Date, TimingFormat::SimpleDate)
    }

    fn use_date_format(&mut self, date_format: &str) -> CommandResult {
        if !time::is_supported_date_format(date_format) {
            return Err(CommandError::new("Bad date format"));
        }
        self.date_format = Some(date_format.to_owned());
        Ok(())
    }

    fn create_time_message(
        &mut self,
        player1: usize,
        tick1: Option<TimeTick>,
        player2: usize,
        tick2: Option<TimeTick>,
        label: Option<&str>,
        arrow_style: Option<&str>,
    ) {
        let style = timing_style(self.skin(), &[SName::Arrow]);
        let color = arrow_style
            .and_then(message_color)
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        self.messages.push(TimeMessage {
            player1,
            tick1,
            player2,
            tick2,
            label: Display::with_newlines(label.unwrap_or_default()),
            color,
        });
    }

    fn highlight(
        &mut self,
        tick_from: TimeTick,
        tick_to: TimeTick,
        caption: Display,
        colors: Colors,
    ) {
        self.highlights.push(Highlight {
            tick_from,
            tick_to,
            caption,
            colors,
        });
    }

    fn panels_of<'a>(&'a self, player: &'a Player) -> Box<dyn Panels + 'a> {
        panels(player, &self.ruler, self.skin())
    }

    fn get_part1_max_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.players.iter().fold(0.0, |width, (_, player)| {
            f64::max(
                width,
                self.panels_of(player).get_left_panel_width(string_bounder),
            )
        })
    }

    fn get_width_total(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_part1_max_width(string_bounder) + self.ruler.get_width() + MARGIN_X1 + MARGIN_X2
    }

    fn player_height(&self, player: &Player, string_bounder: &dyn StringBounder) -> f64 {
        self.panels_of(player).get_full_height(string_bounder)
    }

    /// Where a player's signal starts: below its frame. Without a player, below them all.
    fn get_translate_for_player(
        &self,
        candidate: Option<usize>,
        string_bounder: &dyn StringBounder,
    ) -> f64 {
        let mut y = 0.0;
        for (index, (_, player)) in self.players.iter().enumerate() {
            y += player.get_frame_height(string_bounder, self.skin());
            if candidate == Some(index) {
                return y;
            }
            y += self.player_height(player, string_bounder);
        }
        y
    }

    /// Where a player's frame starts.
    fn get_translate_for_player_frame(
        &self,
        candidate: usize,
        string_bounder: &dyn StringBounder,
    ) -> f64 {
        self.players[..candidate]
            .iter()
            .fold(0.0, |y, (_, player)| {
                y + player.get_frame_height(string_bounder, self.skin())
                    + self.player_height(player, string_bounder)
            })
    }

    fn get_height_inner(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_translate_for_player(None, string_bounder)
    }

    fn get_height_total(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_height_inner(string_bounder) + self.ruler.get_height(string_bounder, self.skin())
    }

    fn line_style(&self) -> Style {
        timing_style(self.skin(), &[])
    }

    fn draw_internal(&self, ug: &UGraphic) {
        self.ruler.ensure_not_empty();
        let string_bounder = ug.string_bounder();
        let part1_max_width = self.get_part1_max_width(string_bounder);
        for (index, (_, player)) in self.players.iter().enumerate() {
            if let Some(background) = &player.general_background_color {
                let full_height = player.get_frame_height(string_bounder, self.skin())
                    + self.player_height(player, string_bounder);
                ug.translated(
                    0.0,
                    self.get_translate_for_player_frame(index, string_bounder),
                )
                .with_color(background.clone())
                .with_backcolor(background.clone())
                .draw(&UShape::Rectangle(URectangle::new(
                    self.get_width_total(string_bounder),
                    full_height,
                )));
            }
        }
        if !self.compact_by_default {
            self.draw_border(ug);
        }
        let ug = ug.translated(MARGIN_X1, 0.0);
        let ug_part2 = ug.translated(part1_max_width, 0.0);
        let height_inner = self.get_height_inner(string_bounder);
        for highlight in &self.highlights {
            self.draw_highlight_back(&ug_part2, highlight, height_inner);
        }
        self.ruler.draw_vlines(&ug_part2, height_inner, self.skin());
        for (index, (_, player)) in self.players.iter().enumerate() {
            let ug_player = ug.translated(
                0.0,
                self.get_translate_for_player(Some(index), string_bounder),
            );
            let ug_frame = ug.translated(
                0.0,
                self.get_translate_for_player_frame(index, string_bounder),
            );
            if !player.compact {
                self.draw_horizontal_separator(&ug_frame);
            }
            player.draw_frame_title(&ug_frame, self.skin());
            let panels = self.panels_of(player);
            panels.draw_left_panel(&ug_player, part1_max_width);
            panels.draw_right_panel(&ug_player.translated(part1_max_width, 0.0));
        }
        self.ruler.draw_time_axis(
            &ug_part2.translated(0.0, height_inner),
            self.time_axis_stategy,
            &self.codes,
            self.skin(),
        );
        for message in &self.messages {
            self.draw_message(&ug_part2, message);
        }
        for highlight in &self.highlights {
            self.draw_highlight_lines(&ug_part2, highlight, height_inner);
        }
    }

    fn draw_horizontal_separator(&self, ug: &UGraphic) {
        let style = self.line_style();
        ug.with_color(style.value(PName::LineColor).as_color())
            .with_stroke(style.stroke())
            .translated(-MARGIN_X1, 0.0)
            .draw(&UShape::Line {
                dx: self.get_width_total(ug.string_bounder()),
                dy: 0.0,
            });
    }

    fn draw_border(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let border = UShape::Line {
            dx: 0.0,
            dy: self.get_height_inner(string_bounder),
        };
        let style = self.line_style();
        let ug = ug
            .with_color(style.value(PName::LineColor).as_color())
            .with_stroke(style.stroke());
        ug.draw(&border);
        ug.translated(self.get_width_total(string_bounder), 0.0)
            .draw(&border);
    }

    fn highlight_style(&self) -> Style {
        timing_style(self.skin(), &[SName::Highlight])
    }

    fn draw_highlight_back(&self, ug: &UGraphic, highlight: &Highlight, height: f64) {
        let back = highlight
            .colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| {
                self.highlight_style()
                    .value(PName::BackGroundColor)
                    .as_color()
            });
        let start = self.ruler.get_pos_in_pixel(&highlight.tick_from);
        let end = self.ruler.get_pos_in_pixel(&highlight.tick_to);
        ug.with_color(HColor::NONE)
            .with_backcolor(back)
            .translated(start, 0.0)
            .draw(&UShape::Rectangle(URectangle::new(end - start, height)));
    }

    fn draw_highlight_lines(&self, ug: &UGraphic, highlight: &Highlight, height: f64) {
        let style = self.highlight_style();
        let line_color = highlight
            .colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let lines = ug.with_stroke(style.stroke()).with_color(line_color);
        let start = self.ruler.get_pos_in_pixel(&highlight.tick_from);
        let end = self.ruler.get_pos_in_pixel(&highlight.tick_to);
        let line = UShape::Line {
            dx: 0.0,
            dy: height,
        };
        lines.translated(start, 0.0).draw(&line);
        lines.translated(end, 0.0).draw(&line);
        let caption = highlight.caption.create0(
            &self.skin().get_font_configuration(FontParam::Timing, None),
            HorizontalAlignment::Left,
            self.skin(),
            0.0,
            CreoleMode::Full,
        );
        caption.draw_u(&ug.translated(start + 3.0, 2.0));
    }

    fn draw_message(&self, ug: &UGraphic, message: &TimeMessage) {
        let string_bounder = ug.string_bounder();
        let projection = |player: usize, tick: &Option<TimeTick>| {
            let tick = tick.as_ref()?;
            let point = self
                .panels_of(&self.players[player].1)
                .get_time_projection(string_bounder, tick)?;
            Some(point.translated(self.get_translate_for_player(Some(player), string_bounder)))
        };
        let (Some(pt1), Some(pt2)) = (
            projection(message.player1, &message.tick1),
            projection(message.player2, &message.tick2),
        ) else {
            return;
        };
        TimeArrow::create(pt1, pt2).draw_u(ug, message, self.skin());
    }
}

/// What a message's arrow style changes; only the colour of its line shows (`WithLinkType`).
struct MessageStyle {
    link_type: LinkType,
    color: Option<HColor>,
}

impl WithLinkType for MessageStyle {
    fn link_type_mut(&mut self) -> &mut LinkType {
        &mut self.link_type
    }

    fn set_specific_color(&mut self, color: HColor, i: usize) {
        if i == 0 {
            self.color = Some(color);
        }
    }
}

/// The colour an arrow's `[...]` style gives a message.
fn message_color(arrow_style: &str) -> Option<HColor> {
    let mut style = MessageStyle {
        link_type: LinkType::new(LinkDecor::None, LinkDecor::None),
        color: None,
    };
    style.apply_style(Some(arrow_style));
    style.color
}

/// The colour an arrow's `[...]` style gives a constraint: its last colour (`CommandArrow.applyStyle`).
fn constraint_color(arrow_style: Option<&str>) -> Result<Option<HColor>, CommandError> {
    let mut color = None;
    for part in arrow_style
        .unwrap_or_default()
        .split(',')
        .filter(|part| !part.is_empty())
    {
        if !["dashed", "dotted", "bold", "hidden"]
            .iter()
            .any(|keyword| part.eq_ignore_ascii_case(keyword))
        {
            color = Some(
                HColor::parse(part)
                    .ok()
                    .flatten()
                    .ok_or_else(CommandError::bad_color)?,
            );
        }
    }
    Ok(color)
}

/// The shortest straight arrow between two intricated points (`TimeArrow`).
struct TimeArrow {
    start: XPoint2D,
    end: XPoint2D,
}

impl TimeArrow {
    fn create(pt1: IntricatedPoint, pt2: IntricatedPoint) -> Self {
        let shorter = |arrow1: Self, arrow2: Self| {
            if arrow1.len() < arrow2.len() {
                arrow1
            } else {
                arrow2
            }
        };
        let arrow = |start, end| Self { start, end };
        shorter(
            shorter(arrow(pt1.a, pt2.a), arrow(pt1.a, pt2.b)),
            shorter(arrow(pt1.b, pt2.a), arrow(pt1.b, pt2.b)),
        )
    }

    fn len(&self) -> f64 {
        self.start.distance(self.end)
    }

    fn on_circle(point: XPoint2D, alpha: f64) -> XPoint2D {
        const RADIUS: f64 = 8.0;
        XPoint2D::new(
            point.x - libm::sin(alpha) * RADIUS,
            point.y - libm::cos(alpha) * RADIUS,
        )
    }

    fn draw_u(&self, ug: &UGraphic, message: &TimeMessage, skin: &SkinParam) {
        let style = timing_style(skin, &[SName::Arrow]);
        let angle = libm::atan2(self.end.x - self.start.x, self.end.y - self.start.y);
        let ug = ug
            .with_color(message.color.clone())
            .with_stroke(style.stroke());
        ug.translated(self.start.x, self.start.y)
            .draw(&UShape::Line {
                dx: self.end.x - self.start.x,
                dy: self.end.y - self.start.y,
            });
        let delta = 20.0 * std::f64::consts::PI / 180.0;
        let pt1 = Self::on_circle(self.end, angle + delta);
        let pt2 = Self::on_circle(self.end, angle - delta);
        let ug = ug.with_backcolor(message.color.clone());
        ug.draw(&UShape::polygon(vec![
            (pt1.x, pt1.y),
            (pt2.x, pt2.y),
            (self.end.x, self.end.y),
        ]));
        let label = message.label.create0(
            &style.font_configuration(),
            HorizontalAlignment::Left,
            skin,
            0.0,
            CreoleMode::Full,
        );
        let x_text = f64::midpoint(pt1.x, pt2.x);
        let mut y_text = f64::midpoint(pt1.y, pt2.y);
        if self.start.y < self.end.y {
            y_text -= label.calculate_dimension(ug.string_bounder()).height;
        }
        label.draw_u(&ug.translated(x_text, y_text));
    }
}

/// The players over the time grid, measured as PlantUML measures it.
struct Drawing<'a>(&'a TimingDiagram);

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.0.get_width_total(string_bounder),
            self.0.get_height_total(string_bounder),
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.0.draw_internal(ug);
    }
}

impl AbstractDiagram for TimingDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for TimingDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for TimingDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let drawing = TextBlockMarged::new(Drawing(self), ClockwiseTopRightBottomLeft::same(10.0));
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{constraint_color, message_color};
    use crate::color::HColor;

    #[test]
    fn arrow_styles_give_colours() {
        assert_eq!(message_color("#red,dashed"), HColor::parse("red").unwrap());
        assert_eq!(message_color("bold"), None);
        assert_eq!(
            constraint_color(Some("dotted,#blue")).unwrap(),
            HColor::parse("blue").unwrap()
        );
        assert!(constraint_color(Some("nocolor")).is_err());
    }
}
