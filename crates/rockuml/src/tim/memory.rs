//! Variable storage: one global scope shared by every function call, plus a local scope per call.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::error::{TimResult, fail};
use super::expression::TokenStack;
use super::trie::Trie;
use super::value::TValue;
use crate::json::JsonValue;
use crate::text::StringLocated;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableScope {
    Local,
    Global,
}

impl VariableScope {
    pub fn lazy_parse(word: &str) -> Option<Self> {
        if word.eq_ignore_ascii_case("local") {
            Some(Self::Local)
        } else if word.eq_ignore_ascii_case("global") {
            Some(Self::Global)
        } else {
            None
        }
    }
}

#[derive(Default)]
struct Variables {
    values: HashMap<String, TValue>,
    names: Trie,
}

impl Variables {
    fn put(&mut self, name: &str, value: TValue) {
        self.values.insert(name.to_owned(), value);
        self.names.add(name);
    }

    fn remove(&mut self, name: &str) {
        self.values.remove(name);
        self.names.remove(name);
    }
}

/// Inside a function: its arguments (and `!local` variables) shadow globals; other new variables are local.
#[derive(Default)]
struct LocalScope {
    overridden: Variables,
    local: Variables,
}

pub struct Memory {
    globals: Rc<RefCell<Variables>>,
    local: Option<LocalScope>,
    pub contexts: ExecutionContexts,
}

impl Memory {
    pub fn new_global() -> Self {
        Self {
            globals: Rc::default(),
            local: None,
            contexts: ExecutionContexts::default(),
        }
    }

