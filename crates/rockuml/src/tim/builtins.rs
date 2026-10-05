//! The `%name()` functions every diagram can call.

// Every builtin shares one signature, including those that cannot fail.
#![allow(clippy::unnecessary_wraps)]

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use super::context::TContext;
use super::eater::Eater;
use super::error::{TimError, TimResult, fail};
use super::function::{FunctionSignature, FunctionType, FunctionsSet, TFunction};
use super::memory::{Memory, VariableScope};
use super::value::TValue;
use crate::color::HColor;
use crate::java;
use crate::jaws;
use crate::json::{self, JsonObject, JsonValue};
use crate::text::StringLocated;

type Body = fn(&mut Call) -> TimResult<TValue>;

/// Name, argument count of the signature, accepted argument counts, implementation.
type BuiltinEntry = (&'static str, usize, fn(usize) -> bool, Body);

/// What a builtin sees of its invocation.
struct Call<'c, 'a> {
    context: &'c mut TContext<'a>,
    memory: &'c mut Memory,
    location: &'c StringLocated,
    arguments: &'c [TValue],
    named: &'c HashMap<String, TValue>,
}

impl Call<'_, '_> {
    fn argument(&self, index: usize) -> TimResult<&TValue> {
        self.arguments.get(index).ok_or(TimError::Fatal)
    }

    fn text(&self, index: usize) -> TimResult<String> {
        Ok(self.argument(index)?.to_string())
    }

    fn int(&self, index: usize) -> TimResult<i32> {
        Ok(self.argument(index)?.to_int())
    }

    fn json(&self, index: usize) -> TimResult<&JsonValue> {
        match self.argument(index)? {
            TValue::Json(json) => Ok(json),
            _ => fail("Not JSON data", self.location),
        }
    }

    fn color(&self, index: usize) -> TimResult<HColor> {
        match HColor::parse(&self.text(index)?) {
            Ok(Some(color)) => Ok(color),
            Ok(None) => fail("No such color", self.location),
            Err(_) => Err(TimError::Fatal),
        }
    }
}

struct Builtin {
    signature: FunctionSignature,
    covers: fn(usize) -> bool,
    body: Body,
}

impl TFunction for Builtin {
    fn signature(&self) -> &FunctionSignature {
        &self.signature
    }

    fn can_cover(&self, argument_count: usize, _named: &HashSet<String>) -> bool {
        (self.covers)(argument_count)
    }

    fn function_type(&self) -> FunctionType {
        FunctionType::ReturnFunction
    }

    fn execute_return_function(
        &self,
        context: &mut TContext,
        memory: &mut Memory,
        location: &StringLocated,
        arguments: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<TValue> {
        (self.body)(&mut Call {
            context,
            memory,
            location,
            arguments,
            named,
        })
    }

    fn execute_procedure(
        &self,
        _: &mut TContext,
        _: &mut Memory,
        _: &StringLocated,
        _: &[TValue],
        _: &HashMap<String, TValue>,
    ) -> TimResult<()> {
        Err(TimError::Fatal)
    }
}

/// `%invoke_procedure(name, arguments...)`, the one builtin that is a procedure.
struct InvokeProcedure {
    signature: FunctionSignature,
}

impl TFunction for InvokeProcedure {
    fn signature(&self) -> &FunctionSignature {
        &self.signature
    }

    fn can_cover(&self, argument_count: usize, _named: &HashSet<String>) -> bool {
        argument_count > 0
    }

    fn function_type(&self) -> FunctionType {
        FunctionType::Procedure
    }

    fn execute_return_function(
        &self,
        _: &mut TContext,
        _: &mut Memory,
        _: &StringLocated,
        _: &[TValue],
        _: &HashMap<String, TValue>,
    ) -> TimResult<TValue> {
        Err(TimError::Fatal)
    }

    fn execute_procedure(
        &self,
        context: &mut TContext,
        memory: &mut Memory,
        location: &StringLocated,
        arguments: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TimResult<()> {
        let name = arguments.first().ok_or(TimError::Fatal)?.to_string();
        let rest = &arguments[1..];
        let Some(function) = context.function_smart(&FunctionSignature::new(&name, rest.len()))
        else {
            return fail(format!("Cannot find void function {name}"), location);
        };
        function.execute_procedure(context, memory, location, rest, named)
    }
}

/// Registers the builtins in PlantUML's order: lookups may iterate them, so the order is observable.
#[allow(clippy::too_many_lines, reason = "one table row per builtin")]
pub fn register(functions: &mut FunctionsSet) {
    let builtins: [BuiltinEntry; 74] = [
        ("%false", 0, |n| n == 0, |_| Ok(TValue::from_bool(false))),
        ("%true", 0, |n| n == 0, |_| Ok(TValue::from_bool(true))),
        (
            "%backslash",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_REAL_BACKSLASH)),
        ),
        ("%boolval", 1, |n| n == 1, boolval),
        (
            "%breakline",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_BREAKLINE)),
        ),
        ("%call_user_func", 1, |n| n > 0, call_user_func),
        ("%chr", 1, |n| n == 1, chr),
        (
            "%darken",
            2,
            |n| n == 2,
            |call| {
                Ok(TValue::string(
                    call.color(0)?.darken(call.int(1)?).as_string(),
                ))
            },
        ),
        ("%date", 3, |n| n <= 3, date),
        (
            "%dec2hex",
            1,
            |n| n == 1,
            |call| {
                Ok(TValue::string(format!(
                    "{:x}",
                    call.int(0)?.cast_unsigned()
                )))
            },
        ),
        (
            "%dirpath",
            0,
            |n| n == 0,
            |call| {
                Ok(TValue::string(
                    call.context.environment.dirpath.clone().unwrap_or_default(),
                ))
            },
        ),
        ("%dollar", 0, |n| n == 0, |_| Ok(TValue::string("$"))),
        ("%eval", 1, |n| n == 1, eval),
        ("%feature", 1, |n| n == 1, feature),
        (
            "%filedate",
            0,
            |n| n == 0,
            |call| {
                Ok(TValue::string(
                    call.context
                        .environment
                        .filedate
                        .clone()
                        .unwrap_or_default(),
                ))
            },
        ),
        (
            "%file_exists",
            1,
            |n| n == 1,
            |call| {
                Ok(TValue::from_bool(
                    call.context.host.file_exists(Path::new(&call.text(0)?)),
                ))
            },
        ),
        (
            "%filename",
            0,
            |n| n == 0,
            |call| {
                Ok(TValue::string(
                    call.context
                        .environment
                        .filename
                        .clone()
                        .unwrap_or_default(),
                ))
            },
        ),
        (
            "%filename_no_extension",
            0,
            |n| n == 0,
            filename_no_extension,
        ),
        (
            "%function_exists",
            1,
            |n| n == 1,
            |call| {
                Ok(TValue::from_bool(
                    call.context.functions.exists(&call.text(0)?),
                ))
            },
        ),
        ("%get_all_stdlib", 1, |n| n <= 1, get_all_stdlib),
        ("%get_all_theme", 0, |n| n == 0, get_all_theme),
        (
            "%get_current_theme",
            0,
            |n| n == 0,
            |call| {
                Ok(TValue::Json(JsonValue::Object(
                    call.context.theme_metadata().clone(),
                )))
            },
        ),
        ("%get_json_keys", 1, |n| n == 1, get_json_keys),
        ("%get_json_type", 1, |n| n == 1, get_json_type),
        ("%get_stdlib", 1, |n| n <= 2, get_stdlib),
        (
            "%get_variable_value",
            1,
            |n| n == 1,
            |call| {
                Ok(call
                    .memory
                    .get_variable(&call.text(0)?)
                    .unwrap_or_else(|| TValue::string("")))
            },
        ),
        (
            "%version",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(crate::PLANTUML_VERSION)),
        ),
        ("%getenv", 1, |n| n == 1, getenv),
        (
            "%hex2dec",
            1,
            |n| n == 1,
            |call| {
                Ok(TValue::Int(
                    i32::from_str_radix(&call.text(0)?, 16).unwrap_or(0),
                ))
            },
        ),
        ("%hsl_color", 3, |n| n == 3 || n == 4, hsl_color),
        ("%intval", 1, |n| n == 1, intval),
        ("%invoke_procedure", 1, |n| n > 0, |_| Err(TimError::Fatal)),
        (
            "%is_dark",
            1,
            |n| n == 1,
            |call| Ok(TValue::from_bool(call.color(0)?.is_dark())),
        ),
        (
            "%is_light",
            1,
            |n| n == 1,
            |call| Ok(TValue::from_bool(!call.color(0)?.is_dark())),
        ),
        ("%json_add", 3, |n| n == 2 || n == 3, json_add),
        ("%json_key_exists", 1, |n| n == 2, json_key_exists),
        ("%json_merge", 2, |n| n == 2, json_merge),
        ("%json_remove", 2, |n| n == 2, json_remove),
        ("%json_set", 3, |n| n == 2 || n == 3, json_set),
        (
            "%left_align",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_NEWLINE_LEFT_ALIGN)),
        ),
        (
            "%lighten",
            2,
            |n| n == 2,
            |call| {
                Ok(TValue::string(
                    call.color(0)?.lighten(call.int(1)?).as_string(),
                ))
            },
        ),
        ("%load_json", 3, |n| (1..=3).contains(&n), load_json),
        (
            "%and",
            2,
            |n| n >= 2,
            |call| {
                Ok(TValue::from_bool(
                    call.arguments.iter().all(TValue::to_bool),
                ))
            },
        ),
        (
            "%nand",
            2,
            |n| n >= 2,
            |call| {
                Ok(TValue::from_bool(
                    !call.arguments.iter().all(TValue::to_bool),
                ))
            },
        ),
        (
            "%nor",
            2,
            |n| n >= 2,
            |call| {
                Ok(TValue::from_bool(
                    !call.arguments.iter().any(TValue::to_bool),
                ))
            },
        ),
        (
            "%not",
            1,
            |n| n == 1,
            |call| Ok(TValue::from_bool(!call.argument(0)?.to_bool())),
        ),
        (
            "%nxor",
            2,
            |n| n >= 2,
            |call| Ok(TValue::from_bool(true_count(call) != 1)),
        ),
        (
            "%or",
            2,
            |n| n >= 2,
            |call| {
                Ok(TValue::from_bool(
                    call.arguments.iter().any(TValue::to_bool),
                ))
            },
        ),
        (
            "%xor",
            2,
            |n| n >= 2,
            |call| Ok(TValue::from_bool(true_count(call) == 1)),
        ),
        (
            "%lower",
            1,
            |n| n == 1,
            |call| Ok(TValue::string(call.text(0)?.to_lowercase())),
        ),
        ("%mod", 2, |n| n == 2, modulo),
        (
            "%newline",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_NEWLINE)),
        ),
        (
            "%n",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_NEWLINE)),
        ),
        (
            "%now",
            0,
            |n| n == 0,
            |call| {
                Ok(TValue::Int(
                    (call.context.host.current_time_millis() / 1000) as i32,
                ))
            },
        ),
        (
            "%ord",
            1,
            |n| n == 1,
            |call| {
                Ok(TValue::Int(
                    call.text(0)?.chars().next().map_or(0, |c| c as i32),
                ))
            },
        ),
        ("%percent", 0, |n| n == 0, |_| Ok(TValue::string("%"))),
        ("%random", 2, |n| n <= 2, random),
        ("%retrieve_procedure", 1, |n| n > 0, retrieve_procedure),
        (
            "%reverse_color",
            1,
            |n| n == 1,
            |call| Ok(TValue::string(call.color(0)?.reverse().as_string())),
        ),
        (
            "%reverse_hsluv_color",
            1,
            |n| n == 1,
            |call| Ok(TValue::string(call.color(0)?.reverse_hsluv().as_string())),
        ),
        (
            "%right_align",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_NEWLINE_RIGHT_ALIGN)),
        ),
        ("%set_variable_value", 2, |n| n == 2, set_variable_value),
        ("%size", 1, |n| n == 1, size),
        ("%splitstr", 3, |n| n == 2, splitstr),
        ("%splitstr_regex", 2, |n| n == 2, splitstr_regex),
        (
            "%str2json",
            1,
            |n| n == 1,
            |call| Ok(json::parse(&call.text(0)?).map_or_else(|_| TValue::string(""), TValue::Json)),
        ),
        (
            "%string",
            1,
            |n| n == 1,
            |call| Ok(TValue::string(call.text(0)?)),
        ),
        (
            "%strlen",
            1,
            |n| n == 1,
            |call| Ok(TValue::Int(utf16_length(&call.text(0)?))),
        ),
        ("%strpos", 2, |n| n == 2, strpos),
        ("%substr", 3, |n| n == 2 || n == 3, substr),
        (
            "%tab",
            0,
            |n| n == 0,
            |_| Ok(TValue::string(jaws::BLOCK_E1_REAL_TABULATION)),
        ),
        (
            "%upper",
            1,
            |n| n == 1,
            |call| Ok(TValue::string(call.text(0)?.to_uppercase())),
        ),
        (
            "%variable_exists",
            1,
            |n| n == 1,
            |call| {
                Ok(TValue::from_bool(
                    call.memory.get_variable(&call.text(0)?).is_some(),
                ))
            },
        ),
        (
            "%xargs",
            0,
            |n| n == 1,
            |call| Ok(TValue::string(call.context.xargs().unwrap_or_default())),
        ),
    ];
    for (name, argument_count, covers, body) in builtins {
        if name == "%invoke_procedure" {
            functions.add(Rc::new(InvokeProcedure {
                signature: FunctionSignature::new(name, argument_count),
            }));
            continue;
        }
        functions.add(Rc::new(Builtin {
            signature: FunctionSignature::new(name, argument_count),
            covers,
            body,
        }));
    }
}

