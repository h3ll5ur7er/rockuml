//! Executes preprocessor lines: expands variables and function calls in plain lines and runs directives.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::builtins;
use super::eater::Eater;
use super::error::{EaterException, TimError, TimResult, fail};
use super::function::{FunctionSignature, FunctionType, FunctionsSet, TFunction};
use super::iterator::{code_iterator, parse_startsub};
use super::line_type::{LineType, is_letter_or_emoji_or_underscore_or_digit, line_type};
use super::memory::{Memory, VariableScope};
use super::paths::{Folder, InputFile, PathSystem};
use super::value::TValue;
use super::{Definitions, PreprocessorEnvironment};
use crate::host::Host;
use crate::java;
use crate::jaws;
use crate::json::{self, JsonObject, JsonValue};
use crate::preproc::read_lines_of_first_diagram;
use crate::stdlib;
use crate::text::{LineLocation, StringLocated};

pub struct TContext<'a> {
    pub functions: FunctionsSet,
    pub subs: HashMap<String, Vec<StringLocated>>,
    result: Vec<StringLocated>,
    debug: Vec<StringLocated>,
    /// Text that preceded a procedure call on its line, prefixed to the procedure's first output line.
    pending_add: Option<String>,
    theme_metadata: JsonObject,
    paths: PathSystem,
    pub(super) host: &'a dyn Host,
    pub(super) environment: &'a PreprocessorEnvironment,
    definitions: &'a dyn Definitions,
    pub(super) options: Vec<(String, String)>,
    pub(super) random: java::Random,
}

impl<'a> TContext<'a> {
    pub fn new(
        host: &'a dyn Host,
        environment: &'a PreprocessorEnvironment,
        definitions: &'a dyn Definitions,
        current_folder: Folder,
    ) -> Self {
        let mut functions = FunctionsSet::default();
        builtins::register(&mut functions);
        Self {
            functions,
            subs: HashMap::new(),
            result: Vec::new(),
            debug: Vec::new(),
            pending_add: None,
            theme_metadata: JsonObject::new(),
            paths: PathSystem::new(current_folder),
            host,
            environment,
            definitions,
            options: Vec::new(),
            random: java::Random::new(host.current_time_millis()),
        }
    }

    pub fn into_result(self) -> Vec<StringLocated> {
        self.result
    }

    pub fn result_mut(&mut self) -> &mut Vec<StringLocated> {
        &mut self.result
    }

    pub fn log(&mut self, line: &StringLocated) {
        self.debug.push(line.clone());
    }

    pub fn theme_metadata(&self) -> &JsonObject {
        &self.theme_metadata
    }

    pub fn function_smart(&self, signature: &FunctionSignature) -> Option<Rc<dyn TFunction>> {
        self.functions.get_smart(signature)
    }

    /// Runs `body`; inside a function, returns the value of the `!return` that ended it.
    pub fn execute_lines(
        &mut self,
        memory: &mut Memory,
        body: &[StringLocated],
        function_type: Option<FunctionType>,
        in_return_function: bool,
    ) -> TimResult<Option<TValue>> {
        let mut lines = code_iterator(body.to_vec());
        while let Some(line) = lines.peek(self, memory)? {
            let result =
                self.execute_one_line_safe(memory, &line, function_type, in_return_function)?;
            if result.is_some() {
                return Ok(result);
            }
            lines.next()?;
        }
        Ok(None)
    }

    fn execute_one_line_safe(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
        function_type: Option<FunctionType>,
        in_return_function: bool,
    ) -> TimResult<Option<TValue>> {
        self.debug.push(line.clone());
        match self.execute_one_line(memory, line, function_type, in_return_function) {
            Err(TimError::Fatal | TimError::JsonParse(_)) => fail("Fatal parsing error", line),
            other => other,
        }
    }

