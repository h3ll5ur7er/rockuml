//! A participant's column: its head and tail, its lifeline, and the activation boxes on it (PlantUML's
//! `LivingSpace`, `LivingSpaces`, `MutingLine`, `LiveBoxes` and `LiveBoxesDrawer`).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use super::components;
use super::key::Key;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{Event, EventId, LiveColors, ParticipantId};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, Component, Context2D};
use crate::skin::rose::life::ComponentRoseActiveLine;
use crate::style::StyleBuilder;

/// How future activations count when asking for a lifeline's activation level at an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EventsHistoryMode {
    IgnoreFutureDeactivate,
    IgnoreFutureActivate,
    ConsiderFutureDeactivate,
}

/// How participant boxes align: heads at the top of the diagram sit on their bottoms, tails at the bottom
/// hang from their tops.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum VerticalAlignment {
    Top,
    Bottom,
}

pub(super) struct LivingSpace<'a> {
    diagram: &'a SequenceDiagram,
    pub participant: ParticipantId,
    pub englober: Option<usize>,
    pos_b: Real,
    pos_c: OnceCell<Real>,
    pos_d: OnceCell<Real>,
    create: Cell<bool>,
    /// Where the lifeline starts (`true`) or stops (`false`) being drawn.
    alive_changes: RefCell<BTreeMap<Key, bool>>,
    margin_before: Rc<Cell<f64>>,
    margin_after: Rc<Cell<f64>>,
    /// Delays, by start, with their heights: the lifeline is dotted there.
    delays: RefCell<BTreeMap<Key, f64>>,
    live_boxes: LiveBoxes<'a>,
}

impl<'a> LivingSpace<'a> {
    pub(super) fn new(
        diagram: &'a SequenceDiagram,
        participant: ParticipantId,
        englober: Option<usize>,
        position: Real,
    ) -> Self {
        Self {
            diagram,
            participant,
            englober,
            pos_b: position,
            pos_c: OnceCell::new(),
            pos_d: OnceCell::new(),
            create: Cell::new(false),
            alive_changes: RefCell::new(BTreeMap::new()),
            margin_before: Rc::new(Cell::new(0.0)),
            margin_after: Rc::new(Cell::new(0.0)),
            delays: RefCell::new(BTreeMap::new()),
            live_boxes: LiveBoxes::new(diagram, participant),
        }
    }

    pub(super) fn level_at(&self, event: EventId, mode: EventsHistoryMode) -> usize {
        self.live_boxes.level_at(event, mode)
    }

    pub(super) fn add_step_for_livebox(&self, event: EventId, y: f64) {
        self.live_boxes.add_step(event, y);
    }

    fn head(&self, top: bool) -> Box<dyn Component> {
        components::participant_component(self.diagram, self.participant, top)
    }

    pub(super) fn head_preferred_dimension(
        &self,
        string_bounder: &dyn StringBounder,
    ) -> XDimension2D {
        self.head(true).preferred_dimension(string_bounder)
    }