fn utf16_length(text: &str) -> i32 {
    i32::try_from(text.encode_utf16().count()).unwrap_or(i32::MAX)
}

fn true_count(call: &Call) -> usize {
    call.arguments
        .iter()
        .filter(|value| value.to_bool())
        .count()
}

fn boolval(call: &mut Call) -> TimResult<TValue> {
    let text = call.text(0)?.to_lowercase();
    match text.as_str() {
        "true" | "1" => Ok(TValue::from_bool(true)),
        "false" | "0" => Ok(TValue::from_bool(false)),
        _ => fail(format!("Cannot convert {text} to boolean."), call.location),
    }
}

fn call_user_func(call: &mut Call) -> TimResult<TValue> {
    let name = call.text(0)?;
    let rest = &call.arguments[1..];
    let Some(function) = call
        .context
        .function_smart(&FunctionSignature::new(&name, rest.len()))
    else {
        return fail(format!("Cannot find void function {name}"), call.location);
    };
    function.execute_return_function(call.context, call.memory, call.location, rest, call.named)
}

/// `Character.toChars`; Java returns `"\0"` for an invalid code point.
fn chr(call: &mut Call) -> TimResult<TValue> {
    let code_point = u32::try_from(call.int(0)?).ok().and_then(char::from_u32);
    Ok(TValue::string(
        code_point.map_or_else(|| "\0".to_owned(), |c| c.to_string()),
    ))
}