    fn execute_one_line(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
        function_type: Option<FunctionType>,
        in_return_function: bool,
    ) -> TimResult<Option<TValue>> {
        static ONLY_WHITESPACE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[\t\n\x0B\x0C\r ]+$").unwrap());

        let in_function = function_type == Some(FunctionType::ReturnFunction);
        match line_type(line.text()) {
            LineType::Includesub => self.execute_includesub(memory, line)?,
            LineType::Theme => self.execute_theme(memory, line)?,
            LineType::Include => self.execute_include(memory, line)?,
            LineType::IncludeDef => self.execute_includedef(memory, line)?,
            LineType::Import => self.execute_import(memory, line)?,
            LineType::DumpMemory => {
                let mut eater = Eater::new(line.trimmed());
                eater.skip_spaces();
                eater.check_and_eat("!dump_memory")?;
            }
            LineType::Assert => self.execute_assert(memory, &line.trimmed())?,
            LineType::Option => self.execute_option(memory, &line.trimmed())?,
            LineType::Undef => {
                let mut eater = Eater::new(line.trimmed());
                eater.skip_spaces();
                eater.check_and_eat("!undef")?;
                eater.skip_spaces();
                memory.remove_variable(&eater.eat_and_get_varname()?);
            }
            LineType::Plain if !in_function => self.add_plain(memory, line)?,
            LineType::Return if in_function => {
                if in_return_function {
                    let mut eater = Eater::new(line.clone());
                    eater.skip_spaces();
                    eater.check_and_eat("!return")?;
                    eater.skip_spaces();
                    return Ok(Some(eater.eat_expression(self, memory)?));
                }
            }
            LineType::Plain => {
                self.apply_functions_and_variables_internal(memory, line)?;
            }
            LineType::AffectationDefine => self.execute_affectation_define(memory, line)?,
            LineType::EndFunction if function_type.is_none() => {}
            LineType::Log => {
                let mut eater = Eater::new(line.trimmed());
                eater.skip_spaces();
                eater.check_and_eat("!log")?;
                eater.skip_spaces();
                let text = line.with_text(eater.eat_all_to_end());
                self.apply_functions_and_variables(memory, &text)?;
            }
            _ if ONLY_WHITESPACE.is_match(line.text()) => {}
            other => {
                let function_type = function_type.map_or("null".to_owned(), |kind| {
                    java_enum_name(&format!("{kind:?}"))
                });
                return fail(
                    format!(
                        "Compile Error {function_type} {}",
                        java_enum_name(&format!("{other:?}"))
                    ),
                    line,
                );
            }
        }
        Ok(None)
    }

    fn add_plain(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let Some(mut lines) = self.apply_functions_and_variables_internal(memory, line)? else {
            return Ok(());
        };
        if let Some(prefix) = self.pending_add.take() {
            let first = lines.first_mut().ok_or(TimError::Fatal)?;
            *first = first.with_text(format!("{prefix}{}", first.text()));
        }
        self.result.extend(lines);
        Ok(())
    }

    /// Expands a line and splits it at the `\n` that expansion may produce.
    fn apply_functions_and_variables_internal(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
    ) -> TimResult<Option<Vec<StringLocated>>> {
        if memory.is_empty() && self.functions.len() == 0 {
            return Ok(Some(vec![line.clone()]));
        }
        let Some(expanded) = self.apply_functions_and_variables(memory, line)? else {
            return Ok(None);
        };
        Ok(Some(
            java::split(&expanded, "\n")
                .into_iter()
                .map(|part| StringLocated::new(part, line.location().clone()))
                .collect(),
        ))
    }

