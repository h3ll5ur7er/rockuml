//! Diagrams the engine panics on, where PlantUML throws and draws a crash report instead: the panic is caught
//! and reported, and the run carries on with the other diagrams.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

use rockuml::diagram::NotYetPorted;

use crate::console::Console;
use crate::exit_status::ExitStatus;

/// Why a diagram has no image.
#[derive(Debug)]
pub(crate) enum Unrendered {
    NotPorted(NotYetPorted),
    /// The engine panicked with this message.
    Crashed(String),
}

impl Unrendered {
    /// Reports the image `output` names as missing. A crash counts as a diagram error, as PlantUML's crash
    /// reports do.
    pub(crate) fn report(&self, output: &str, status: &ExitStatus, console: &mut Console) {
        match self {
            Self::NotPorted(not_ported) => {
                console.error(&format!("rockuml: {output}: {not_ported}"));
                status.goes_not_ported();
            }
            Self::Crashed(message) => {
                console.error(&format!("rockuml: {output}: crashed: {message}"));
                status.goes_has_errors();
            }
        }
    }
}

/// `work`'s result, with both ways it can fail as one.
pub(crate) fn render<T>(work: impl FnOnce() -> Result<T, NotYetPorted>) -> Result<T, Unrendered> {
    catch(work)
        .map_err(Unrendered::Crashed)?
        .map_err(Unrendered::NotPorted)
}

/// The panics are reported as messages of their own, without the default hook's output.
pub(crate) fn silence_panic_hook() {
    panic::set_hook(Box::new(|_| {}));
}

/// `work`'s result, or the message it panicked with.
pub(crate) fn catch<T>(work: impl FnOnce() -> T) -> Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(work)).map_err(|payload| message(payload.as_ref()))
}

pub(crate) fn message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "no message".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_become_their_message() {
        assert_eq!(catch(|| 1), Ok(1));
        assert_eq!(
            catch(|| -> i32 { panic!("Infinite Loop?") }),
            Err("Infinite Loop?".to_owned())
        );
        let detail = 3;
        assert_eq!(
            catch(|| -> i32 { panic!("bad {detail}") }),
            Err("bad 3".to_owned())
        );
    }
}