    fn preferred_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.head_preferred_dimension(string_bounder).width
    }

    pub(super) fn pos_b(&self) -> &Real {
        &self.pos_b
    }

    pub(super) fn pos_c(&self, string_bounder: &dyn StringBounder) -> Real {
        self.pos_c
            .get_or_init(|| {
                self.pos_b
                    .add_fixed(self.preferred_width(string_bounder) / 2.0)
            })
            .clone()
    }

    /// The centre, after the widest nesting of activation boxes.
    pub(super) fn pos_c2(&self, string_bounder: &dyn StringBounder) -> Real {
        self.pos_c(string_bounder)
            .add_fixed(self.live_boxes.max_position(string_bounder))
    }

    pub(super) fn pos_d(&self, string_bounder: &dyn StringBounder) -> Real {
        self.pos_d
            .get_or_init(|| self.pos_b.add_fixed(self.preferred_width(string_bounder)))
            .clone()
    }

    /// The left edge, before whatever sticks out to the left, as known now.
    pub(super) fn pos_a(&self) -> Real {
        self.pos_b.add_fixed(-self.margin_before.get())
    }

    pub(super) fn pos_e(&self, string_bounder: &dyn StringBounder) -> Real {
        self.pos_d(string_bounder)
            .add_fixed(self.margin_after.get())
    }

    /// Like [`Self::pos_a`], with the margin read when solving, after it has grown.
    pub(super) fn pos_a_live(&self) -> Real {
        let margin = self.margin_before.clone();
        self.pos_b.with_live_offset(Rc::new(move || -margin.get()))
    }

    pub(super) fn pos_e_live(&self, string_bounder: &dyn StringBounder) -> Real {
        let margin = self.margin_after.clone();
        self.pos_d(string_bounder)
            .with_live_offset(Rc::new(move || margin.get()))
    }

    pub(super) fn ensure_margin_before(&self, margin: f64) {
        self.margin_before.set(self.margin_before.get().max(margin));
    }

    pub(super) fn ensure_margin_after(&self, margin: f64) {
        self.margin_after.set(self.margin_after.get().max(margin));
    }

    pub(super) fn go_create(&self) {
        self.create.set(true);
    }

    pub(super) fn go_create_at(&self, y: f64) {
        self.alive_changes.borrow_mut().insert(Key(y), true);
        self.create.set(true);
    }

    pub(super) fn go_destroy(&self, y: f64) {
        self.alive_changes.borrow_mut().insert(Key(y), false);
    }

    pub(super) fn delay_on(&self, y: f64, height: f64) {
        self.delays.borrow_mut().insert(Key(y), height);
        self.live_boxes.delay_on(y, height);
    }

    /// The head a creation message ends on, left of `ug` when `right_aligned`.
    pub(super) fn draw_created_head(&self, ug: &UGraphic, context: Context2D, right_aligned: bool) {
        self.draw_head_or_tail(ug, context, VerticalAlignment::Top, right_aligned, true);
    }

    /// Created participants have their head where they are created, not at the top.
    fn draw_head_or_tail(
        &self,
        ug: &UGraphic,
        context: Context2D,
        alignment: VerticalAlignment,
        right_aligned: bool,
        head: bool,
    ) {
        if self.create.get() && alignment == VerticalAlignment::Bottom {
            return;
        }
        let component = self.head(head);
        let dimension = component.preferred_dimension(ug.string_bounder());
        let ug = if right_aligned {
            ug.translated(-dimension.width, 0.0)
        } else {
            ug.clone()
        };
        let url = self.diagram.participant(self.participant).url.as_ref();
        if let Some(url) = url {
            ug.start_url(url);
        }
        component.draw_u(&ug, &Area::new(dimension.width, dimension.height), context);
        if url.is_some() {
            ug.close_url();
        }
    }

    pub(super) fn draw_line_and_liveboxes(&self, ug: &UGraphic, height: f64, context: Context2D) {
        let mut alive = !self.create.get();
        let mut alive_since = 0.0;
        for (&Key(y), &now_alive) in self.alive_changes.borrow().iter() {
            if !alive && now_alive {
                alive_since = y;
            } else if alive && !now_alive {
                self.draw_line(ug, context, alive_since, y);
            }
            alive = now_alive;
        }
        if alive {
            self.draw_line(ug, context, alive_since, height);
        }
        self.live_boxes
            .draw_boxes(ug, context, self.first_create_y(), height);
    }

    fn first_create_y(&self) -> f64 {
        self.alive_changes
            .borrow()
            .iter()
            .find(|(_, alive)| **alive)
            .map_or(0.0, |(&Key(y), _)| y)
    }

    /// `MutingLine.drawLine`: dotted where delays are.
    fn draw_line(&self, ug: &UGraphic, context: Context2D, create_y: f64, end_y: f64) {
        let delays = self.delays.borrow();
        if delays.is_empty() {
            self.draw_line_part(ug, context, create_y, end_y, false);
            return;
        }
        let mut y = create_y;
        for (&Key(start), &height) in delays.iter() {
            if start >= create_y && start + height <= end_y {
                self.draw_line_part(ug, context, y, start, false);
                self.draw_line_part(ug, context, start, start + height, true);
                y = start + height;
            }
        }
        self.draw_line_part(ug, context, y, end_y, false);
    }

    fn draw_line_part(&self, ug: &UGraphic, context: Context2D, y1: f64, y2: f64, delay: bool) {
        if y2 == y1 {
            return;
        }
        assert!(y2 > y1, "lifelines go down");
        let component = if delay {
            components::delay_line(self.diagram, self.participant)
        } else {
            components::lifeline(self.diagram, self.participant)
        };
        let width = component.preferred_width(ug.string_bounder());
        component.draw_u(&ug.translated(0.0, y1), &Area::new(width, y2 - y1), context);
    }
}