    pub fn fork_from_global(&self, arguments: HashMap<String, TValue>) -> Self {
        let mut scope = LocalScope::default();
        for (name, value) in arguments {
            scope.overridden.put(&name, value);
        }
        Self {
            globals: Rc::clone(&self.globals),
            local: Some(scope),
            contexts: ExecutionContexts::default(),
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<TValue> {
        let Some(scope) = &self.local else {
            return self.globals.borrow().values.get(name).cloned();
        };
        scope
            .overridden
            .values
            .get(name)
            .cloned()
            .or_else(|| self.globals.borrow().values.get(name).cloned())
            .or_else(|| scope.local.values.get(name).cloned())
    }

    pub fn put_variable(
        &mut self,
        name: &str,
        value: TValue,
        scope: Option<VariableScope>,
        location: &StringLocated,
    ) -> TimResult<()> {
        let Some(local) = &mut self.local else {
            if scope == Some(VariableScope::Local) {
                return fail("Cannot use local variable here", location);
            }
            self.globals.borrow_mut().put(name, value);
            return Ok(());
        };
        if scope == Some(VariableScope::Global) {
            self.globals.borrow_mut().put(name, value);
        } else if scope == Some(VariableScope::Local) || local.overridden.values.contains_key(name) {
            local.overridden.put(name, value);
        } else if self.globals.borrow().values.contains_key(name) {
            self.globals.borrow_mut().put(name, value);
        } else {
            local.local.put(name, value);
        }
        Ok(())
    }

    pub fn remove_variable(&mut self, name: &str) {
        let Some(local) = &mut self.local else {
            self.globals.borrow_mut().remove(name);
            return;
        };
        if local.overridden.values.contains_key(name) {
            local.overridden.remove(name);
        } else if self.globals.borrow().values.contains_key(name) {
            self.globals.borrow_mut().remove(name);
        } else {
            local.local.remove(name);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.globals.borrow().values.is_empty()
            && self
                .local
                .as_ref()
                .is_none_or(|scope| scope.local.values.is_empty() && scope.overridden.values.is_empty())
    }

    /// The longest variable name visible in any scope that starts at `position`.
    pub fn variable_name_at(&self, chars: &[char], position: usize) -> String {
        let global = self.globals.borrow().names.longest_match_starting_in(chars, position);
        let Some(scope) = &self.local else {
            return global;
        };
        [
            scope.overridden.names.longest_match_starting_in(chars, position),
            scope.local.names.longest_match_starting_in(chars, position),
        ]
        .into_iter()
        .fold(global, |longest, candidate| if candidate.len() > longest.len() { candidate } else { longest })
    }
}

/// The `!if`, `!while` and `!foreach` blocks currently open in one scope.
#[derive(Default)]
pub struct ExecutionContexts {
    pub ifs: Vec<IfContext>,
    pub whiles: Vec<WhileContext>,
    pub foreachs: Vec<ForeachContext>,
}

impl ExecutionContexts {
    pub fn are_all_ifs_ok(&self) -> bool {
        self.ifs.iter().all(|context| context.is_true)
    }
}

pub struct IfContext {
    is_true: bool,
    /// A branch of this `!if` has already been taken, so later `!elseif`/`!else` branches are skipped.
    has_been_burnt: bool,
}

impl IfContext {
    pub fn new(is_true: bool) -> Self {
        Self {
            is_true,
            has_been_burnt: is_true,
        }
    }

    pub fn has_been_burnt(&self) -> bool {
        self.has_been_burnt
    }

    pub fn entering_else_if(&mut self) {
        self.is_true = false;
    }

    pub fn now_in_some_else_if(&mut self) {
        self.is_true = true;
        self.has_been_burnt = true;
    }

    pub fn now_in_else(&mut self) {
        self.is_true = !self.has_been_burnt;
    }
}

pub struct WhileContext {
    pub condition: TokenStack,
    pub start: usize,
    pub skip: bool,
}

pub struct ForeachContext {
    pub variable: String,
    pub values: JsonValue,
    pub start: usize,
    pub skip: bool,
    index: usize,
}

impl ForeachContext {
    pub fn new(variable: String, values: JsonValue, start: usize) -> Self {
        Self {
            variable,
            values,
            start,
            skip: false,
            index: 0,
        }
    }

    /// Arrays yield their elements, objects their member names.
    pub fn current_value(&self) -> Option<JsonValue> {
        match &self.values {
            JsonValue::Array(values) => values.get(self.index).cloned(),
            JsonValue::Object(object) => object.names().nth(self.index).map(|name| JsonValue::String(name.to_owned())),
            _ => None,
        }
    }

    pub fn increment(&mut self) {
        self.index += 1;
        if self.index >= self.values.container_len().unwrap_or(0) {
            self.skip = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::LineLocation;

    fn location() -> StringLocated {
        StringLocated::new("", LineLocation::new("t", None))
    }

    #[test]
    fn functions_see_and_update_globals() {
        let mut global = Memory::new_global();
        global.put_variable("$g", TValue::Int(1), None, &location()).unwrap();
        let mut local = global.fork_from_global(HashMap::new());
        local.put_variable("$g", TValue::Int(2), None, &location()).unwrap();
        local.put_variable("$l", TValue::Int(3), None, &location()).unwrap();
        assert_eq!(global.get_variable("$g"), Some(TValue::Int(2)));
        assert_eq!(global.get_variable("$l"), None);
        assert_eq!(local.get_variable("$l"), Some(TValue::Int(3)));
    }

    #[test]
    fn arguments_shadow_globals() {
        let mut global = Memory::new_global();
        global.put_variable("$x", TValue::Int(1), None, &location()).unwrap();
        let mut local = global.fork_from_global(HashMap::from([("$x".to_owned(), TValue::Int(9))]));
        local.put_variable("$x", TValue::Int(10), None, &location()).unwrap();
        assert_eq!(local.get_variable("$x"), Some(TValue::Int(10)));
        assert_eq!(global.get_variable("$x"), Some(TValue::Int(1)));
    }

    #[test]
    fn local_scope_is_rejected_at_top_level() {
        let mut global = Memory::new_global();
        assert!(global.put_variable("$x", TValue::Int(1), Some(VariableScope::Local), &location()).is_err());
    }

    #[test]
    fn else_runs_only_when_no_branch_was_taken() {
        let mut context = IfContext::new(false);
        context.entering_else_if();
        context.now_in_some_else_if();
        context.now_in_else();
        assert!(!context.is_true);
    }
}