    /// Replaces variables and function calls in `line`. `None` when a procedure call took over the line:
    /// the procedure then wrote its own output lines.
    pub fn apply_functions_and_variables(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
    ) -> TimResult<Option<String>> {
        if memory.is_empty() && self.functions.len() == 0 {
            return Ok(Some(line.text().to_owned()));
        }
        let chars: Vec<char> = line.text().chars().collect();
        let mut result = String::new();
        let mut index = 0;
        while index < chars.len() {
            if let Some(name) = self.function_name_at(&chars, index) {
                let rest = line.with_text(chars[index..].iter().collect::<String>());
                let call = FunctionCall::parse(
                    &rest,
                    self.functions.is_legacy_define(&name),
                    self.functions.is_unquoted(&name),
                    self,
                    memory,
                )?;
                let signature = FunctionSignature::with_named(
                    &name,
                    call.values.len(),
                    call.named.keys().cloned().collect(),
                );
                let Some(function) = self.functions.get_smart(&signature) else {
                    return fail(format!("Function not found {name}"), line);
                };
                match function.function_type() {
                    FunctionType::Procedure => {
                        self.pending_add = Some(result);
                        function.execute_procedure(
                            self,
                            memory,
                            line,
                            &call.values,
                            &call.named,
                        )?;
                        let remaining: String = chars[index + call.end_position..].iter().collect();
                        if !remaining.is_empty() {
                            self.append_to_last_result(&remaining)?;
                        }
                        return Ok(None);
                    }
                    FunctionType::LegacyDefinelong => {
                        self.pending_add = Some(chars[..index].iter().collect());
                        function.execute_procedure(
                            self,
                            memory,
                            line,
                            &call.values,
                            &call.named,
                        )?;
                        return Ok(None);
                    }
                    FunctionType::ReturnFunction | FunctionType::LegacyDefine => {
                        let value = function.execute_return_function(
                            self,
                            memory,
                            line,
                            &call.values,
                            &call.named,
                        )?;
                        result.push_str(&value.to_string());
                        index += call.end_position;
                    }
                }
            } else if let Some(name) = variable_name_at(memory, &chars, index) {
                index = self.replace_variable(memory, line, &chars, index, &name, &mut result)?;
            } else {
                result.push(chars[index]);
                index += 1;
            }
        }
        Ok(Some(result))
    }

    fn append_to_last_result(&mut self, remaining: &str) -> TimResult<()> {
        let last = self.result.last_mut().ok_or(TimError::Fatal)?;
        *last = last.append(remaining);
        Ok(())
    }

    /// A function name counts only at the start of a word, or right after `\n` written in the source.
    fn function_name_at(&self, chars: &[char], position: usize) -> Option<String> {
        if is_just_after_a_letter(chars, position)
            && chars[position] != '%'
            && chars[position] != '$'
        {
            return None;
        }
        self.functions.name_called_at(chars, position)
    }

    /// Appends the value of variable `name` (found at `index`) and returns the index after it.
    fn replace_variable(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
        chars: &[char],
        index: usize,
        name: &str,
        result: &mut String,
    ) -> TimResult<usize> {
        if result.ends_with("##") {
            result.truncate(result.len() - 2);
        }
        let value = memory.get_variable(name).ok_or(TimError::Fatal)?;
        let mut last = index + name.chars().count() - 1;
        match &value {
            TValue::Json(JsonValue::String(string)) => result.push_str(string),
            TValue::Json(JsonValue::Number(number)) => result.push_str(number),
            TValue::Json(json @ (JsonValue::Array(_) | JsonValue::Object(_))) => {
                last = self.replace_json(memory, line, json.clone(), chars, last + 1, result)? - 1;
            }
            TValue::Json(JsonValue::Bool(_) | JsonValue::Null) => return Err(TimError::Fatal),
            other => result.push_str(&other.to_string()),
        }
        if chars.get(last + 1) == Some(&'#') && chars.get(last + 2) == Some(&'#') {
            last += 2;
        }
        Ok(last + 1)
    }

    /// Follows `.field` and `[index]` accessors after a JSON variable, appends what they select and returns
    /// the index after the last accessor.
    fn replace_json(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
        json: JsonValue,
        chars: &[char],
        mut index: usize,
        result: &mut String,
    ) -> TimResult<usize> {
        let mut selected = Some(json);
        while index < chars.len() {
            match chars[index] {
                '.' => {
                    index += 1;
                    let start = index;
                    while index < chars.len() && is_java_identifier_part(chars[index]) {
                        index += 1;
                    }
                    let field: String = chars[start..index].iter().collect();
                    let Some(JsonValue::Object(object)) = &selected else {
                        return Err(TimError::Fatal);
                    };
                    selected = object.get(&field).cloned();
                }
                '[' => {
                    index += 1;
                    let start = index;
                    let mut level = 0;
                    loop {
                        match chars.get(index) {
                            None => return Err(TimError::Fatal),
                            Some('[') => level += 1,
                            Some(']') if level == 0 => break,
                            Some(']') => level -= 1,
                            _ => {}
                        }
                        index += 1;
                    }
                    let inside = line.with_text(chars[start..index].iter().collect::<String>());
                    let key = self
                        .apply_functions_and_variables(memory, &inside)?
                        .ok_or(TimError::Fatal)?;
                    selected = match &selected {
                        Some(JsonValue::Array(values)) => {
                            let position: usize = key.parse().map_err(|_| TimError::Fatal)?;
                            Some(values.get(position).cloned().ok_or(TimError::Fatal)?)
                        }
                        Some(JsonValue::Object(object)) => object.get(&key).cloned(),
                        _ => return fail("Major parsing error", line),
                    };
                    if selected.is_none() {
                        return fail("Data parsing error", line);
                    }
                    index += 1;
                }
                _ => break,
            }
        }
        match selected {
            Some(JsonValue::String(string)) => result.push_str(&string),
            Some(other) => result.push_str(&other.to_string()),
            None => {}
        }
        Ok(index)
    }

