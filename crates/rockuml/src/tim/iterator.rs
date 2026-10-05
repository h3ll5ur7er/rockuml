//! Walks a list of lines while consuming the directives that steer the walk itself: comments, `!startsub`,
//! function and procedure bodies, `!if`, `!define`, `!while`, `!foreach` and variable assignments.
//!
//! Each layer filters what the layer below yields, in the same order as PlantUML's iterator chain; the
//! order decides, for example, that a function body is captured before its `!if` lines are evaluated.

use super::context::TContext;
use super::eater::Eater;
use super::error::{TimError, TimResult, fail};
use super::function::{FunctionType, TFunction, UserFunction};
use super::line_type::{LineType, line_type};
use super::memory::{ForeachContext, IfContext, Memory, VariableScope, WhileContext};
use super::value::TValue;
use crate::java;
use crate::text::StringLocated;

pub trait CodeIterator {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>>;
    fn next(&mut self) -> TimResult<()>;
    fn code_position(&self) -> usize;
    fn jump_to_code_position(&mut self, position: usize, location: &StringLocated)
    -> TimResult<()>;
}

/// Builds the full chain over `lines`.
pub fn code_iterator(lines: Vec<StringLocated>) -> Box<dyn CodeIterator> {
    let lines = Box::new(Lines::new(lines));
    let long_comments = Box::new(LongComments { source: lines });
    let short_comments = Box::new(ShortComments {
        source: long_comments,
    });
    let inner_comments = Box::new(InnerComments {
        source: short_comments,
    });
    let subs = Box::new(Subs {
        source: inner_comments,
        replaying: None,
    });
    let return_functions = Box::new(ReturnFunctions { source: subs });
    let procedures = Box::new(Procedures {
        source: return_functions,
    });
    let ifs = Box::new(Ifs { source: procedures });
    let legacy_defines = Box::new(LegacyDefines { source: ifs });
    let whiles = Box::new(Whiles {
        source: legacy_defines,
    });
    let foreachs = Box::new(Foreachs { source: whiles });
    Box::new(Affectations { source: foreachs })
}

macro_rules! delegate_position {
    () => {
        fn code_position(&self) -> usize {
            self.source.code_position()
        }

        fn jump_to_code_position(
            &mut self,
            position: usize,
            location: &StringLocated,
        ) -> TimResult<()> {
            self.source.jump_to_code_position(position, location)
        }
    };
}

macro_rules! delegate_navigation {
    () => {
        fn next(&mut self) -> TimResult<()> {
            self.source.next()
        }

        delegate_position!();
    };
}

struct Lines {
    lines: Vec<StringLocated>,
    current: usize,
    jumps: usize,
}

impl Lines {
    fn new(lines: Vec<StringLocated>) -> Self {
        Self {
            lines,
            current: 0,
            jumps: 0,
        }
    }

    fn current_line(&self) -> Option<StringLocated> {
        self.lines.get(self.current).cloned()
    }
}

impl CodeIterator for Lines {
    fn peek(&mut self, _: &mut TContext, _: &mut Memory) -> TimResult<Option<StringLocated>> {
        Ok(self.current_line())
    }

    fn next(&mut self) -> TimResult<()> {
        if self.current >= self.lines.len() {
            return Err(TimError::Fatal);
        }
        self.current += 1;
        Ok(())
    }

    fn code_position(&self) -> usize {
        self.current
    }

    fn jump_to_code_position(
        &mut self,
        position: usize,
        location: &StringLocated,
    ) -> TimResult<()> {
        self.jumps += 1;
        if self.jumps > 999 {
            return fail("Infinite loop?", location);
        }
        self.current = position;
        Ok(())
    }
}

struct LongComments {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for LongComments {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        loop {
            let Some(line) = self.source.peek(context, memory)? else {
                return Ok(None);
            };
            if line_type(line.text()) != LineType::CommentLongStart {
                return Ok(Some(line));
            }
            while let Some(line) = self.source.peek(context, memory)? {
                context.log(&line);
                self.source.next()?;
                if java::trim(line.text()).ends_with("'/") {
                    break;
                }
            }
        }
    }

    delegate_navigation!();
}

struct ShortComments {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for ShortComments {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        while let Some(line) = self.source.peek(context, memory)? {
            if line_type(line.text()) != LineType::CommentSimple {
                return Ok(Some(line));
            }
            context.log(&line);
            self.next()?;
        }
        Ok(None)
    }

    delegate_navigation!();
}

