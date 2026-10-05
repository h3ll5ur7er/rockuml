//! Built-in and user-defined preprocessor functions and procedures.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::context::TContext;
use super::error::{TimError, TimResult, fail};
use super::line_type::{LineType, line_type};
use super::memory::Memory;
use super::trie::Trie;
use super::value::TValue;
use crate::java;
use crate::text::StringLocated;

/// Functions are told apart by name and argument count; named arguments only matter for `can_cover`.
#[derive(Clone, Debug)]
pub struct FunctionSignature {
    name: String,
    argument_count: usize,
    named_arguments: HashSet<String>,
}

impl FunctionSignature {
    pub fn new(name: &str, argument_count: usize) -> Self {
        Self::with_named(name, argument_count, HashSet::new())
    }

    pub fn with_named(name: &str, argument_count: usize, named_arguments: HashSet<String>) -> Self {
        Self {
            name: name.to_owned(),
            argument_count,
            named_arguments,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn argument_count(&self) -> usize {
        self.argument_count
    }

    pub fn named_arguments(&self) -> &HashSet<String> {
        &self.named_arguments
    }

    fn java_hash_code(&self) -> i32 {
        let name_hash = java::string_hash_code(&self.name);
        31i32
            .wrapping_add(name_hash)
            .wrapping_mul(31)
            .wrapping_add(i32::try_from(self.argument_count).unwrap_or(i32::MAX))
    }
}

impl PartialEq for FunctionSignature {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.argument_count == other.argument_count
    }
}

impl Eq for FunctionSignature {}

impl Hash for FunctionSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.argument_count.hash(state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionType {
    Procedure,
    ReturnFunction,
    LegacyDefine,
    LegacyDefinelong,
}

impl FunctionType {
    pub fn is_legacy(self) -> bool {
        matches!(self, Self::LegacyDefine | Self::LegacyDefinelong)
    }
}

pub trait TFunction {
    fn signature(&self) -> &FunctionSignature;
    fn can_cover(&self, argument_count: usize, named_arguments: &HashSet<String>) -> bool;
    fn function_type(&self) -> FunctionType;
    fn is_unquoted(&self) -> bool {
        false
    }
    fn execute_return_function(
        &self,
        context: &mut TContext,
        memory: &mut Memory,
        location: &StringLocated,
        arguments: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<TValue>;
    fn execute_procedure(
        &self,
        context: &mut TContext,
        memory: &mut Memory,
        location: &StringLocated,
        arguments: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<()>;
}

#[derive(Clone, Debug)]
pub struct FunctionArgument {
    name: String,
    default: Option<TValue>,
}

impl FunctionArgument {
    pub fn new(name: String, default: Option<TValue>) -> Self {
        Self { name, default }
    }
}

/// A `!function`, `!procedure`, `!define` or `!definelong` written in the diagram source.
pub struct UserFunction {
    signature: FunctionSignature,
    arguments: Vec<FunctionArgument>,
    body: Vec<StringLocated>,
    unquoted: bool,
    function_type: FunctionType,
    legacy_definition: Option<String>,
    contains_return: bool,
}

impl UserFunction {
    pub fn new(
        name: &str,
        arguments: Vec<FunctionArgument>,
        unquoted: bool,
        function_type: FunctionType,
    ) -> Self {
        let names = arguments
            .iter()
            .map(|argument| argument.name.clone())
            .collect();
        Self {
            signature: FunctionSignature::with_named(name, arguments.len(), names),
            arguments,
            body: Vec::new(),
            unquoted,
            function_type,
            legacy_definition: None,
            contains_return: false,
        }
    }

    pub fn add_body(&mut self, line: StringLocated) -> TimResult<()> {
        if line_type(line.text()) == LineType::Return {
            self.contains_return = true;
            if self.function_type == FunctionType::Procedure {
                return fail(
                    "A procedure cannot have !return directive. Declare it as a function instead ?",
                    &line,
                );
            }
        }
        self.body.push(line);
        Ok(())
    }

    pub fn has_body(&self) -> bool {
        !self.body.is_empty()
    }

    pub fn contains_return(&self) -> bool {
        self.contains_return
    }

    pub fn set_legacy_definition(&mut self, definition: String) {
        self.legacy_definition = Some(definition);
    }

    /// A one-line `!definelong` behaves exactly like `!define`.
    fn finalize_enddefinelong(&mut self) {
        if self.body.len() == 1 {
            self.function_type = FunctionType::LegacyDefine;
            self.legacy_definition = Some(self.body[0].text().to_owned());
        }
    }

    fn new_memory(
        &self,
        memory: &Memory,
        values: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<Memory> {
        let mut positional = values.iter();
        let mut bound = HashMap::new();
        for argument in &self.arguments {
            let value = named
                .get(&argument.name)
                .or_else(|| positional.next())
                .or(argument.default.as_ref())
                .ok_or(TimError::Fatal)?;
            bound.insert(argument.name.clone(), value.clone());
        }
        Ok(memory.fork_from_global(bound))
    }
}

impl TFunction for UserFunction {
    fn signature(&self) -> &FunctionSignature {
        &self.signature
    }

    fn can_cover(&self, argument_count: usize, named_arguments: &HashSet<String>) -> bool {
        if !named_arguments.is_subset(&self.signature.named_arguments)
            || argument_count > self.arguments.len()
        {
            return false;
        }
        let needed = self
            .arguments
            .iter()
            .filter(|argument| {
                !named_arguments.contains(&argument.name) && argument.default.is_none()
            })
            .count();
        argument_count >= needed
    }

    fn function_type(&self) -> FunctionType {
        self.function_type
    }

    fn is_unquoted(&self) -> bool {
        self.unquoted
    }

    fn execute_return_function(
        &self,
        context: &mut TContext,
        memory: &mut Memory,
        location: &StringLocated,
        arguments: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<TValue> {
        if self.function_type == FunctionType::LegacyDefine {
            let mut local = self.new_memory(memory, arguments, &HashMap::new())?;
            let definition =
                location.with_text(self.legacy_definition.clone().ok_or(TimError::Fatal)?);
            let expanded = context.apply_functions_and_variables(&mut local, &definition)?;
            return Ok(TValue::string(expanded.unwrap_or_default()));
        }
        if self.function_type != FunctionType::ReturnFunction {
            return fail(
                "Illegal call here. Is there a return directive in your function?",
                location,
            );
        }
        let mut local = self.new_memory(memory, arguments, named)?;
        match context.execute_lines(
            &mut local,
            &self.body,
            Some(FunctionType::ReturnFunction),
            true,
        )? {
            Some(result) => Ok(result),
            None => fail("No return directive found in your function", location),
        }
    }

    fn execute_procedure(
        &self,
        context: &mut TContext,
        memory: &mut Memory,
        _location: &StringLocated,
        arguments: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<()> {
        if !matches!(
            self.function_type,
            FunctionType::Procedure | FunctionType::LegacyDefinelong
        ) {
            return Err(TimError::Fatal);
        }
        let mut local = self.new_memory(memory, arguments, named)?;
        context.execute_lines(&mut local, &self.body, Some(FunctionType::Procedure), false)?;
        Ok(())
    }
}

#[derive(Default)]
pub struct FunctionsSet {
    /// In insertion order; replacing a function keeps its slot, as `HashMap.put` does.
    functions: Vec<Rc<dyn TFunction>>,
    finals: HashSet<FunctionSignature>,
    names: Trie,
    pending: Option<UserFunction>,
}

impl FunctionsSet {
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    pub fn add(&mut self, function: Rc<dyn TFunction>) {
        self.names.add(&format!("{}(", function.signature().name()));
        match self
            .functions
            .iter()
            .position(|existing| existing.signature() == function.signature())
        {
            Some(index) => self.functions[index] = function,
            None => self.functions.push(function),
        }
    }

    pub fn add_user_function(&mut self, mut function: UserFunction) {
        if function.function_type == FunctionType::LegacyDefinelong {
            function.finalize_enddefinelong();
        }
        self.add(Rc::new(function));
    }

    /// The function with exactly this signature, or else the first one with the same name that accepts
    /// the arguments, in the order a Java `HashMap` would offer them.
    pub fn get_smart(&self, searched: &FunctionSignature) -> Option<Rc<dyn TFunction>> {
        if let Some(exact) = self
            .functions
            .iter()
            .find(|function| function.signature() == searched)
        {
            return Some(Rc::clone(exact));
        }
        let hashes: Vec<i32> = self
            .functions
            .iter()
            .map(|function| function.signature().java_hash_code())
            .collect();
        java::hash_map_iteration_order(&hashes)
            .into_iter()
            .map(|index| &self.functions[index])
            .find(|function| {
                function.signature().name() == searched.name()
                    && function.can_cover(searched.argument_count(), searched.named_arguments())
            })
            .cloned()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.with_name(name).next().is_some()
    }

    pub fn is_legacy_define(&self, name: &str) -> bool {
        self.with_name(name)
            .any(|function| function.function_type().is_legacy())
    }

    pub fn is_unquoted(&self, name: &str) -> bool {
        self.with_name(name).any(|function| function.is_unquoted())
    }

    fn with_name<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Rc<dyn TFunction>> {
        self.functions
            .iter()
            .filter(move |function| function.signature().name() == name)
    }

    /// The name of a function called at `position`, i.e. a known name directly followed by `(`.
    pub fn name_called_at(&self, chars: &[char], position: usize) -> Option<String> {
        let mut name = self.names.longest_match_starting_in(chars, position);
        name.pop()?;
        Some(name)
    }

    pub fn pending(&self) -> Option<&UserFunction> {
        self.pending.as_ref()
    }

    pub fn pending_mut(&mut self) -> Option<&mut UserFunction> {
        self.pending.as_mut()
    }

    pub fn start_pending(&mut self, function: UserFunction) {
        self.pending = Some(function);
    }

    pub fn finish_pending(&mut self) {
        if let Some(function) = self.pending.take() {
            self.add_user_function(function);
        }
    }

    /// Registers a `!function` or `!procedure` declaration, honouring `!final`.
    pub fn declare(
        &mut self,
        function: UserFunction,
        is_final: bool,
        location: &StringLocated,
    ) -> TimResult<()> {
        let signature = function.signature.clone();
        let already_defined = self
            .functions
            .iter()
            .any(|existing| *existing.signature() == signature);
        if already_defined && (is_final || self.finals.contains(&signature)) {
            return fail("This function is already defined", location);
        }
        if is_final {
            self.finals.insert(signature);
        }
        if function.has_body() {
            self.add_user_function(function);
        } else {
            self.pending = Some(function);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(name: &str, arguments: &[(&str, Option<i32>)]) -> UserFunction {
        let arguments = arguments
            .iter()
            .map(|(name, default)| {
                FunctionArgument::new((*name).to_owned(), default.map(TValue::Int))
            })
            .collect();
        UserFunction::new(name, arguments, false, FunctionType::ReturnFunction)
    }

    #[test]
    fn defaults_make_trailing_arguments_optional() {
        let f = function("$f", &[("$a", None), ("$b", Some(1))]);
        assert!(f.can_cover(1, &HashSet::new()));
        assert!(f.can_cover(2, &HashSet::new()));
        assert!(!f.can_cover(0, &HashSet::new()));
        assert!(!f.can_cover(3, &HashSet::new()));
        assert!(f.can_cover(0, &HashSet::from(["$a".to_owned()])));
        assert!(!f.can_cover(1, &HashSet::from(["$z".to_owned()])));
    }

    #[test]
    fn smart_lookup_falls_back_to_a_covering_overload() {
        let mut set = FunctionsSet::default();
        set.add_user_function(function("$f", &[("$a", None), ("$b", Some(1))]));
        let found = set.get_smart(&FunctionSignature::new("$f", 1)).unwrap();
        assert_eq!(found.signature().argument_count(), 2);
        assert!(set.get_smart(&FunctionSignature::new("$g", 1)).is_none());
    }

    #[test]
    fn called_names_need_an_opening_parenthesis() {
        let mut set = FunctionsSet::default();
        set.add_user_function(function("$f", &[]));
        let chars: Vec<char> = "x $f() $f".chars().collect();
        assert_eq!(set.name_called_at(&chars, 2).as_deref(), Some("$f"));
        assert_eq!(set.name_called_at(&chars, 7), None);
    }
}
