//! Commands PlantUML has and rockuml has not ported yet. They accept exactly the lines PlantUML's commands
//! accept, so that the diagram type a source is read as, and the errors it gets, come out as in PlantUML;
//! executing one leaves the diagram not ported.

/// A diagram told that it holds something rockuml cannot draw yet.
pub(crate) trait NotPortedCommands {
    /// `command` names the PlantUML command that accepted the lines.
    fn command_not_ported(&mut self, command: &'static str);
}