struct InnerComments {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for InnerComments {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        Ok(self
            .source
            .peek(context, memory)?
            .map(|line| line.remove_inner_comment()))
    }

    delegate_navigation!();
}

/// Records `!startsub NAME` ... `!endsub` blocks for `!includesub`, then replays them in place.
struct Subs {
    source: Box<dyn CodeIterator>,
    replaying: Option<Lines>,
}

impl CodeIterator for Subs {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        if let Some(replaying) = &self.replaying {
            return Ok(replaying.current_line());
        }
        let Some(line) = self.source.peek(context, memory)? else {
            return Ok(None);
        };
        if line_type(line.text()) == LineType::Startsub {
            let name = parse_startsub(&line.trimmed())?;
            let mut lines = Vec::new();
            self.source.next()?;
            while let Some(inner) = self.source.peek(context, memory)? {
                match line_type(inner.text()) {
                    LineType::Startsub => return fail("Cannot nest sub", &line),
                    LineType::Endsub => {
                        self.source.next()?;
                        self.replaying = Some(Lines::new(lines.clone()));
                        break;
                    }
                    _ => {
                        lines.push(inner);
                        self.source.next()?;
                    }
                }
            }
            context.subs.insert(name, lines);
        }
        match &self.replaying {
            Some(replaying) => Ok(replaying.current_line()),
            None => Ok(Some(line)),
        }
    }

    fn next(&mut self) -> TimResult<()> {
        let Some(replaying) = &mut self.replaying else {
            return self.source.next();
        };
        replaying.next()?;
        if replaying.current_line().is_none() {
            self.replaying = None;
        }
        Ok(())
    }

    delegate_position!();
}

/// `!startsub NAME`: the name must be a single word.
pub fn parse_startsub(line: &StringLocated) -> TimResult<String> {
    let mut eater = Eater::new(line.clone());
    eater.skip_spaces();
    eater.check_and_eat("!startsub")?;
    eater.skip_spaces();
    let name = eater.eat_all_to_end();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return fail("Bad sub name", line);
    }
    Ok(name)
}

struct ReturnFunctions {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for ReturnFunctions {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        while let Some(line) = self.source.peek(context, memory)? {
            let pending_is_function = context
                .functions
                .pending()
                .is_some_and(|pending| pending.function_type() == FunctionType::ReturnFunction);
            if pending_is_function {
                context.log(&line);
                if line_type(line.text()) == LineType::EndFunction {
                    if !context
                        .functions
                        .pending()
                        .is_some_and(UserFunction::contains_return)
                    {
                        return fail(
                            "This function does not have any !return directive. Declare it as a procedure instead ?",
                            &line,
                        );
                    }
                    context.functions.finish_pending();
                } else if let Some(pending) = context.functions.pending_mut() {
                    pending.add_body(line)?;
                }
                self.next()?;
                continue;
            }
            if line_type(line.text()) == LineType::DeclareReturnFunction {
                context.log(&line);
                context.declare_function(memory, &line, FunctionType::ReturnFunction)?;
                self.next()?;
                continue;
            }
            return Ok(Some(line));
        }
        Ok(None)
    }

    delegate_navigation!();
}

struct Procedures {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for Procedures {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        while let Some(line) = self.source.peek(context, memory)? {
            let pending_is_procedure = context.functions.pending().is_some_and(|pending| {
                matches!(
                    pending.function_type(),
                    FunctionType::Procedure | FunctionType::LegacyDefinelong
                )
            });
            if pending_is_procedure {
                context.log(&line);
                if line_type(line.text()) == LineType::EndFunction {
                    context.functions.finish_pending();
                } else if let Some(pending) = context.functions.pending_mut() {
                    pending.add_body(line)?;
                }
                self.next()?;
                continue;
            }
            if line_type(line.text()) == LineType::DeclareProcedure {
                context.log(&line);
                context.declare_function(memory, &line, FunctionType::Procedure)?;
                self.next()?;
                continue;
            }
            return Ok(Some(line));
        }
        Ok(None)
    }

    delegate_navigation!();
}

struct Ifs {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for Ifs {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        while let Some(line) = self.source.peek(context, memory)? {
            let kind = line_type(line.text());
            let is_conditional = matches!(
                kind,
                LineType::If
                    | LineType::Ifdef
                    | LineType::Ifndef
                    | LineType::Else
                    | LineType::Elseif
                    | LineType::Endif
            );
            if is_conditional {
                context.log(&line);
                execute_conditional(kind, &line, context, memory)?;
                self.next()?;
                continue;
            }
            if !memory.contexts.ifs.is_empty() && !memory.contexts.are_all_ifs_ok() {
                context.log(&line);
                self.next()?;
                continue;
            }
            return Ok(Some(line));
        }
        Ok(None)
    }