fn date(call: &mut Call) -> TimResult<TValue> {
    super::date::format_date(call.context.host, call.arguments, call.location)
}

fn eval(call: &mut Call) -> TimResult<TValue> {
    let expression = call.location.with_text(call.text(0)?);
    let value = Eater::new(StringLocated::new(
        expression.text(),
        expression.location().clone(),
    ))
    .eat_expression(call.context, call.memory)?;
    Ok(TValue::Int(value.to_int()))
}

fn feature(call: &mut Call) -> TimResult<TValue> {
    let feature = call.text(0)?;
    let known = feature.eq_ignore_ascii_case("style") || feature.eq_ignore_ascii_case("theme");
    Ok(TValue::Int(i32::from(known)))
}

fn filename_no_extension(call: &mut Call) -> TimResult<TValue> {
    let name = call
        .context
        .environment
        .filename
        .clone()
        .unwrap_or_default();
    let without_extension = name.rfind('.').map_or(name.as_str(), |dot| &name[..dot]);
    Ok(TValue::string(without_extension))
}

fn get_all_stdlib(call: &mut Call) -> TimResult<TValue> {
    let names = crate::stdlib::library_names();
    if call.arguments.is_empty() {
        return Ok(TValue::Json(JsonValue::Array(
            names.into_iter().map(JsonValue::String).collect(),
        )));
    }
    let mut result = JsonObject::new();
    for name in names {
        let library = crate::stdlib::Stdlib::retrieve(&name);
        let mut entry = JsonObject::new();
        entry.add("name", JsonValue::String(name.clone()));
        entry.add(
            "version",
            optional_string(library.as_ref().and_then(|library| library.version())),
        );
        entry.add(
            "source",
            optional_string(library.as_ref().and_then(|library| library.source())),
        );
        result.add(name, JsonValue::Object(entry));
    }
    Ok(TValue::Json(JsonValue::Object(result)))
}