/// The columns of all participants, in display order.
pub(super) struct LivingSpaces<'a> {
    all: Vec<LivingSpace<'a>>,
    index: HashMap<ParticipantId, usize>,
}

impl<'a> LivingSpaces<'a> {
    pub(super) fn new() -> Self {
        Self {
            all: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub(super) fn push(&mut self, living_space: LivingSpace<'a>) {
        self.index.insert(living_space.participant, self.all.len());
        self.all.push(living_space);
    }

    pub(super) fn values(&self) -> &[LivingSpace<'a>] {
        &self.all
    }

    pub(super) fn get(&self, participant: ParticipantId) -> &LivingSpace<'a> {
        &self.all[self.index[&participant]]
    }

    pub(super) fn index_of(&self, participant: ParticipantId) -> usize {
        self.index[&participant]
    }

    pub(super) fn previous(&self, participant: ParticipantId) -> Option<&LivingSpace<'a>> {
        self.index_of(participant)
            .checked_sub(1)
            .map(|index| &self.all[index])
    }

    pub(super) fn next(&self, participant: ParticipantId) -> Option<&LivingSpace<'a>> {
        self.all.get(self.index_of(participant) + 1)
    }

    pub(super) fn first(&self) -> &LivingSpace<'a> {
        &self.all[0]
    }

    pub(super) fn last(&self) -> &LivingSpace<'a> {
        self.all
            .last()
            .expect("a sequence diagram has participants")
    }

    pub(super) fn add_constraints(&self, string_bounder: &dyn StringBounder) {
        for pair in self.all.windows(2) {
            let point1 = pair[0].pos_e(string_bounder);
            let point2 = pair[1].pos_a();
            point2.ensure_bigger_than(&point1.add_fixed(10.0));
        }
    }

    pub(super) fn head_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.all
            .iter()
            .map(|living_space| living_space.head_preferred_dimension(string_bounder).height)
            .fold(0.0, f64::max)
    }

    /// Heads at the top, aligned on their bottoms (`Bottom`), or tails at the bottom (`Top`).
    pub(super) fn draw_heads(
        &self,
        ug: &UGraphic,
        context: Context2D,
        alignment: VerticalAlignment,
    ) {
        let string_bounder = ug.string_bounder();
        let head_height = self.head_height(string_bounder);
        for living_space in &self.all {
            let x = living_space.pos_b.current_value();
            let y = match alignment {
                VerticalAlignment::Bottom => {
                    head_height - living_space.head_preferred_dimension(string_bounder).height
                }
                VerticalAlignment::Top => 0.0,
            };
            living_space.draw_head_or_tail(
                &ug.translated(x, y),
                context,
                alignment,
                false,
                alignment == VerticalAlignment::Bottom,
            );
        }
    }

    pub(super) fn draw_life_lines(&self, ug: &UGraphic, height: f64, context: Context2D) {
        for living_space in &self.all {
            let x = living_space.pos_c(ug.string_bounder()).current_value();
            living_space.draw_line_and_liveboxes(&ug.translated(x, 0.0), height, context);
        }
    }

    pub(super) fn delay_on(&self, y: f64, height: f64) {
        for living_space in &self.all {
            living_space.delay_on(y, height);
        }
    }
}

/// One stair of a lifeline's activation levels, from `value` down (PlantUML's `Step`).
struct Step {
    value: f64,
    destroy: bool,
    indent: usize,
    colors: Option<LiveColors>,
    style_builder: Option<Rc<StyleBuilder>>,
}