    /// What a bare word means inside an expression: a variable, a JSON path into one, or nothing.
    pub fn variable_for_expression(
        &mut self,
        memory: &mut Memory,
        name: &str,
        location: &StringLocated,
    ) -> TimResult<Option<TValue>> {
        if !name.contains(['.', '[']) {
            return Ok(memory.get_variable(name));
        }
        let expanded = self
            .apply_functions_and_variables(
                memory,
                &StringLocated::new(name, location.location().clone()),
            )?
            .ok_or(TimError::Fatal)?;
        Ok(Some(match json::parse(&expanded) {
            Ok(json) => TValue::Json(json),
            Err(_) => TValue::string(expanded),
        }))
    }

    pub fn execute_affectation(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
    ) -> TimResult<()> {
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!")?;
        eater.skip_spaces();
        let mut name = eater.eat_and_get_varname()?;
        let mut scope = VariableScope::lazy_parse(&name);
        if scope.is_some() {
            eater.skip_spaces();
            if matches!(eater.peek_char(), '?' | '=') {
                scope = None;
            } else {
                name = eater.eat_and_get_varname()?;
            }
        }
        eater.skip_spaces();
        let conditional = eater.peek_char() == '?';
        if conditional {
            eater.check_and_eat_char('?')?;
        }
        eater.check_and_eat_char('=')?;
        if conditional && memory.get_variable(&name).is_some() {
            return Ok(());
        }
        eater.skip_spaces();
        let value = eater.eat_expression(self, memory)?;
        memory.put_variable(&name, value, scope, eater.line())
    }

    fn execute_affectation_define(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
    ) -> TimResult<()> {
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!define")?;
        eater.skip_spaces();
        let name = eater.eat_and_get_varname()?;
        eater.skip_spaces();
        let definition = line.with_text(eater.eat_all_to_end());
        let value = self
            .apply_functions_and_variables(memory, &definition)?
            .ok_or(TimError::Fatal)?;
        memory.put_variable(
            &name,
            TValue::string(value),
            Some(VariableScope::Global),
            eater.line(),
        )
    }

    pub fn execute_legacy_define(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
    ) -> TimResult<()> {
        if self.functions.pending().is_some() {
            return fail("already0048", line);
        }
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!define")?;
        eater.skip_spaces();
        let mut function =
            eater.eat_declare_function(self, memory, true, false, FunctionType::LegacyDefine)?;
        function.set_legacy_definition(eater.eat_all_to_end());
        self.functions.add(Rc::new(function));
        Ok(())
    }

    pub fn execute_legacy_definelong(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
    ) -> TimResult<()> {
        if self.functions.pending().is_some() {
            return fail("already0068", line);
        }
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!definelong")?;
        eater.skip_spaces();
        let function =
            eater.eat_declare_function(self, memory, true, true, FunctionType::LegacyDefinelong)?;
        self.functions.start_pending(function);
        Ok(())
    }