    delegate_navigation!();
}

fn execute_conditional(
    kind: LineType,
    line: &StringLocated,
    context: &mut TContext,
    memory: &mut Memory,
) -> TimResult<()> {
    match kind {
        LineType::If => {
            let is_true = if memory.contexts.are_all_ifs_ok() {
                eval_condition_after(line, "!if", context, memory)?
            } else {
                false
            };
            memory.contexts.ifs.push(IfContext::new(is_true));
        }
        LineType::Ifdef => {
            let mut eater = Eater::new(line.clone());
            eater.skip_spaces();
            eater.check_and_eat("!ifdef")?;
            eater.skip_spaces();
            let expression = eater.eat_all_to_end();
            let is_true = super::eval_boolean::eval(&expression, |name| {
                memory.get_variable(name).is_some() || context.functions.exists(name)
            })
            .ok_or(TimError::Fatal)?;
            memory.contexts.ifs.push(IfContext::new(is_true));
        }
        LineType::Ifndef => {
            let mut eater = Eater::new(line.clone());
            eater.skip_spaces();
            eater.check_and_eat("!ifndef")?;
            eater.skip_spaces();
            let name = eater.eat_and_get_varname()?;
            let is_true = memory.get_variable(&name).is_none() && !context.functions.exists(&name);
            memory.contexts.ifs.push(IfContext::new(is_true));
        }
        LineType::Elseif => {
            if memory.contexts.ifs.is_empty() {
                return fail("No if related to this elseif", line);
            }
            let last = memory.contexts.ifs.len() - 1;
            memory.contexts.ifs[last].entering_else_if();
            if !memory.contexts.ifs[last].has_been_burnt()
                && eval_condition_after(line, "!elseif", context, memory)?
            {
                memory.contexts.ifs[last].now_in_some_else_if();
            }
        }
        LineType::Else => match memory.contexts.ifs.last_mut() {
            Some(context) => context.now_in_else(),
            None => return fail("No if related to this else", line),
        },
        LineType::Endif => {
            if memory.contexts.ifs.pop().is_none() {
                return fail("No if related to this endif", line);
            }
        }
        _ => unreachable!("not a conditional directive"),
    }
    Ok(())
}

fn eval_condition_after(
    line: &StringLocated,
    keyword: &str,
    context: &mut TContext,
    memory: &mut Memory,
) -> TimResult<bool> {
    let mut eater = Eater::new(line.clone());
    eater.skip_spaces();
    eater.check_and_eat(keyword)?;
    eater.skip_spaces();
    Ok(eater.eat_expression(context, memory)?.to_bool())
}

struct LegacyDefines {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for LegacyDefines {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        while let Some(line) = self.source.peek(context, memory)? {
            match line_type(line.text()) {
                LineType::LegacyDefine => {
                    context.log(&line);
                    context.execute_legacy_define(memory, &line)?;
                }
                LineType::LegacyDefinelong => {
                    context.log(&line);
                    context.execute_legacy_definelong(memory, &line)?;
                }
                _ => return Ok(Some(line)),
            }
            self.next()?;
        }
        Ok(None)
    }

    delegate_navigation!();
}

struct Whiles {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for Whiles {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        let mut level = 0;
        while let Some(line) = self.source.peek(context, memory)? {
            let kind = line_type(line.text());
            if memory
                .contexts
                .whiles
                .last()
                .is_some_and(|current| current.skip)
            {
                if kind == LineType::While {
                    level += 1;
                } else if kind == LineType::Endwhile {
                    level -= 1;
                    if level == -1 {
                        memory.contexts.whiles.pop();
                        level = 0;
                    }
                }
                self.next()?;
                continue;
            }
            match kind {
                LineType::While => {
                    context.log(&line);
                    let mut eater = Eater::new(line.trimmed());
                    eater.skip_spaces();
                    eater.check_and_eat("!while")?;
                    eater.skip_spaces();
                    let condition = eater.eat_token_stack()?;
                    let is_true = condition
                        .get_result(&line.trimmed(), context, memory)?
                        .to_bool();
                    memory.contexts.whiles.push(WhileContext {
                        condition,
                        start: self.source.code_position(),
                        skip: !is_true,
                    });
                }
                LineType::Endwhile => {
                    context.log(&line);
                    let Some(current) = memory.contexts.whiles.last() else {
                        return fail("No while related to this endwhile", &line);
                    };
                    let (condition, start) = (current.condition.clone(), current.start);
                    if condition.get_result(&line, context, memory)?.to_bool() {
                        self.source.jump_to_code_position(start, &line)?;
                    } else {
                        memory.contexts.whiles.pop();
                    }
                }
                _ => return Ok(Some(line)),
            }
            self.next()?;
        }
        Ok(None)
    }