/// The activation levels along a lifeline, read from the diagram's events (PlantUML's `LiveBoxes`).
struct LiveBoxes<'a> {
    diagram: &'a SequenceDiagram,
    participant: ParticipantId,
    /// The y each event concerning the participant was placed at.
    events_step: RefCell<HashMap<EventId, f64>>,
    delays: RefCell<BTreeMap<Key, f64>>,
    level_cache: OnceCell<LevelCache>,
}

/// Each event's activation level, and for each event the next life event reachable through messages and
/// notes.
struct LevelCache {
    level: Vec<usize>,
    next_life: Vec<Option<usize>>,
}

impl<'a> LiveBoxes<'a> {
    fn new(diagram: &'a SequenceDiagram, participant: ParticipantId) -> Self {
        Self {
            diagram,
            participant,
            events_step: RefCell::new(HashMap::new()),
            delays: RefCell::new(BTreeMap::new()),
            level_cache: OnceCell::new(),
        }
    }

    fn events(&self) -> &'a [Event] {
        self.diagram.events()
    }

    /// A deactivation at the same height as another event goes 5 lower.
    fn add_step(&self, event: EventId, y: f64) {
        if !self.events()[event].deals_with(self.participant) {
            return;
        }
        let mut steps = self.events_step.borrow_mut();
        let deactivate =
            matches!(&self.events()[event], Event::LifeEvent(life) if life.is_deactivate());
        let y = if deactivate && steps.values().any(|&other| other == y) {
            y + 5.0
        } else {
            y
        };
        steps.insert(event, y);
    }

    fn cache(&self) -> &LevelCache {
        self.level_cache.get_or_init(|| {
            let events = self.events();
            let mut level: usize = 0;
            let mut levels = Vec::with_capacity(events.len());
            for event in events {
                if let Event::LifeEvent(life) = event
                    && life.participant == self.participant
                {
                    if life.is_activate() {
                        level += 1;
                    }
                    if life.is_deactivate_or_destroy() {
                        level = level.saturating_sub(1);
                    }
                }
                levels.push(level);
            }
            let mut next_life = vec![None; events.len() + 1];
            for index in (0..events.len()).rev() {
                next_life[index] = match &events[index] {
                    Event::LifeEvent(_) => Some(index),
                    Event::Note(_) | Event::Message(_) | Event::MessageExo(_) => {
                        next_life[index + 1]
                    }
                    _ => None,
                };
            }
            LevelCache {
                level: levels,
                next_life,
            }
        })
    }

    fn level_at(&self, event: EventId, mode: EventsHistoryMode) -> usize {
        let cache = self.cache();
        let events = self.events();
        let mut level = cache.level[event] as i64;
        let participant = self.participant;
        match &events[event] {
            current if current.message_common().is_some() => {
                let mut seen_activate = false;
                let mut seen_deactivate = false;
                let mut next = cache.next_life[event + 1];
                while let Some(index) = next {
                    next = cache.next_life[index + 1];
                    let Event::LifeEvent(life) = &events[index] else {
                        unreachable!("the chain only holds life events")
                    };
                    let same_message = life
                        .message
                        .is_some_and(|message| self.diagram.is_parallel_with(message, event));
                    if !same_message {
                        continue;
                    }
                    let concerned =
                        current.deals_with(participant) && life.participant == participant;
                    if mode != EventsHistoryMode::IgnoreFutureActivate
                        && life.is_activate()
                        && concerned
                    {
                        seen_activate = true;
                        if seen_deactivate {
                            break;
                        }
                        level += 1;
                    }
                    if mode == EventsHistoryMode::ConsiderFutureDeactivate
                        && life.is_deactivate_or_destroy()
                        && concerned
                    {
                        seen_deactivate = true;
                        if seen_activate {
                            break;
                        }
                        level = (level - 1).max(0);
                    }
                }
            }
            Event::Note(_) => {
                for next in events[event + 1..]
                    .iter()
                    .filter(|next| !matches!(next, Event::Note(_)))
                {
                    let Event::LifeEvent(life) = next else {
                        break;
                    };
                    if life.participant != participant || life.message.is_none() {
                        continue;
                    }
                    if mode != EventsHistoryMode::IgnoreFutureActivate && life.is_activate() {
                        level += 1;
                    }
                    if mode == EventsHistoryMode::ConsiderFutureDeactivate
                        && life.is_deactivate_or_destroy()
                    {
                        level = (level - 1).max(0);
                    }
                }
            }
            _ => {}
        }
        level.max(0) as usize
    }

    /// Whether the life events right after a message destroy the participant.
    fn is_next_event_a_destroy(&self, event: EventId) -> bool {
        if !matches!(self.events()[event], Event::Message(_)) {
            return false;
        }
        for next in &self.events()[event + 1..] {
            match next {
                Event::Note(_) => {}
                Event::LifeEvent(life) => {
                    if life.participant == self.participant
                        && life.kind == crate::diagram::sequence::model::LifeEventType::Destroy
                    {
                        return true;
                    }
                }
                _ => return false,
            }
        }
        false
    }

    fn activate_color(&self, event: EventId) -> Option<LiveColors> {
        let events = self.events();
        if let Event::LifeEvent(life) = &events[event]
            && life.is_activate()
        {
            return Some(life.colors.clone());
        }
        events[event].message_common()?;
        for next in &events[event + 1..] {
            match next {
                Event::Note(_) => {}
                Event::LifeEvent(life) if life.message == Some(event) => {
                    if life.is_activate() && life.participant == self.participant {
                        return Some(life.colors.clone());
                    }
                }
                _ => return None,
            }
        }
        None
    }

    fn stairs(&self, create_y: f64, total_height: f64) -> Vec<Step> {
        let mut stairs: Vec<Step> = Vec::new();
        let mut add = |step: Step| {
            if stairs.last().is_none_or(|last| step.value >= last.value) {
                stairs.push(step);
            }
        };
        let steps = self.events_step.borrow();
        let mut indent = 0;
        let mut last_message: Option<EventId> = None;
        let mut position: Option<f64> = None;
        let mut seen_activate = false;
        let mut seen_deactivate = false;
        for (id, event) in self.events().iter().enumerate() {
            if matches!(event, Event::Note(_)) {
                last_message = None;
                seen_activate = false;
                seen_deactivate = false;
            }
            let potential = steps.get(&id).copied();
            match (position, last_message, event) {
                (None, _, _) | (_, None, _) => position = potential,
                (_, Some(last), Event::LifeEvent(life)) => {
                    if !event.deals_with(self.participant) {
                        continue;
                    }
                    if life
                        .message
                        .is_none_or(|message| !self.diagram.is_parallel_with(message, last))
                        || (life.is_activate() && seen_deactivate)
                        || (life.is_deactivate() && seen_activate)
                    {
                        position = potential;
                    }
                    seen_activate |= life.is_activate();
                    seen_deactivate |= life.is_deactivate();
                }
                _ => position = potential,
            }
            if let Some(common) = event.message_common() {
                let _ = common;
                if last_message.is_some_and(|last| self.diagram.is_parallel_with(id, last)) {
                    continue;
                }
                seen_activate = false;
                seen_deactivate = false;
                last_message = Some(id);
            }
            if let Some(position) = position {
                indent = self.level_at(id, EventsHistoryMode::ConsiderFutureDeactivate);
                let style_builder = match event {
                    Event::LifeEvent(life) => Some(life.style_builder.clone()),
                    other => other
                        .message_common()
                        .map(|common| common.style_builder.clone()),
                };
                add(Step {
                    value: create_y.max(position),
                    destroy: self.is_next_event_a_destroy(id),
                    indent,
                    colors: self.activate_color(id),
                    style_builder,
                });
            }
        }
        add(Step {
            value: total_height,
            destroy: false,
            indent,
            colors: None,
            style_builder: None,
        });
        stairs
    }

    fn max_value(&self) -> usize {
        let mut max = 0;
        let mut level: i64 = 0;
        for event in self.events() {
            if let Event::LifeEvent(life) = event {
                if life.participant == self.participant && life.is_activate() {
                    level += 1;
                }
                max = max.max(level);
                if life.participant == self.participant && life.is_deactivate_or_destroy() {
                    level -= 1;
                }
            }
        }
        max.max(0) as usize
    }

    fn max_position(&self, string_bounder: &dyn StringBounder) -> f64 {
        let _ = string_bounder;
        ComponentRoseActiveLine::WIDTH / 2.0 * self.max_value() as f64
    }

    fn delay_on(&self, y: f64, height: f64) {
        self.delays.borrow_mut().insert(Key(y), height);
    }

    fn draw_boxes(&self, ug: &UGraphic, context: Context2D, create_y: f64, end_y: f64) {
        let stairs = self.stairs(create_y, end_y);
        let max = stairs.iter().map(|step| step.indent).max().unwrap_or(0);
        for level in 1..=max {
            self.draw_one_level(ug, level, &stairs, context);
        }
        for step in &stairs {
            if step.indent == 0 {
                self.draw_destroy_if_needed(ug, step, context);
            }
        }
    }

    fn draw_one_level(&self, ug: &UGraphic, level: usize, stairs: &[Step], context: Context2D) {
        let ug = ug.translated(
            (level - 1) as f64 * ComponentRoseActiveLine::WIDTH / 2.0,
            0.0,
        );
        let mut start: Option<&Step> = None;
        for (index, step) in stairs.iter().enumerate() {
            let is_last = index + 1 == stairs.len();
            match start {
                None if step.indent == level => start = Some(step),
                Some(open) if is_last || step.indent < level => {
                    self.draw_box(&ug, open, step.value, context);
                    if step.indent > 0 {
                        self.draw_destroy_if_needed(&ug, step, context);
                    }
                    start = None;
                }
                _ => {}
            }
        }
    }

    /// `LiveBoxesDrawer.doDrawing`: the box from `start` to `end`, cut by delays.
    fn draw_box(&self, ug: &UGraphic, start: &Step, end: f64, context: Context2D) {
        let segments = cut_segment(start.value, end, &self.delays.borrow());
        let count = segments.len();
        for (index, (y1, y2)) in segments.into_iter().enumerate() {
            let close_up = index == 0;
            let close_down = index + 1 == count;
            let component = components::activation_box(
                self.diagram,
                self.participant,
                start.style_builder.as_deref(),
                start.colors.as_ref(),
                close_up,
                close_down,
            );
            let width = ComponentRoseActiveLine::WIDTH;
            component.draw_u(
                &ug.translated(-width / 2.0, y1),
                &Area::new(width, y2 - y1),
                context,
            );
        }
    }

    fn draw_destroy_if_needed(&self, ug: &UGraphic, step: &Step, context: Context2D) {
        if !step.destroy {
            return;
        }
        let cross = components::destroy(self.diagram);
        let dimension = cross.preferred_dimension(ug.string_bounder());
        cross.draw_u(
            &ug.translated(-dimension.width / 2.0, step.value - dimension.height / 2.0),
            &Area::default(),
            context,
        );
    }
}

/// `Segment.cutSegmentIfNeed`: the parts of `[pos1, pos2]` outside the delays.
fn cut_segment(pos1: f64, pos2: f64, delays: &BTreeMap<Key, f64>) -> Vec<(f64, f64)> {
    let mut result = Vec::new();
    let mut pending_start = pos1;
    for (&Key(start), &height) in delays {
        let end = start + height;
        if (start - pending_start).abs() < 0.001 {
            pending_start = end;
            continue;
        }
        if start < pending_start {
            continue;
        }
        if start > pos2 {
            if pending_start < pos2 {
                result.push((pending_start, pos2));
            }
            return result;
        }
        if start >= pos1 && start <= pos2 && end >= pos1 && end <= pos2 {
            result.push((pending_start, start));
            pending_start = end;
        }
    }
    if pending_start < pos2 {
        result.push((pending_start, pos2));
    }
    result
}