    /// `![unquoted|final ]function NAME(...)` or `!procedure`, possibly with a one-line `return` body.
    pub fn declare_function(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
        kind: FunctionType,
    ) -> TimResult<()> {
        if self.functions.pending().is_some() {
            return fail("already0068", line);
        }
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!")?;
        let mut unquoted = false;
        let mut is_final = false;
        loop {
            if eater.peek_char() == 'u' {
                eater.check_and_eat("unquoted")?;
                eater.skip_spaces();
                unquoted = true;
            } else if eater.peek_char() == 'f' && eater.peek_char_n2() == 'i' {
                eater.check_and_eat("final")?;
                eater.skip_spaces();
                is_final = true;
            } else {
                break;
            }
        }
        let keyword = if kind == FunctionType::Procedure {
            "procedure"
        } else {
            "function"
        };
        eater.check_and_eat(keyword)?;
        eater.skip_spaces();
        let mut function = eater.eat_declare_function(self, memory, unquoted, false, kind)?;
        if kind == FunctionType::ReturnFunction {
            let has_inline_return = match eater.peek_char() {
                'r' => {
                    eater.check_and_eat("return")?;
                    true
                }
                '!' => {
                    eater.check_and_eat("!return")?;
                    true
                }
                _ => false,
            };
            if has_inline_return {
                eater.skip_spaces();
                let body = format!("!return {}", eater.eat_all_to_end());
                function.add_body(StringLocated::new(body, line.location().clone()))?;
            }
        }
        self.functions.declare(function, is_final, line)
    }

    fn execute_assert(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let mut eater = Eater::new(line.clone());
        eater.skip_spaces();
        eater.check_and_eat("!assert")?;
        eater.skip_spaces();
        let value = eater.eat_expression_stop_at_colon(self, memory)?;
        eater.skip_spaces();
        if value.to_bool() {
            return Ok(());
        }
        if eater.peek_char() == ':' {
            eater.check_and_eat_char(':')?;
            let message = eater.eat_expression(self, memory)?;
            return fail(format!("Assertion error : {message}"), line);
        }
        fail("Assertion error", line)
    }

    fn execute_option(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        const OPTIONS: [(&str, Option<&str>); 6] = [
            ("LANGUAGE", None),
            ("USE_DESCRIPTIVE_NAMES", None),
            ("HANDWRITTEN", Some("true")),
            ("DEBUG", Some("true")),
            ("SVG_DESC", None),
            ("SVG_TITLE", None),
        ];
        let simplify = |s: &str| -> String {
            s.chars()
                .filter(char::is_ascii_alphabetic)
                .map(|c| c.to_ascii_lowercase())
                .collect()
        };
        let mut eater = Eater::new(line.clone());
        eater.skip_spaces();
        eater.check_and_eat("!option")?;
        eater.skip_spaces();
        let key = eater.eat_and_get_varname()?;
        eater.skip_spaces();
        let value = match eater.eat_expression(self, memory) {
            Ok(value) => Some(value),
            Err(TimError::Eater(_)) => None,
            Err(other) => return Err(other),
        };
        if let Some((name, default)) = OPTIONS
            .iter()
            .find(|(name, _)| simplify(name) == simplify(&key))
        {
            if let Some(value) = value
                .map(|value| value.to_string())
                .or(default.map(str::to_owned))
            {
                self.options.push(((*name).to_owned(), value));
            }
        }
        Ok(())
    }

    fn eat_directive_argument(
        &mut self,
        memory: &mut Memory,
        line: &StringLocated,
        keyword: &str,
    ) -> TimResult<String> {
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat(keyword)?;
        eater.skip_spaces();
        let argument = line.with_text(eater.eat_all_to_end());
        self.apply_functions_and_variables(memory, &argument)?
            .ok_or(TimError::Fatal)
    }

    fn execute_import(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let what = self.eat_directive_argument(memory, line, "!import")?;
        if self.host.file_exists(std::path::Path::new(&what)) {
            return Ok(());
        }
        fail("Cannot import", line)
    }