fn optional_string(value: Option<String>) -> JsonValue {
    value.map_or(JsonValue::Null, JsonValue::String)
}

fn get_all_theme(_: &mut Call) -> TimResult<TValue> {
    let mut names: Vec<String> = crate::assets::FILES
        .iter()
        .filter_map(|(path, _)| {
            path.strip_prefix("themes/puml-theme-")?
                .strip_suffix(".puml")
        })
        .map(str::to_owned)
        .collect();
    names.sort();
    Ok(TValue::Json(JsonValue::Array(
        names.into_iter().map(JsonValue::String).collect(),
    )))
}

fn get_json_keys(call: &mut Call) -> TimResult<TValue> {
    let keys = |object: &JsonObject| {
        object
            .names()
            .map(|name| JsonValue::String(name.to_owned()))
            .collect::<Vec<_>>()
    };
    match call.json(0)? {
        JsonValue::Object(object) => Ok(TValue::Json(JsonValue::Array(keys(object)))),
        JsonValue::Array(values) => Ok(TValue::Json(JsonValue::Array(
            values
                .iter()
                .filter_map(|value| match value {
                    JsonValue::Object(object) => Some(keys(object)),
                    _ => None,
                })
                .flatten()
                .collect(),
        ))),
        _ => fail("Bad JSON type", call.location),
    }
}

