//! The preprocessor (PlantUML's "tim"): variables, functions, conditionals, loops, includes and themes.

mod builtins;
mod context;
mod date;
mod eater;
mod error;
mod eval_boolean;
mod expression;
mod function;
mod iterator;
mod line_type;
mod memory;
mod paths;
mod trie;
mod value;

pub(crate) use paths::Folder;

/// `Date.toString()` in the host's time zone, as `%filedate()` reports a file's modification time.
pub fn java_date_string(millis: i64, host: &dyn Host) -> String {
    let zone = host
        .local_time_zone()
        .and_then(|name| jiff::tz::TimeZone::get(&name).ok())
        .unwrap_or(jiff::tz::TimeZone::UTC);
    date::java_date_to_string(millis, &zone)
}

use context::TContext;
use error::TimError;
use memory::{Memory, VariableScope};
use value::TValue;

use crate::host::Host;
use crate::jaws;
use crate::text::StringLocated;

/// What the preprocessor knows about the diagram file and the command line.
#[derive(Default)]
pub struct PreprocessorEnvironment {
    pub filename: Option<String>,
    pub dirpath: Option<String>,
    pub filedate: Option<String>,
    /// `-DNAME=value` definitions, visible as global variables.
    pub defines: Vec<(String, String)>,
}

/// `@startdef(id=NAME)` blocks of the same source, for `!includedef NAME`.
pub(crate) trait Definitions {
    fn definition(&self, name: &str) -> Vec<String>;
}

pub(crate) struct Preprocessed {
    pub lines: Vec<StringLocated>,
    /// The last line then carries the error message.
    pub failed: bool,
}

pub(crate) fn preprocess_block(
    lines: &[StringLocated],
    host: &dyn Host,
    environment: &PreprocessorEnvironment,
    definitions: &dyn Definitions,
    current_folder: Folder,
) -> Preprocessed {
    let mut context = TContext::new(host, environment, definitions, current_folder);
    let mut memory = Memory::new_global();
    if let Some(first) = lines.first() {
        for (name, value) in &environment.defines {
            let _ = memory.put_variable(
                name,
                TValue::string(value.clone()),
                Some(VariableScope::Global),
                first,
            );
        }
    }
    let outcome = context.execute_lines(&mut memory, lines, None, false);
    let mut result = context.into_result();
    let failed = match outcome {
        Ok(_) => false,
        Err(TimError::Eater(error)) => {
            result.push(error.location.with_preprocessor_error(error.message));
            true
        }
        Err(TimError::Fatal | TimError::JsonParse(_)) => {
            if let Some(last) = lines.last() {
                result.push(last.with_preprocessor_error("Fatal parsing error"));
            }
            true
        }
    };
    Preprocessed {
        lines: jaws::expand_breaklines(result),
        failed,
    }
}