    fn execute_include(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!include")?;
        match eater.peek_char() {
            'u' => eater.check_and_eat("url")?,
            '_' => {
                eater.check_and_eat("_")?;
                if eater.peek_char() == 'm' {
                    eater.check_and_eat("many")?;
                } else {
                    eater.check_and_eat("once")?;
                }
            }
            _ => {}
        }
        eater.skip_spaces();
        let argument = line.with_text(eater.eat_all_to_end());
        let mut what = self
            .apply_functions_and_variables(memory, &argument)?
            .ok_or(TimError::Fatal)?;
        if let Some(bang) = what.rfind('!') {
            what.truncate(bang);
        }

        let included = if let Some(stdlib_path) = what
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let file = self.input_file(&what, line)?.ok_or(TimError::Fatal)?;
            let description = format!("<{stdlib_path}>");
            stdlib::puml_resource(stdlib_path).map(|content| {
                (
                    file.parent_folder(),
                    lines_of(&content, &description, &description, None),
                )
            })
        } else if what.starts_with("http://") || what.starts_with("https://") {
            return fail("Cannot open URL", line);
        } else if what.starts_with('[') && what.ends_with(']') {
            return fail("cannot include java.io.IOException: To be finished", line);
        } else if let Some(file) = self.input_file(&what, line)? {
            let Some(content) = file.read(self.host) else {
                return fail("Cannot include file", line);
            };
            // PlantUML labels lines read from a diagram block inside an included file "desc2".
            let lines = lines_of(&content, "desc2", &what, Some(line.location().clone()));
            Some((file.parent_folder(), lines))
        } else {
            None
        };

        let Some((folder, lines)) = included else {
            return fail(format!("cannot include {what}"), line);
        };
        let saved = self.enter_folder(folder);
        let outcome = self.execute_lines(memory, &skip_yaml_header(lines).0, None, false);
        self.paths = saved;
        outcome.map(|_| ())
    }

    fn input_file(&self, name: &str, line: &StringLocated) -> TimResult<Option<InputFile>> {
        self.paths
            .input_file(name, self.host)
            .map_err(|message| EaterException::new(message, line).into())
    }

    fn execute_includesub(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let what = self.eat_directive_argument(memory, line, "!includesub")?;
        let mut sub = None;
        let mut saved_paths = None;
        if let Some((file_name, block_name)) = what.split_once('!') {
            if let Some(file) = self.input_file(file_name, line)? {
                saved_paths = Some(self.enter_folder(file.parent_folder()));
                let Some(content) = file.read(self.host) else {
                    self.restore_paths(saved_paths);
                    return fail(format!("cannot include {what}"), line);
                };
                let lines = crate::preproc::read_uncommented_merged_lines(
                    &String::from_utf8_lossy(&content),
                    &what,
                    Some(line.location().clone()),
                );
                sub = match sub_from_lines(&lines, block_name) {
                    Ok(found) => found,
                    Err(error) => {
                        self.restore_paths(saved_paths);
                        return Err(error);
                    }
                };
            }
        }
        let sub = sub.or_else(|| self.subs.get(&what).cloned());
        let outcome = match sub {
            Some(lines) => self.execute_lines(memory, &lines, None, false).map(|_| ()),
            None => fail(format!("cannot include {what}"), line),
        };
        self.restore_paths(saved_paths);
        outcome
    }

    /// Makes `folder` the base for relative includes and returns the previous paths to restore later.
    fn enter_folder(&mut self, folder: Folder) -> PathSystem {
        let entered = self.paths.with_current_dir(folder);
        std::mem::replace(&mut self.paths, entered)
    }

    fn restore_paths(&mut self, saved: Option<PathSystem>) {
        if let Some(saved) = saved {
            self.paths = saved;
        }
    }

    fn execute_includedef(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let name = self.eat_directive_argument(memory, line, "!includedef")?;
        let body: Vec<StringLocated> = self
            .definitions
            .definition(&name)
            .into_iter()
            .map(|text| StringLocated::new(text, line.location().clone()))
            .collect();
        self.execute_lines(memory, &body, None, false).map(|_| ())
    }

    fn execute_theme(&mut self, memory: &mut Memory, line: &StringLocated) -> TimResult<()> {
        let mut eater = Eater::new(line.trimmed());
        eater.skip_spaces();
        eater.check_and_eat("!theme")?;
        eater.skip_spaces();
        let mut name = eater.eat_all_to_end();
        let mut from = None;
        if let Some(position) = name.to_lowercase().find(" from ") {
            let from_text = java::trim(&name[position + " from ".len()..]).to_owned();
            from = Some(
                self.apply_functions_and_variables(memory, &line.with_text(from_text))?
                    .ok_or(TimError::Fatal)?,
            );
            name = java::trim(&name[..position]).to_owned();
        }
        let real_name = self
            .apply_functions_and_variables(memory, &line.with_text(name.clone()))?
            .ok_or(TimError::Fatal)?;
        let Some(theme) = self.load_theme(&real_name, from.as_deref(), line)? else {
            let location = from.map(|from| format!(" in {from}")).unwrap_or_default();
            return fail(format!("Cannot load theme {real_name}{location}"), line);
        };
        let (body, metadata) = skip_yaml_header(theme);
        let outcome = self.execute_lines(memory, &body, None, false);
        self.theme_metadata = metadata;
        outcome.map(|_| ())
    }

    fn load_theme(
        &self,
        name: &str,
        from: Option<&str>,
        line: &StringLocated,
    ) -> TimResult<Option<Vec<StringLocated>>> {
        let file_name = format!("puml-theme-{name}.puml");
        let Some(from) = from else {
            let resource = format!("themes/{file_name}");
            if let Some(content) = crate::assets::get(&resource) {
                let description = format!("</{resource}>");
                return Ok(Some(lines_of(content, &description, &description, None)));
            }
            let local = self
                .input_file(&file_name, line)?
                .and_then(|file| file.read(self.host));
            let description = format!("theme {name}");
            return Ok(local.map(|content| lines_of(&content, &description, &description, None)));
        };
        if let Some(library) = from
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let content = stdlib::puml_resource(&format!("{library}/{file_name}"));
            let description = format!("{name} from {from}");
            return Ok(content.map(|content| lines_of(&content, &description, &description, None)));
        }
        if from.starts_with("http://") || from.starts_with("https://") {
            return fail("Cannot open URL", line);
        }
        let separator = if from.ends_with('/') { "" } else { "/" };
        let file = self
            .input_file(&format!("{from}{separator}{file_name}"), line)?
            .ok_or(TimError::Fatal)?;
        let content = file.read(self.host).ok_or(TimError::Fatal)?;
        let description = format!("{name} from {from}");
        Ok(Some(lines_of(&content, &description, &description, None)))
    }

    /// Takes the output lines a procedure wrote since `start` back out, joined into one value.
    pub fn extract_from_result(&mut self, start: usize) -> String {
        let extracted: Vec<String> = self
            .result
            .drain(start..)
            .map(|line| line.text().to_owned())
            .collect();
        extracted.join(&jaws::BLOCK_E1_NEWLINE.to_string())
    }

    /// `%xargs()`: what follows the first word of the diagram's first output line.
    ///
    /// PlantUML reads it from the line's debug form, `(SL) <text>`, so the first word is that prefix.
    pub fn xargs(&self) -> Option<String> {
        let first = self.result.first()?;
        let shown = if first.text().is_empty() {
            "<<<EMPTY STRING>>>".to_owned()
        } else {
            format!("(SL) {}", first.text())
        };
        let space = shown.find(' ')?;
        Some(java::trim(&shown[space + 1..]).to_owned())
    }
}