fn get_json_type(call: &mut Call) -> TimResult<TValue> {
    let kind = match call.argument(0)? {
        TValue::String(_) | TValue::Json(JsonValue::String(_)) => "string",
        TValue::Int(_) | TValue::Json(JsonValue::Number(_)) => "number",
        TValue::Json(JsonValue::Array(_)) => "array",
        TValue::Json(JsonValue::Object(_)) => "object",
        TValue::Json(JsonValue::Bool(_)) => "boolean",
        TValue::Json(JsonValue::Null) => "json",
    };
    Ok(TValue::string(kind))
}

fn get_stdlib(call: &mut Call) -> TimResult<TValue> {
    let lowercase_metadata = |library: &crate::stdlib::Stdlib| {
        let mut metadata = JsonObject::new();
        for (key, value) in library.metadata() {
            metadata.add(key.to_lowercase(), JsonValue::String(value.clone()));
        }
        metadata
    };
    let mut result = JsonObject::new();
    match call.arguments.len() {
        0 => {
            for name in crate::stdlib::library_names() {
                if let Some(library) = crate::stdlib::Stdlib::retrieve(&name) {
                    result.add(name, JsonValue::Object(lowercase_metadata(&library)));
                }
            }
        }
        1 => {
            let library = crate::stdlib::Stdlib::retrieve(&call.text(0)?).ok_or(TimError::Fatal)?;
            result = lowercase_metadata(&library);
        }
        _ => {
            let library = crate::stdlib::Stdlib::retrieve(&call.text(0)?).ok_or(TimError::Fatal)?;
            let key = call.text(1)?.to_lowercase();
            let value = library
                .metadata_value(&key)
                .or_else(|| library.metadata_value(&key.to_uppercase()));
            return Ok(TValue::string(value.unwrap_or_default()));
        }
    }
    Ok(TValue::Json(JsonValue::Object(result)))
}