    delegate_navigation!();
}

struct Foreachs {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for Foreachs {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        let mut level = 0;
        while let Some(line) = self.source.peek(context, memory)? {
            let kind = line_type(line.text());
            if memory
                .contexts
                .foreachs
                .last()
                .is_some_and(|current| current.skip)
            {
                if kind == LineType::Foreach {
                    level += 1;
                } else if kind == LineType::Endforeach {
                    level -= 1;
                    if level == -1 {
                        memory.contexts.foreachs.pop();
                        level = 0;
                    }
                }
                self.next()?;
                continue;
            }
            match kind {
                LineType::Foreach => {
                    context.log(&line);
                    self.start_foreach(&line.trimmed(), context, memory)?;
                }
                LineType::Endforeach => {
                    context.log(&line);
                    let Some(current) = memory.contexts.foreachs.last_mut() else {
                        return fail("No foreach related to this endforeach", &line);
                    };
                    current.increment();
                    if current.skip {
                        memory.contexts.foreachs.pop();
                    } else {
                        let start = current.start;
                        set_loop_variable(memory, &line)?;
                        self.source.jump_to_code_position(start, &line)?;
                    }
                }
                _ => return Ok(Some(line)),
            }
            self.next()?;
        }
        Ok(None)
    }

    delegate_navigation!();
}

impl Foreachs {
    fn start_foreach(
        &mut self,
        line: &StringLocated,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<()> {
        let mut eater = Eater::new(line.clone());
        eater.skip_spaces();
        eater.check_and_eat("!foreach")?;
        eater.skip_spaces();
        let variable = eater.eat_and_get_varname()?;
        eater.skip_spaces();
        eater.check_and_eat("in")?;
        eater.skip_spaces();
        let values = eater.eat_expression(context, memory)?;
        let values = values.as_json().cloned();
        let length = match &values {
            None => 0,
            Some(values) => values.container_len().ok_or(TimError::Fatal)?,
        };
        let mut foreach = ForeachContext::new(
            variable,
            values.unwrap_or(crate::json::JsonValue::Null),
            self.source.code_position(),
        );
        let skip = length == 0;
        foreach.skip = skip;
        memory.contexts.foreachs.push(foreach);
        if !skip {
            set_loop_variable(memory, line)?;
        }
        Ok(())
    }
}

fn set_loop_variable(memory: &mut Memory, location: &StringLocated) -> TimResult<()> {
    let current = memory.contexts.foreachs.last().ok_or(TimError::Fatal)?;
    let value = current.current_value().ok_or(TimError::Fatal)?;
    let variable = current.variable.clone();
    memory.put_variable(
        &variable,
        TValue::Json(value),
        Some(VariableScope::Global),
        location,
    )
}

struct Affectations {
    source: Box<dyn CodeIterator>,
}

impl CodeIterator for Affectations {
    fn peek(
        &mut self,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Option<StringLocated>> {
        while let Some(line) = self.source.peek(context, memory)? {
            if line_type(line.text()) != LineType::Affectation {
                return Ok(Some(line));
            }
            context.log(&line);
            self.assign_reading_ahead_for_json(line, context, memory)?;
            self.next()?;
        }
        Ok(None)
    }

    delegate_navigation!();
}

impl Affectations {
    /// A JSON value may span several lines: on a parse error, append the next line and retry for as long
    /// as the error moves further along.
    fn assign_reading_ahead_for_json(
        &mut self,
        mut line: StringLocated,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<()> {
        let mut last_column = None;
        for _ in 0..9999 {
            match context.execute_affectation(memory, &line) {
                Err(TimError::JsonParse(error)) => {
                    if last_column.is_some_and(|last| error.column <= last) {
                        return fail("Error in JSON format", &line);
                    }
                    last_column = Some(error.column);
                    self.next()?;
                    let following = self.source.peek(context, memory)?.ok_or(TimError::Fatal)?;
                    line = line.append(following.text());
                }
                result => return result,
            }
        }
        Ok(())
    }
}