fn java_enum_name(debug_name: &str) -> String {
    let mut name = String::new();
    for (index, c) in debug_name.chars().enumerate() {
        if c.is_ascii_uppercase() && index > 0 {
            name.push('_');
        }
        name.push(c.to_ascii_uppercase());
    }
    name
}

fn variable_name_at(memory: &Memory, chars: &[char], position: usize) -> Option<String> {
    if is_just_after_a_letter(chars, position) && chars[position] != '$' {
        return None;
    }
    let name = memory.variable_name_at(chars, position);
    if name.is_empty() {
        return None;
    }
    let after = position + name.chars().count();
    (after == chars.len() || !is_letter_or_emoji_or_underscore_or_digit(chars[after]))
        .then_some(name)
}

fn is_just_after_a_letter(chars: &[char], position: usize) -> bool {
    let after_backslash_n =
        position > 1 && chars[position - 2] == '\\' && chars[position - 1] == 'n';
    position > 0
        && is_letter_or_emoji_or_underscore_or_digit(chars[position - 1])
        && !after_backslash_n
}

fn is_java_identifier_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Lines of an included resource: the first diagram block when it contains one, otherwise all of it.
fn lines_of(
    content: &[u8],
    extracted_description: &str,
    plain_description: &str,
    parent: Option<LineLocation>,
) -> Vec<StringLocated> {
    let text = String::from_utf8_lossy(content);
    read_lines_of_first_diagram(&text, extracted_description, plain_description, parent)
}