/// Only `plantuml*` names (and two JVM properties) are readable unless the security profile is INSECURE.
fn getenv(call: &mut Call) -> TimResult<TValue> {
    let name = call.text(0)?;
    let lowercase = name.to_lowercase();
    let insecure = call
        .context
        .host
        .getenv("PLANTUML_SECURITY_PROFILE")
        .is_some_and(|profile| matches!(profile.to_uppercase().as_str(), "INSECURE" | "UNSECURE"));
    let value = match lowercase.as_str() {
        "path.separator" => Some(if cfg!(windows) { ";" } else { ":" }.to_owned()),
        "line.separator" => Some(if cfg!(windows) { "\r\n" } else { "\n" }.to_owned()),
        _ if lowercase.starts_with("plantuml.security") => None,
        _ if lowercase.starts_with("plantuml") || insecure => call.context.host.getenv(&name),
        _ => None,
    };
    Ok(TValue::string(value.unwrap_or_default()))
}

fn hsl_color(call: &mut Call) -> TimResult<TValue> {
    let alpha = if call.arguments.len() == 4 {
        (f64::from(call.int(3)?) / 100.0) as f32
    } else {
        1.0
    };
    #[allow(clippy::cast_precision_loss)]
    let color = crate::color::hsl_to_rgb(
        call.int(0)? as f32,
        call.int(1)? as f32,
        call.int(2)? as f32,
        alpha,
    );
    Ok(TValue::string(HColor::Simple(color).as_string()))
}

fn intval(call: &mut Call) -> TimResult<TValue> {
    let text = call.text(0)?;
    match text.parse() {
        Ok(value) => Ok(TValue::Int(value)),
        Err(_) => fail(format!("Cannot convert {text} to integer."), call.location),
    }
}

fn json_add(call: &mut Call) -> TimResult<TValue> {
    let mut json = call.json(0)?.clone();
    match &mut json {
        JsonValue::Array(values) => values.push(call.argument(1)?.to_json_value()),
        JsonValue::Object(object) => object.add(call.text(1)?, call.argument(2)?.to_json_value()),
        _ => return Ok(call.argument(0)?.clone()),
    }
    Ok(TValue::Json(json))
}

fn json_key_exists(call: &mut Call) -> TimResult<TValue> {
    let TValue::Json(JsonValue::Object(object)) = call.argument(0)? else {
        return Ok(TValue::from_bool(false));
    };
    let key = call.argument(1)?;
    let is_text = matches!(key, TValue::String(_) | TValue::Json(JsonValue::String(_)));
    Ok(TValue::from_bool(
        is_text && object.contains(&key.to_string()),
    ))
}

fn json_merge(call: &mut Call) -> TimResult<TValue> {
    let base = call.json(0)?;
    let other = call.json(1)?;
    let is_container =
        |json: &JsonValue| matches!(json, JsonValue::Array(_) | JsonValue::Object(_));
    let merged = match (base, other) {
        (JsonValue::Array(values), JsonValue::Array(more)) => {
            JsonValue::Array(values.iter().chain(more).cloned().collect())
        }
        (JsonValue::Object(object), JsonValue::Object(more)) => {
            let mut merged = object.clone();
            merged.merge(more);
            JsonValue::Object(merged)
        }
        _ if is_container(base) == is_container(other) => return Ok(call.argument(0)?.clone()),
        _ => return fail("Bad JSON type", call.location),
    };
    Ok(TValue::Json(merged))
}

fn json_remove(call: &mut Call) -> TimResult<TValue> {
    let mut json = call.json(0)?.clone();
    match &mut json {
        JsonValue::Array(values) => {
            if let TValue::Int(index) = call.argument(1)?
                && let Ok(index) = usize::try_from(*index)
                && index < values.len()
            {
                values.remove(index);
            }
        }
        JsonValue::Object(object) => object.remove(&call.text(1)?),
        _ => return Ok(call.argument(0)?.clone()),
    }
    Ok(TValue::Json(json))
}

fn json_set(call: &mut Call) -> TimResult<TValue> {
    let mut json = call.json(0)?.clone();
    if !matches!(json, JsonValue::Array(_) | JsonValue::Object(_)) {
        return Ok(call.argument(0)?.clone());
    }
    if call.arguments.len() == 2
        && let JsonValue::Object(object) = &mut json
    {
        if let JsonValue::Object(patch) = call.argument(1)?.to_json_value() {
            object.deep_merge(&patch);
        }
        return Ok(TValue::Json(json));
    }
    match &mut json {
        JsonValue::Array(values) => {
            if let TValue::Int(index) = call.argument(1)? {
                let value = call.argument(2)?.to_json_value();
                if let Ok(index) = usize::try_from(*index)
                    && index < values.len()
                {
                    values[index] = value;
                }
            }
        }
        JsonValue::Object(object) => object.set(&call.text(1)?, call.argument(2)?.to_json_value()),
        _ => unreachable!("checked above"),
    }
    Ok(TValue::Json(json))
}

fn load_json(call: &mut Call) -> TimResult<TValue> {
    let path = call.text(0)?;
    let data = if path.starts_with('<') || path.starts_with('>') {
        let inner = path
            .get(1..path.len().saturating_sub(1))
            .ok_or(TimError::Fatal)?;
        crate::stdlib::json_resource(inner).map_err(|_| TimError::Fatal)?
    } else if path.starts_with("http://") || path.starts_with("https://") {
        call.context
            .read_url(&path)
            .filter(|bytes| !bytes.is_empty())
    } else {
        call.context
            .host
            .read_file(Path::new(&path))
            .filter(|bytes| !bytes.is_empty())
    };
    let text = match data {
        Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        None if call.arguments.len() > 1 => call.text(1)?,
        None => "{}".to_owned(),
    };
    match json::parse(&text) {
        Ok(json) => Ok(TValue::Json(json)),
        Err(error) => fail(
            format!(
                "JSON parse issue in source {path} on location {}:{}",
                error.line, error.column
            ),
            call.location,
        ),
    }
}

fn modulo(call: &mut Call) -> TimResult<TValue> {
    let divisor = call.int(1)?;
    if divisor == 0 {
        return fail("Divide by zero", call.location);
    }
    Ok(TValue::Int(call.int(0)?.wrapping_rem(divisor)))
}

fn random(call: &mut Call) -> TimResult<TValue> {
    let arguments: Vec<i32> = call.arguments.iter().map(TValue::to_int).collect();
    let random = &mut call.context.random;
    let value = match arguments[..] {
        [] => random.next_int(2),
        [bound] => random.next_int(bound),
        [min, max, ..] => random
            .next_int(max.wrapping_sub(min))
            .map(|value| value.wrapping_add(min)),
    };
    value.map(TValue::Int).ok_or(TimError::Fatal)
}