/// Drops a leading `---` YAML block and returns its `key: value` pairs.
fn skip_yaml_header(lines: Vec<StringLocated>) -> (Vec<StringLocated>, JsonObject) {
    let mut metadata = JsonObject::new();
    if lines.first().is_none_or(|line| line.text() != "---") {
        return (lines, metadata);
    }
    let mut rest = lines.into_iter().skip(1);
    for line in rest.by_ref() {
        if line.text() == "---" {
            break;
        }
        if let Some((key, value)) = line.text().split_once(':')
            && !key.is_empty()
        {
            metadata.add(key.trim(), JsonValue::String(value.trim().to_owned()));
        }
    }
    (rest.collect(), metadata)
}

/// The lines between `!startsub NAME` and `!endsub` in an included file.
fn sub_from_lines(lines: &[StringLocated], name: &str) -> TimResult<Option<Vec<StringLocated>>> {
    let mut sub: Option<Vec<StringLocated>> = None;
    let mut skipping = false;
    for line in lines {
        match line_type(java::trim(line.text())) {
            LineType::Startsub => {
                if parse_startsub(&line.trimmed())? == name {
                    skipping = false;
                    sub.get_or_insert_with(Vec::new);
                }
                continue;
            }
            LineType::Endsub if sub.is_some() => skipping = true,
            _ => {}
        }
        if let Some(sub) = &mut sub
            && !skipping
        {
            sub.push(line.clone());
        }
    }
    Ok(sub)
}

/// The arguments of a call in a plain line: positional values and `name = value` pairs.
struct FunctionCall {
    values: Vec<TValue>,
    named: HashMap<String, TValue>,
    /// Characters consumed from the start of the call, up to and including `)`.
    end_position: usize,
}

impl FunctionCall {
    fn parse(
        call: &StringLocated,
        is_legacy_define: bool,
        unquoted: bool,
        context: &mut TContext,
        memory: &mut Memory,
    ) -> TimResult<Self> {
        let mut eater = Eater::new(call.clone());
        let mut values = Vec::new();
        let mut named = HashMap::new();
        eater.skip_until_char('(');
        eater.check_and_eat_char('(')?;
        eater.skip_spaces();
        if eater.peek_char() == ')' {
            eater.check_and_eat_char(')')?;
            return Ok(Self {
                values,
                named,
                end_position: eater.position(),
            });
        }
        loop {
            eater.skip_spaces();
            if is_legacy_define || unquoted {
                let name = if unquoted && !is_legacy_define && eater.match_affectation() {
                    let name = eater.eat_and_get_varname()?;
                    eater.skip_spaces();
                    eater.check_and_eat_char('=')?;
                    eater.skip_spaces();
                    Some(name)
                } else {
                    None
                };
                let read = eater.eat_and_get_optional_quoted_string()?;
                let value = context
                    .apply_functions_and_variables(
                        memory,
                        &StringLocated::new(read, call.location().clone()),
                    )?
                    .ok_or(TimError::Fatal)?;
                match name {
                    Some(name) => {
                        named.insert(name, TValue::string(value));
                    }
                    None => values.push(TValue::string(value)),
                }
            } else {
                let name = if eater.match_affectation() {
                    let name = eater.eat_and_get_varname()?;
                    eater.skip_spaces();
                    eater.check_and_eat_char('=')?;
                    eater.skip_spaces();
                    Some(name)
                } else {
                    None
                };
                let mut tokens =
                    super::expression::TokenStack::eat_until_close_parenthesis_or_comma(
                        &mut eater,
                    )?;
                tokens.guess_functions(eater.line())?;
                let value = tokens.get_result(eater.line(), context, memory)?;
                match name {
                    Some(name) => {
                        named.insert(name, value);
                    }
                    None => values.push(value),
                }
            }
            eater.skip_spaces();
            match eater.eat_one_char()? {
                ',' => {}
                ')' => break,
                _ if unquoted => {
                    return fail(
                        "unquoted function/procedure cannot use expression.",
                        eater.line(),
                    );
                }
                _ => return fail("call001", eater.line()),
            }
        }
        Ok(Self {
            values,
            named,
            end_position: eater.position(),
        })
    }
}