fn retrieve_procedure(call: &mut Call) -> TimResult<TValue> {
    let name = call.text(0)?;
    let rest = &call.arguments[1..];
    let function = call
        .context
        .function_smart(&FunctionSignature::new(&name, rest.len()))
        .ok_or(TimError::Fatal)?;
    let start = call.context.result_mut().len();
    function.execute_procedure(
        call.context,
        call.memory,
        call.location,
        rest,
        &HashMap::new(),
    )?;
    Ok(TValue::string(call.context.extract_from_result(start)))
}

fn set_variable_value(call: &mut Call) -> TimResult<TValue> {
    let name = call.text(0)?;
    let value = call.argument(1)?.clone();
    call.memory
        .put_variable(&name, value, Some(VariableScope::Global), call.location)?;
    Ok(TValue::string(""))
}

fn size(call: &mut Call) -> TimResult<TValue> {
    let size = match call.argument(0)? {
        TValue::Int(_) => 0,
        TValue::String(text) => utf16_length(text),
        TValue::Json(json) => json
            .container_len()
            .map_or(0, |length| i32::try_from(length).unwrap_or(i32::MAX)),
    };
    Ok(TValue::Int(size))
}

/// `StringTokenizer`: every character of the separator delimits, and empty tokens are dropped.
fn splitstr(call: &mut Call) -> TimResult<TValue> {
    let text = call.text(0)?;
    let delimiters = call.text(1)?;
    let tokens = text
        .split(|c| delimiters.contains(c))
        .filter(|token| !token.is_empty())
        .map(|token| JsonValue::String(token.to_owned()));
    Ok(TValue::Json(JsonValue::Array(tokens.collect())))
}

fn splitstr_regex(call: &mut Call) -> TimResult<TValue> {
    let text = call.text(0)?;
    let separator = crate::pattern::try_java_regex(&call.text(1)?, false).ok_or(TimError::Fatal)?;
    let parts = java::regex_split(&separator, &text);
    Ok(TValue::Json(JsonValue::Array(
        parts.into_iter().map(JsonValue::String).collect(),
    )))
}

fn strpos(call: &mut Call) -> TimResult<TValue> {
    let full: Vec<u16> = call.text(0)?.encode_utf16().collect();
    let searched: Vec<u16> = call.text(1)?.encode_utf16().collect();
    let position = if searched.is_empty() {
        Some(0)
    } else {
        full.windows(searched.len())
            .position(|window| window == searched.as_slice())
    };
    Ok(TValue::Int(position.map_or(-1, |position| {
        i32::try_from(position).unwrap_or(i32::MAX)
    })))
}

/// Indexes count UTF-16 code units, as in Java.
fn substr(call: &mut Call) -> TimResult<TValue> {
    let full: Vec<u16> = call.text(0)?.encode_utf16().collect();
    let start = call.int(1)?;
    if usize::try_from(start).is_ok_and(|start| start >= full.len()) {
        return Ok(TValue::string(""));
    }
    let start = usize::try_from(start).map_err(|_| TimError::Fatal)?;
    let mut result = &full[start..];
    if call.arguments.len() == 3 {
        let length = call.int(2)?;
        if let Ok(length) = usize::try_from(length)
            && length < result.len()
        {
            result = &result[..length];
        } else if length < 0 {
            return Err(TimError::Fatal);
        }
    }
    Ok(TValue::string(String::from_utf16_lossy(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_are_registered() {
        let mut functions = FunctionsSet::default();
        register(&mut functions);
        assert_eq!(functions.len(), 74);
        assert!(functions.exists("%strlen"));
        assert!(functions.exists("%invoke_procedure"));
    }
}
